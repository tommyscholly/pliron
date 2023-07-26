//! [Dialect]s are a mechanism to group related [Op](crate::op::Op)s, [Type](crate::type::Type)s
//! and [Attribute](crate::attribute::Attribute)s.
use std::ops::Deref;

use combine::{easy, ParseResult, Parser};

use crate::{
    attribute::AttrId,
    context::Context,
    op::OpId,
    parsable::{self, Parsable, ParsableStream, StateStream},
    printable::{self, Printable},
    r#type::TypeId,
};

/// Dialect name: Safe wrapper around a String.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DialectName(String);

impl DialectName {
    /// Create a new DialectName
    pub fn new(name: &str) -> DialectName {
        DialectName(name.to_string())
    }
}

impl Printable for DialectName {
    fn fmt(
        &self,
        _ctx: &Context,
        _state: &printable::State,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Parsable for DialectName {
    type Parsed = DialectName;

    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
    ) -> ParseResult<Self::Parsed, easy::ParseError<ParsableStream<'a>>>
    where
        Self: Sized,
    {
        let id = parsable::parse_id();
        let mut parser = id.and_then(|dialect_name| {
            let dialect_name = DialectName::new(&dialect_name);
            if state_stream.state.ctx.dialects.contains_key(&dialect_name) {
                Ok(dialect_name)
            } else {
                Err(easy::Error::Message(
                    format!("Unregistered dialect {}", *dialect_name).into(),
                ))
            }
        });
        parser.parse_stream(&mut state_stream.stream)
    }
}

impl Deref for DialectName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A collection of Types and Ops.
/// Dialects are identified by their names.
pub struct Dialect {
    /// Name of this dialect.
    name: DialectName,
    /// Ops that are part of this dialect.
    ops: Vec<OpId>,
    /// Types that are part of this dialect.
    types: Vec<TypeId>,
    /// Attributes that are part of this dialect.
    attributes: Vec<AttrId>,
}

impl Printable for Dialect {
    fn fmt(
        &self,
        ctx: &Context,
        _state: &printable::State,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(f, "{}", self.name.disp(ctx))
    }
}

impl Dialect {
    /// Create a new unregistered dialect.
    pub fn new(name: DialectName) -> Dialect {
        Dialect {
            name,
            ops: vec![],
            types: vec![],
            attributes: vec![],
        }
    }

    /// Register this dialect if not already registered.
    pub fn register(self, ctx: &mut Context) {
        ctx.dialects.entry(self.name.clone()).or_insert(self);
    }

    /// Add an [Op](crate::op::Op) to this dialect.
    pub fn add_op(&mut self, op: OpId) {
        assert!(op.dialect == self.name);
        self.ops.push(op);
    }

    /// Add a [Type](crate::type::Type) to this dialect.
    pub fn add_type(&mut self, ty: TypeId) {
        assert!(ty.dialect == self.name);
        self.types.push(ty);
    }

    /// Add an [Attribute](crate::attribute::Attribute) to this dialect.
    pub fn add_attr(&mut self, attr: AttrId) {
        assert!(attr.dialect == self.name);
        self.attributes.push(attr);
    }

    /// This Dialect's name.
    pub fn get_name(&self) -> &DialectName {
        &self.name
    }

    /// Get reference to a registered Dialect by name.
    pub fn get_ref(ctx: &Context, name: DialectName) -> Option<&Dialect> {
        ctx.dialects.get(&name)
    }

    /// Get mutable reference to a registered Dialect by name.
    pub fn get_mut(ctx: &mut Context, name: DialectName) -> Option<&mut Dialect> {
        ctx.dialects.get_mut(&name)
    }
}

#[cfg(test)]
mod test {
    use combine::{easy, stream::position};
    use expect_test::expect;

    use crate::{
        context::Context,
        dialects,
        parsable::{self, Parsable, StateStream},
        printable::Printable,
    };

    use super::DialectName;

    #[test]
    fn parse_dialect() {
        let mut ctx = Context::new();
        dialects::builtin::register(&mut ctx);

        let input_stream = easy::Stream::from(position::Stream::new("non_existant"));
        let state_stream = StateStream {
            stream: input_stream,
            state: parsable::State { ctx: &mut ctx },
        };

        let res = DialectName::parser().parse(state_stream);
        let err_msg = format!("{}", res.err().unwrap());

        let expected_err_msg = expect![[r#"
            Parse error at line: 1, column: 1
            Unregistered dialect non_existant
        "#]];
        expected_err_msg.assert_eq(&err_msg);

        let input_stream = easy::Stream::from(position::Stream::new("builtin"));
        let state_stream = StateStream {
            stream: input_stream,
            state: parsable::State { ctx: &mut ctx },
        };

        let parsed = DialectName::parser().parse(state_stream).unwrap().0;
        assert_eq!(parsed.disp(&ctx).to_string(), "builtin");
    }
}
