//! IR objects that can be parsed from their text representation.

use std::collections::hash_map::Entry;

use crate::{
    basic_block::BasicBlock,
    builtin::{
        op_interfaces::{IsolatedFromAboveInterface, OneResultInterface},
        ops::ForwardRefOp,
    },
    context::{Context, Ptr},
    identifier::Identifier,
    input_err,
    irfmt::parsers::int_parser,
    location::{self, Located, Location},
    op::op_impls,
    operation::Operation,
    result::{self, Result},
    value::Value,
};
use combine::{
    Parser, Positioned, StreamOnce,
    easy::{self, Errors, ParseError},
    error::{StdParseResult2, Tracked},
    stream::{
        self, IteratorStream, buffered,
        position::{self, SourcePosition},
        state::Stream,
    },
};
use rustc_hash::FxHashMap;
use thiserror::Error;
use utf8_chars::BufReadCharsExt;

/// State during parsing of any [Parsable] object.
/// Every parser implemented using [Parsable] will be passed
/// a mutable reference (wrapped with [StateStream]) to this state.
pub struct State<'a> {
    pub ctx: &'a mut Context,
    pub(crate) name_tracker: NameTracker,
    pub src: location::Source,
}

impl<'a> State<'a> {
    /// Create a new empty [State].
    pub fn new(ctx: &'a mut Context, src: location::Source) -> State<'a> {
        State {
            ctx,
            name_tracker: NameTracker::default(),
            src,
        }
    }
}

/// A wrapper around any [char] [Iterator] object.
/// Buffering and positioning are automatically handled hereafter.
pub struct CharIterator<'a>(Box<dyn Iterator<Item = char> + 'a>);

impl Iterator for CharIterator<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// A [State]ful [Stream]. Every [Parsable::parser] gets this as input,
/// allowing for the parser to have access to a state.
pub type StateStream<'a> = Stream<
    buffered::Stream<
        easy::Stream<
            stream::position::Stream<stream::IteratorStream<CharIterator<'a>>, SourcePosition>,
        >,
    >,
    State<'a>,
>;

impl Located for StateStream<'_> {
    fn loc(&self) -> location::Location {
        location::Location::SrcPos {
            src: self.state.src,
            pos: self.position(),
        }
    }

    fn set_loc(&mut self, _loc: Location) {
        panic!("Cannot set location of parser");
    }
}

pub type ParseResult<'a, T> = StdParseResult2<T, <StateStream<'a> as StreamOnce>::Error>;

/// Any object that can be parsed from its [Printable](crate::printable::Printable) text.
///
/// Implement [parse](Parsable::parse) and call [parser](Parsable::parser)
/// to get a parser combinator that can be combined with any other parser
/// from the [combine] library.
///
/// The [parse](Parsable::parse) function may take arguments whose type is
/// specified and constrained by the associated type [Parsable::Arg].
/// Example:
/// ```
/// use combine::{
///     Parser, Stream, StreamOnce, easy, stream::position,
///     parser::char::digit, many1
/// };
/// use pliron::{context::Context, parsable::
///     { state_stream_from_iterator, StateStream, Parsable, State, ParseResult},
///     location::Source,
/// };
/// #[derive(PartialEq, Eq)]
/// struct Number { n: u64 }
/// impl Parsable for Number {
///     type Arg = ();
///     type Parsed = Number;
///     fn parse<'a>(
///         state_stream: &mut StateStream<'a>,
///         arg: Self::Arg,
///     ) -> ParseResult<'a, Self::Parsed> {
///         many1::<String, _, _>(digit())
///         .map(|digits| {
///             let _ : &mut Context = state_stream.state.ctx;
///             Number { n: digits.parse::<u64>().unwrap() }
///         })
///         .parse_stream(&mut state_stream.stream).into()
///     }
/// }
/// let mut ctx = Context::new();
/// let state_stream = state_stream_from_iterator("100".chars(), State::new(&mut ctx, Source::InMemory));
/// assert!(Number::parser(()).parse(state_stream).unwrap().0 == Number { n: 100 });
///
/// ```
pub trait Parsable {
    /// Type of the argument that must be passed to the parser.
    type Arg: Clone + 'static;
    /// The type of the parsed entity.
    type Parsed;

