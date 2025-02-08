//! Attributes belonging to the LLVM dialect.

use pliron::derive::{def_attribute, format};

use pliron::impl_verify_succ;

/// Integer overflow flags for arithmetic operations.
/// The description below is from LLVM's
/// [release notes](https://releases.llvm.org/2.6/docs/ReleaseNotes.html)
/// that added the flags.
/// "nsw" and "nuw" bits indicate that the operation is guaranteed to not overflow
/// (in the signed or unsigned case, respectively). This gives the optimizer more information
///  and can be used for things like C signed integer values, which are undefined on overflow.
#[def_attribute("llvm.integer_overlflow_flags")]
#[format]
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum IntegerOverflowFlagsAttr {
    None,
    Nsw,
    Nuw,
}

impl_verify_succ!(IntegerOverflowFlagsAttr);

#[def_attribute("llvm.icmp_predicate")]
#[format]
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ICmpPredicateAttr {
    EQ,
    NE,
    SLT,
    SLE,
    SGT,
    SGE,
    ULT,
    ULE,
    UGT,
    UGE,
}

impl_verify_succ!(ICmpPredicateAttr);

/// An index for a GEP can be either a constant or an SSA operand.
/// Contrary to its name, this isn't an [Attribute][pliron::attribute::Attribute].
#[derive(PartialEq, Eq, Clone, Debug)]
#[format]
pub enum GepIndexAttr {
    /// This GEP index is a raw u32 compile time constant
    Constant(u32),
    /// This GEP Index is the SSA value in the containing
    /// [Operation](pliron::operation::Operation)s `operands[idx]`
    OperandIdx(usize),
}

#[def_attribute("llvm.gep_indices")]
#[format("`[` vec($0, CharSpace(`,`)) `]`")]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct GepIndicesAttr(pub Vec<GepIndexAttr>);
impl_verify_succ!(GepIndicesAttr);
