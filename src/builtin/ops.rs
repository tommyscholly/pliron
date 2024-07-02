use combine::{token, Parser};
use pliron_derive::def_op;
use thiserror::Error;

use crate::{
    basic_block::BasicBlock,
    builtin::op_interfaces::ZeroResultInterface,
    common_traits::{Named, Verify},
    context::{Context, Ptr},
    identifier::Identifier,
    impl_op_interface, impl_verify_succ, input_err,
    irfmt::{
        parsers::{spaced, type_parser},
        printers::op::{region, symb_op_header, typed_symb_op_header},
    },
    linked_list::ContainsLinkedList,
    location::{Located, Location},
    op::{Op, OpObj},
    operation::Operation,
    parsable::{Parsable, ParseResult, StateStream},
    printable::{self, Printable},
    r#type::{TypeObj, TypePtr, Typed},
    region::Region,
    result::Result,
    verify_err,
};

use super::{
    attr_interfaces::TypedAttrInterface,
    attributes::TypeAttr,
    op_interfaces::{
        self, IsolatedFromAboveInterface, OneRegionInterface, OneResultInterface,
        SingleBlockRegionInterface, SymbolOpInterface, SymbolTableInterface, ZeroOpdInterface,
    },
    types::{FunctionType, UnitType},
};

/// Represents a module, a top level container operation.
///
/// See MLIR's [builtin.module](https://mlir.llvm.org/docs/Dialects/Builtin/#builtinmodule-mlirmoduleop).
/// It contains a single [SSACFG](super::op_interfaces::RegionKind::SSACFG)
/// region containing a single block which can contain any operations and
/// does not have a terminator.
///
/// Attributes:
///
/// | key | value | via Interface |
/// |-----|-------|-----|
/// | [ATTR_KEY_SYM_NAME](super::op_interfaces::ATTR_KEY_SYM_NAME) | [StringAttr](super::attributes::StringAttr) | [SymbolOpInterface] |
#[def_op("builtin.module")]
pub struct ModuleOp {}

impl Printable for ModuleOp {
    fn fmt(
        &self,
        ctx: &Context,
        state: &printable::State,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        symb_op_header(self).fmt(ctx, state, f)?;
        write!(f, " ")?;
        region(self).fmt(ctx, state, f)?;
        Ok(())
    }
}

impl Parsable for ModuleOp {
    type Arg = Vec<(Identifier, Location)>;
    type Parsed = OpObj;
    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        results: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        if !results.is_empty() {
            input_err!(
                state_stream.loc(),
                op_interfaces::ZeroResultVerifyErr(Self::get_opid_static().to_string())
            )?
        }
        let op = Operation::new(
            state_stream.state.ctx,
            Self::get_opid_static(),
            vec![],
            vec![],
            vec![],
            0,
        );
        let mut parser =
            spaced(token('@').with(Identifier::parser(()))).and(spaced(Region::parser(op)));
        parser
            .parse_stream(state_stream)
            .map(|(name, _region)| -> OpObj {
                let op = Box::new(ModuleOp { op });
                op.set_symbol_name(state_stream.state.ctx, &name);
                op
            })
            .into()
    }
}

impl_verify_succ!(ModuleOp);

impl ModuleOp {
    /// Create a new [ModuleOp].
    /// The underlying [Operation] is not linked to a [BasicBlock].
    /// The returned module has a single [crate::region::Region] with a single (BasicBlock)[crate::basic_block::BasicBlock].
    pub fn new(ctx: &mut Context, name: &str) -> ModuleOp {
        let op = Operation::new(ctx, Self::get_opid_static(), vec![], vec![], vec![], 1);
        let opop = ModuleOp { op };
        opop.set_symbol_name(ctx, name);

        // Create an empty block.
        let region = op.deref_mut(ctx).get_region(0).unwrap();
        let block = BasicBlock::new(ctx, None, vec![]);
        block.insert_at_front(region, ctx);

        opop
    }
}