    /// Define a parser using existing combinators and call
    /// `into` on [Parser::parse_stream] to get the final [ParseResult].
    /// Use [state_stream.state](StateStream::state) as necessary.
    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        arg: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed>;

    /// Get a parser combinator that can work on [StateStream] as its input.
    fn parser<'a>(
        arg: Self::Arg,
    ) -> Box<dyn Parser<StateStream<'a>, Output = Self::Parsed, PartialState = ()> + 'a> {
        combine::parser(move |parsable_state: &mut StateStream<'a>| {
            Self::parse(parsable_state, arg.clone())
        })
        .boxed()
    }

    /// Same as [Self::parser] but takes a unit reference for use as [ParserFn]
    fn parser_fn<'a>(
        _: &'a (),
        arg: Self::Arg,
    ) -> Box<dyn Parser<StateStream<'a>, Output = Self::Parsed, PartialState = ()> + 'a> {
        Self::parser(arg)
    }
}

/// Build a [StateStream] from an iterator, for use with [Parsable].
pub fn state_stream_from_iterator<'a, T: Iterator<Item = char> + 'a>(
    input: T,
    state: State<'a>,
) -> StateStream<'a> {
    StateStream {
        stream: buffered::Stream::new(
            easy::Stream::from(position::Stream::with_positioner(
                IteratorStream::new(CharIterator(Box::new(input))),
                SourcePosition::default(),
            )),
            100,
        ),
        state,
    }
}

/// Build a [StateStream] from a file, for use with [Parsable].
pub fn state_stream_from_file<'a>(
    file_reader: &'a mut std::io::BufReader<std::fs::File>,
    state: State<'a>,
) -> StateStream<'a> {
    state_stream_from_iterator(
        file_reader.chars().map(|c| {
            c.map_err(|e| eprintln!("Error reading chars from file: {}", e))
                .unwrap()
        }),
        state,
    )
}

/// A storable parser function. This allows storing a function pointer
/// to a parser in a table, allowing for invoking it indirectly.
// (if we can get rid of the dummy parameter, we wouldn't need [Parsable::parser_fn]).
pub type ParserFn<Arg, Parsed> =
    for<'a> fn(
        _: &'a (),
        arg: Arg,
    ) -> Box<dyn Parser<StateStream<'a>, Output = Parsed, PartialState = ()> + 'a>;

/// Convert [Result] into [StdParseResult2].
/// Enables using `?` on [Result] during parsing.
pub trait IntoParseResult<'a, T> {
    fn into_parse_result(self) -> StdParseResult2<T, <StateStream<'a> as StreamOnce>::Error>;
}

impl<'a, T> IntoParseResult<'a, T> for Result<T> {
    fn into_parse_result(self) -> StdParseResult2<T, <StateStream<'a> as StreamOnce>::Error> {
        match self {
            Ok(t) => combine::ParseResult::CommitOk(t),
            Err(e) => combine::ParseResult::CommitErr(e.into()),
        }
        .into()
    }
}

impl From<result::Error> for ParseError<StateStream<'_>> {
    fn from(value: result::Error) -> Self {
        let position = if let Location::SrcPos { pos, .. } = value.loc {
            pos
        } else {
            SourcePosition::default()
        };
        easy::Errors::from_errors(position, vec![easy::Error::Other(value.err)])
    }
}

impl From<result::Error> for combine::error::Commit<Tracked<Errors<char, char, SourcePosition>>> {
    fn from(value: result::Error) -> Self {
        let res: StdParseResult2<(), ParseError<StateStream<'_>>> =
            combine::ParseResult::CommitErr(value.into()).into();
        res.err().unwrap()
    }
}

