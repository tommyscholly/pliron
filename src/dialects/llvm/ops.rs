use combine::{easy::ParseError, ParseResult};

use crate::{
    common_traits::Verify,
    context::Context,
    declare_op,
    dialect::Dialect,
    dialects::builtin::op_interfaces::IsTerminatorInterface,
    error::Result,
    impl_op_interface,
    op::{Op, OpObj},
    operation::Operation,
    parsable::{Parsable, StateStream},
    printable::{self, Printable},
    use_def_lists::Value,
};

declare_op!(
    /// Equivalent to LLVM's return opcode.
    ///
    /// Operands:
    ///
    /// | operand | description |
    /// |-----|-------|
    /// | `arg` | any type |
    ReturnOp,
    "return",
    "llvm"
);

impl ReturnOp {
    pub fn new_unlinked(ctx: &mut Context, value: Value) -> ReturnOp {
        let op = Operation::new(ctx, Self::get_opid_static(), vec![], vec![value], 0);
        ReturnOp { op }
    }
}

impl Printable for ReturnOp {
    fn fmt(
        &self,
        ctx: &Context,
        _state: &printable::State,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        write!(
            f,
            "{} {}",
            self.get_opid().disp(ctx),
            self.get_operation()
                .deref(ctx)
                .get_operand_ref(0)
                .unwrap()
                .disp(ctx)
        )
    }
}

impl Verify for ReturnOp {
    fn verify(&self, _ctx: &Context) -> Result<()> {
        Ok(())
    }
}

impl Parsable for ReturnOp {
    type Parsed = OpObj;
    fn parse<'a>(
        _state_stream: &mut crate::parsable::StateStream<'a>,
    ) -> ParseResult<Self::Parsed, ParseError<StateStream<'a>>> {
        todo!()
    }
}

impl_op_interface!(IsTerminatorInterface for ReturnOp {});

pub fn register(ctx: &mut Context, dialect: &mut Dialect) {
    ReturnOp::register(ctx, dialect, ReturnOp::parser_fn);
}