impl_op_interface!(OneRegionInterface for ModuleOp {});
impl_op_interface!(SingleBlockRegionInterface for ModuleOp {});
impl_op_interface!(SymbolTableInterface for ModuleOp {});
impl_op_interface!(SymbolOpInterface for ModuleOp {});
impl_op_interface!(IsolatedFromAboveInterface for ModuleOp {});
impl_op_interface!(ZeroOpdInterface for ModuleOp {});
impl_op_interface!(ZeroResultInterface for ModuleOp {});

/// An operation with a name containing a single SSA control-flow-graph region.
/// See MLIR's [func.func](https://mlir.llvm.org/docs/Dialects/Func/#funcfunc-mlirfuncfuncop).
///
/// Attributes:
///
/// | key | value | via Interface |
/// |-----|-------|-----|
/// | [ATTR_KEY_SYM_NAME](super::op_interfaces::ATTR_KEY_SYM_NAME) | [StringAttr](super::attributes::StringAttr) | [SymbolOpInterface] |
/// | [ATTR_KEY_FUNC_TYPE](func_op::ATTR_KEY_FUNC_TYPE) | [TypeAttr](super::attributes::TypeAttr) | N/A |
#[def_op("builtin.func")]
pub struct FuncOp {}

pub mod func_op {
    use super::*;
    /// Attribute key for the function type.
    pub static ATTR_KEY_FUNC_TYPE: crate::Lazy<Identifier> =
        crate::Lazy::new(|| "builtin_func_type".try_into().unwrap());
}

impl FuncOp {
    /// Create a new [FuncOp].
    /// The returned function has a single region with an empty `entry` block.
    pub fn new(ctx: &mut Context, name: &str, ty: TypePtr<FunctionType>) -> Self {
        let ty_attr = TypeAttr::new(ty.into());
        let op = Operation::new(ctx, Self::get_opid_static(), vec![], vec![], vec![], 1);

        // Create an empty entry block.
        let arg_types = ty.deref(ctx).get_inputs().clone();
        let region = op.deref_mut(ctx).get_region(0).unwrap();
        let body = BasicBlock::new(ctx, Some("entry".try_into().unwrap()), arg_types);
        body.insert_at_front(region, ctx);
        {
            let opref = &mut *op.deref_mut(ctx);
            // Set function type attributes.
            opref
                .attributes
                .set(func_op::ATTR_KEY_FUNC_TYPE.clone(), ty_attr);
        }
        let opop = FuncOp { op };
        opop.set_symbol_name(ctx, name);

        opop
    }

    /// Get the function signature (type).
    pub fn get_type(&self, ctx: &Context) -> Ptr<TypeObj> {
        let opref = self.get_operation().deref(ctx);
        opref
            .attributes
            .get_as::<dyn TypedAttrInterface>(&func_op::ATTR_KEY_FUNC_TYPE)
            .unwrap()
            .get_type()
    }

    /// Get the entry block of this function.
    pub fn get_entry_block(&self, ctx: &Context) -> Ptr<BasicBlock> {
        self.get_region(ctx).deref(ctx).get_head().unwrap()
    }

    /// Get an iterator over all operations.
    pub fn op_iter<'a>(&self, ctx: &'a Context) -> impl Iterator<Item = Ptr<Operation>> + 'a {
        self.get_region(ctx)
            .deref(ctx)
            .iter(ctx)
            .flat_map(|bb| bb.deref(ctx).iter(ctx))
    }
}

impl Typed for FuncOp {
    fn get_type(&self, ctx: &Context) -> Ptr<TypeObj> {
        self.get_type(ctx)
    }
}

impl_op_interface!(OneRegionInterface for FuncOp {});
impl_op_interface!(SymbolOpInterface for FuncOp {});
impl_op_interface!(IsolatedFromAboveInterface for FuncOp {});

impl Printable for FuncOp {
    fn fmt(
        &self,
        ctx: &Context,
        state: &printable::State,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        typed_symb_op_header(self).fmt(ctx, state, f)?;
        write!(f, " ")?;
        region(self).fmt(ctx, state, f)?;
        Ok(())
    }
}

impl Parsable for FuncOp {
    type Arg = Vec<(Identifier, Location)>;
    type Parsed = OpObj;
    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        results: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        if !results.is_empty() {
            input_err!(
                state_stream.loc(),
                op_interfaces::ZeroResultVerifyErr(Self::get_opid_static().to_string())
            )?
        }