/// Serves a similar purpose to what [ForwardRefOp] serves for SSA names.
enum LabelRef {
    ForwardRef(Ptr<BasicBlock>),
    Defined(Ptr<BasicBlock>),
}

impl LabelRef {
    fn label(&self) -> Ptr<BasicBlock> {
        match self {
            LabelRef::ForwardRef(label) => *label,
            LabelRef::Defined(label) => *label,
        }
    }
}

/// Utility for parsing SSA names and block labels.
#[derive(Default)]
pub(crate) struct NameTracker {
    ssa_name_scope: Vec<FxHashMap<Identifier, Value>>,
    block_label_scope: Vec<FxHashMap<Identifier, LabelRef>>,
}

#[derive(Error, Debug)]
#[error("Identifier {0} was not resolved to any definition in the scope")]
pub struct UnresolvedReference(Identifier);

#[derive(Error, Debug)]
pub enum ParserNameTrackerError {
    #[error("Identifier {0} defined more than once in the scope")]
    MultipleDefinitions(Identifier),
    #[error("Regions in a top-level operation must be IsolatedFromAbove")]
    TopLevelOpRegionNotIsolatedFromAbove,
}

impl NameTracker {
    /// An SSA use is seen. Get its [definition value][Value]
    /// or return a [forward reference][ForwardRefOp] that will
    /// be updated when the actual definition is seen.
    pub(crate) fn ssa_use(&mut self, ctx: &mut Context, id: &Identifier) -> Value {
        let scope = self
            .ssa_name_scope
            .last_mut()
            .expect("NameTracker doesn't have an active scope.");
        match scope.entry(id.clone()) {
            Entry::Occupied(occ) => *occ.get(),
            Entry::Vacant(vac) => {
                // Insert a forward reference.
                let forward_def = ForwardRefOp::new(ctx).result(ctx);
                vac.insert(forward_def);
                forward_def
            }
        }
    }

    /// Register an SSA definition. If `id` is already associated with a
    /// [forward reference][ForwardRefOp], update and replace all uses with `def`.
    pub(crate) fn ssa_def(
        &mut self,
        ctx: &mut Context,
        id: &(Identifier, Location),
        def: Value,
    ) -> Result<()> {
        let scope = self
            .ssa_name_scope
            .last_mut()
            .expect("NameTracker doesn't have an active scope.");

        match scope.entry(id.0.clone()) {
            Entry::Occupied(mut occ) => match occ.get_mut() {
                Value::OpResult { op, res_idx: _ } => {
                    let fref_opt = Operation::op(*op, ctx)
                        .downcast_ref::<ForwardRefOp>()
                        .map(|op| op.result(ctx));
                    if let Some(fref) = fref_opt {
                        // If there's already a def and its a forward ref, replace that.
                        fref.replace_some_uses_with(ctx, |_, _| true, &def);
                        Operation::erase(*op, ctx);
                        occ.insert(def);
                    } else {
                        // There's another def and it isn't a forward ref.
                        input_err!(
                            id.1.clone(),
                            ParserNameTrackerError::MultipleDefinitions(id.0.clone())
                        )?
                    }
                }
                Value::BlockArgument { .. } => {
                    // There's another def and it isn't a forward ref.
                    input_err!(
                        id.1.clone(),
                        ParserNameTrackerError::MultipleDefinitions(id.0.clone())
                    )?
                }
            },
            Entry::Vacant(vac) => {
                vac.insert(def);
            }
        }
        Ok(())
    }

    /// A [BasicBlock] use is seen. Get the actual block it refers to.
    /// If the block wasn't seen yet, create one now. When exiting the scope,
    /// if the block ends up not being seen, an undefined reference error is returned.
    pub(crate) fn block_use(&mut self, ctx: &mut Context, id: &Identifier) -> Ptr<BasicBlock> {
        let scope = self
            .block_label_scope
            .last_mut()
            .expect("NameTracker doesn't have an active scope.");
        match scope.entry(id.clone()) {
            Entry::Occupied(occ) => occ.get().label(),
            Entry::Vacant(vac) => {
                // Insert a forward reference.
                let block_forward = BasicBlock::new(ctx, Some(id.clone()), vec![]);
                vac.insert(LabelRef::ForwardRef(block_forward));
                block_forward
            }
        }
    }

