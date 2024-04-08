//! [Op] Interfaces defined in the LLVM dialect.

use thiserror::Error;

use crate::{
    context::{Context, Ptr},
    dialects::builtin::{
        op_interfaces::{OneResultInterface, SameOperandsAndResultType},
        types::{IntegerType, Signedness},
    },
    error::Result,
    location::Located,
    op::{op_cast, Op},
    operation::Operation,
    r#type::{TypeObj, Typed},
    use_def_lists::Value,
    verify_err,
};

use super::{attributes::IntegerOverflowFlagsAttr, types::PointerType};

#[derive(Error, Debug)]
#[error("Binary Arithmetic Op must have exactly two operands and one result")]
pub struct BinArithOpErr;

/// Binary arithmetic [Op].
pub trait BinArithOp: Op + SameOperandsAndResultType {
    /// Create a new binary arithmetic operation given the operands.
    fn new(ctx: &mut Context, lhs: Value, rhs: Value) -> Self
    where
        Self: Sized,
    {
        let op = Operation::new(
            ctx,
            Self::get_opid_static(),
            vec![lhs.get_type(ctx)],
            vec![lhs, rhs],
            0,
        );
        *Operation::get_op(op, ctx).downcast::<Self>().ok().unwrap()
    }

    fn verify(op: &dyn Op, ctx: &Context) -> Result<()>
    where
        Self: Sized,
    {
        let op = op.get_operation().deref(ctx);
        if op.get_num_results() != 1 || op.get_num_operands() != 2 {
            return verify_err!(op.loc(), BinArithOpErr);
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("Integer binary arithmetic Op can only have signless integer result/operand type")]
pub struct IntBinArithOpErr;

/// Integer binary arithmetic [Op]
pub trait IntBinArithOp: Op + BinArithOp {
    fn verify(op: &dyn Op, ctx: &Context) -> Result<()>
    where
        Self: Sized,
    {
        let ty = op_cast::<dyn SameOperandsAndResultType>(op)
            .expect("Op must impl SameOperandsAndResultType")
            .get_type(ctx)
            .deref(ctx);
        let Some(int_ty) = ty.downcast_ref::<IntegerType>() else {
            return verify_err!(op.get_operation().deref(ctx).loc(), IntBinArithOpErr);
        };

        if int_ty.get_signedness() != Signedness::Signless {
            return verify_err!(op.get_operation().deref(ctx).loc(), IntBinArithOpErr);
        }

        Ok(())
    }
}

/// Attribute key for integer overflow flags.
pub const ATTR_KEY_INTEGER_OVERFLOW_FLAGS: &str = "llvm.integer_overflow_flags";

#[derive(Error, Debug)]
#[error("IntegerOverflowFlag missing on Op")]
pub struct IntBinArithOpWithOverflowFlagErr;

/// Integer binary arithmetic [Op] with [IntegerOverflowFlagsAttr]
pub trait IntBinArithOpWithOverflowFlag: Op + IntBinArithOp {
    /// Get the integer overflow flag on this [Op].
    fn integer_overflow_flag(op: &dyn Op, ctx: &Context) -> IntegerOverflowFlagsAttr
    where
        Self: Sized,
    {
        op.get_operation()
            .deref(ctx)
            .attributes
            .get(ATTR_KEY_INTEGER_OVERFLOW_FLAGS)
            .expect("Integer overflow flag missing")
            .downcast_ref::<IntegerOverflowFlagsAttr>()
            .expect("Attribute expected to be IntegerOverflowFlag")
            .clone()
    }

    /// Set the integer overflow flag for this [Op].
    fn set_integer_overflow_flag(op: &dyn Op, ctx: &Context, flag: IntegerOverflowFlagsAttr)
    where
        Self: Sized,
    {
        op.get_operation()
            .deref_mut(ctx)
            .attributes
            .insert(ATTR_KEY_INTEGER_OVERFLOW_FLAGS, Box::new(flag));
    }

    fn verify(op: &dyn Op, ctx: &Context) -> Result<()>
    where
        Self: Sized,
    {
        let op = op.get_operation().deref(ctx);
        if !matches!(op.attributes.get(ATTR_KEY_INTEGER_OVERFLOW_FLAGS), Some(attr) if attr.is::<IntegerOverflowFlagsAttr>())
        {
            return verify_err!(op.loc(), IntBinArithOpWithOverflowFlagErr);
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("Result must be a pointer type, but is not")]
pub struct PointerTypeResultVerifyErr;

/// An [Op] with a single result whose type is [PointerType]
pub trait PointerTypeResult: Op + OneResultInterface {
    /// Get the pointee type of the result pointer.
    fn result_pointee_type(&self, ctx: &Context) -> Ptr<TypeObj> {
        self.result_type(ctx)
            .deref(ctx)
            .downcast_ref::<PointerType>()
            .unwrap()
            .get_pointee_type()
    }

    fn verify(op: &dyn Op, ctx: &Context) -> Result<()>
    where
        Self: Sized,
    {
        if !op_cast::<dyn OneResultInterface>(op)
            .expect("An Op here must impl OneResultInterface")
            .result_type(ctx)
            .deref(ctx)
            .is::<PointerType>()
        {
            return verify_err!(
                op.get_operation().deref(ctx).loc(),
                PointerTypeResultVerifyErr
            );
        }

        Ok(())
    }
}