        let op = Operation::new(
            state_stream.state.ctx,
            Self::get_opid_static(),
            vec![],
            vec![],
            vec![],
            0,
        );

        let mut parser = (
            spaced(token('@').with(Identifier::parser(()))).skip(spaced(token(':'))),
            spaced(type_parser()),
            spaced(Region::parser(op)),
        );

        // Parse and build the function, providing name and type details.
        parser
            .parse_stream(state_stream)
            .map(|(fname, fty, _region)| -> OpObj {
                let ctx = &mut state_stream.state.ctx;
                {
                    let ty_attr = TypeAttr::new(fty);
                    let opref = &mut *op.deref_mut(ctx);
                    // Set function type attributes.
                    opref
                        .attributes
                        .set(func_op::ATTR_KEY_FUNC_TYPE.clone(), ty_attr);
                }
                let opop = Box::new(FuncOp { op });
                opop.set_symbol_name(ctx, &fname);
                opop
            })
            .into()
    }
}

#[derive(Error, Debug)]
#[error("function does not have function type")]
pub struct FuncOpTypeErr;

impl Verify for FuncOp {
    fn verify(&self, ctx: &Context) -> Result<()> {
        let op = &*self.get_operation().deref(ctx);
        let ty = self.get_type(ctx);
        if !(ty.deref(ctx).is::<FunctionType>()) {
            return verify_err!(op.loc(), FuncOpTypeErr);
        }
        Ok(())
    }
}

impl_op_interface!(ZeroOpdInterface for FuncOp {});
impl_op_interface!(ZeroResultInterface for FuncOp {});

/// A placeholder during parsing to refer to yet undefined operations.
/// MLIR [uses](https://github.com/llvm/llvm-project/blob/185b81e034ba60081023b6e59504dfffb560f3e3/mlir/lib/AsmParser/Parser.cpp#L1075)
/// [UnrealizedConversionCastOp](https://mlir.llvm.org/docs/Dialects/Builtin/#builtinunrealized_conversion_cast-unrealizedconversioncastop)
/// for this purpose.
#[def_op("builtin.forward_ref")]
pub struct ForwardRefOp {}

impl Printable for ForwardRefOp {
    fn fmt(
        &self,
        ctx: &Context,
        _state: &printable::State,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} = {}",
            self.get_result(ctx).unique_name(ctx),
            self.get_opid().disp(ctx),
        )
    }
}

impl_op_interface! (OneResultInterface for ForwardRefOp {});
impl_op_interface! (ZeroOpdInterface for ForwardRefOp {});

#[derive(Error, Debug)]
#[error("{0} is a temporary Op during parsing. It must not exit in a well-formed program.")]
pub struct ForwardRefOpExistenceErr(String);

impl Verify for ForwardRefOp {
    fn verify(&self, ctx: &Context) -> Result<()> {
        verify_err!(
            self.get_operation().deref(ctx).loc(),
            ForwardRefOpExistenceErr(self.get_result(ctx).unique_name(ctx))
        )
    }
}

impl Parsable for ForwardRefOp {
    type Arg = Vec<(Identifier, Location)>;
    type Parsed = OpObj;

    fn parse<'a>(
        state_stream: &mut StateStream<'a>,
        _results: Self::Arg,
    ) -> ParseResult<'a, Self::Parsed> {
        input_err!(
            state_stream.loc(),
            ForwardRefOpExistenceErr(
                ForwardRefOp::get_opid_static()
                    .disp(state_stream.state.ctx)
                    .to_string()
            )
        )?
    }
}

impl ForwardRefOp {
    /// Create a new [ForwardRefOp].
    pub fn new(ctx: &mut Context) -> Self {
        let ty = UnitType::get(ctx).into();
        let op = Operation::new(ctx, Self::get_opid_static(), vec![ty], vec![], vec![], 0);
        ForwardRefOp { op }
    }
}

pub fn register(ctx: &mut Context) {
    ModuleOp::register(ctx, ModuleOp::parser_fn);
    FuncOp::register(ctx, FuncOp::parser_fn);
    ForwardRefOp::register(ctx, ForwardRefOp::parser_fn);
}