    /// A [BasicBlock] def is seen. If refs was seen earlier,
    /// they are all updated now to refer to the provided block instead.
    pub(crate) fn block_def(
        &mut self,
        ctx: &mut Context,
        id: &(Identifier, Location),
        block: Ptr<BasicBlock>,
    ) -> Result<()> {
        let scope = self
            .block_label_scope
            .last_mut()
            .expect("NameTracker doesn't have an active scope.");
        match scope.entry(id.0.clone()) {
            Entry::Occupied(mut occ) => match occ.get_mut() {
                LabelRef::ForwardRef(fref) => {
                    fref.retarget_some_preds_to(ctx, |_, _| true, block);
                    BasicBlock::erase(*fref, ctx);
                    occ.insert(LabelRef::Defined(block));
                }
                LabelRef::Defined(_) => input_err!(
                    id.1.clone(),
                    ParserNameTrackerError::MultipleDefinitions(id.0.clone())
                )?,
            },
            Entry::Vacant(vac) => {
                vac.insert(LabelRef::Defined(block));
            }
        }
        Ok(())
    }

    /// Enter a new region.
    /// - If the parent op is [IsolatedFromAboveInterface],
    ///   then a new independent SSA name scope is created.
    /// - A new independent block label scope is always created.
    pub(crate) fn enter_region(&mut self, ctx: &Context, parent_op: Ptr<Operation>) -> Result<()> {
        if op_impls::<dyn IsolatedFromAboveInterface>(&*Operation::op(parent_op, ctx)) {
            self.ssa_name_scope.push(FxHashMap::default());
        } else if self.ssa_name_scope.is_empty() {
            input_err!(
                parent_op.deref(ctx).loc(),
                ParserNameTrackerError::TopLevelOpRegionNotIsolatedFromAbove
            )?
        }
        self.block_label_scope.push(FxHashMap::default());
        Ok(())
    }

    /// Exit a region.
    /// - If the parent op is [IsolatedFromAboveInterface], then the top SSA name scope is popped.
    /// - The top block label scope is popped.
    pub(crate) fn exit_region(
        &mut self,
        ctx: &Context,
        parent_op: Ptr<Operation>,
        loc: Location,
    ) -> Result<()> {
        if op_impls::<dyn IsolatedFromAboveInterface>(&*Operation::op(parent_op, ctx)) {
            // Check if there are any [ForwardRefOp].
            let ssa_scope = self
                .ssa_name_scope
                .pop()
                .expect("Exiting an isolated-from-above region which wasn't entered into.");
            for (id, op) in ssa_scope {
                if matches!(op, Value::OpResult { op, .. } if Operation::op(op, ctx).is::<ForwardRefOp>())
                {
                    input_err!(loc.clone(), UnresolvedReference(id.clone()))?
                }
            }
        }

        let label_scope = self
            .block_label_scope
            .pop()
            .expect("Exiting an isolated-from-above region which wasn't entered into.");

        // Check if there are any unresolved forward label references.
        for (id, op) in label_scope {
            if matches!(op, LabelRef::ForwardRef(_)) {
                input_err!(loc.clone(), UnresolvedReference(id.clone()))?
            }
        }

        Ok(())
    }
}

impl Parsable for usize {
    type Arg = ();
    type Parsed = usize;

    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        _arg: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        int_parser::<usize>().parse_stream(state_stream).into()
    }
}
impl Parsable for u64 {
    type Arg = ();
    type Parsed = u64;

    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        _arg: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        int_parser::<u64>().parse_stream(state_stream).into()
    }
}

impl Parsable for u32 {
    type Arg = ();
    type Parsed = u32;

    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        _arg: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        int_parser::<u32>().parse_stream(state_stream).into()
    }
}
