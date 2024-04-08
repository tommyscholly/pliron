//! Attributes belonging to the LLVM dialect.

use combine::{choice, parser::char::string, Parser};
use pliron_derive::def_attribute;

use crate::{
    context::Context,
    impl_verify_succ,
    parsable::{self, Parsable},
    printable::{Printable, State},
};

/// Integer overflow flags for arithmetic operations.
/// The description below is from LLVM's
/// [release notes](https://releases.llvm.org/2.6/docs/ReleaseNotes.html)
/// that added the flags.
/// "nsw" and "nuw" bits indicate that the operation is guaranteed to not overflow
/// (in the signed or unsigned case, respectively). This gives the optimizer more information
///  and can be used for things like C signed integer values, which are undefined on overflow.
#[def_attribute("llvm.integer_overlflow_flags")]
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum IntegerOverflowFlagsAttr {
    None,
    Nsw,
    Nuw,
}

impl Parsable for IntegerOverflowFlagsAttr {
    type Arg = ();
    type Parsed = Self;

    fn parse<'a>(
        state_stream: &mut parsable::StateStream<'a>,
        _arg: Self::Arg,
    ) -> parsable::ParseResult<'a, Self> {
        choice((
            string("none").map(|_| IntegerOverflowFlagsAttr::None),
            string("nsw").map(|_| IntegerOverflowFlagsAttr::Nsw),
            string("nuw").map(|_| IntegerOverflowFlagsAttr::Nuw),
        ))
        .parse_stream(state_stream)
        .into()
    }
}

impl_verify_succ!(IntegerOverflowFlagsAttr);

impl Printable for IntegerOverflowFlagsAttr {
    fn fmt(
        &self,
        _ctx: &Context,
        _state: &State,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            IntegerOverflowFlagsAttr::None => write!(f, "none"),
            IntegerOverflowFlagsAttr::Nsw => write!(f, "nsw"),
            IntegerOverflowFlagsAttr::Nuw => write!(f, "nuw"),
        }
    }
}

#[def_attribute("llvm.icmp_predicate")]
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

impl Printable for ICmpPredicateAttr {
    fn fmt(
        &self,
        _ctx: &Context,
        _state: &State,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ICmpPredicateAttr::EQ => write!(f, "eq"),
            ICmpPredicateAttr::NE => write!(f, "ne"),
            ICmpPredicateAttr::SLT => write!(f, "slt"),
            ICmpPredicateAttr::SLE => write!(f, "sle"),
            ICmpPredicateAttr::SGT => write!(f, "sgt"),
            ICmpPredicateAttr::SGE => write!(f, "sge"),
            ICmpPredicateAttr::ULT => write!(f, "ult"),
            ICmpPredicateAttr::ULE => write!(f, "ule"),
            ICmpPredicateAttr::UGT => write!(f, "ugt"),
            ICmpPredicateAttr::UGE => write!(f, "uge"),
        }
    }
}

impl Parsable for ICmpPredicateAttr {
    type Arg = ();
    type Parsed = Self;

    fn parse<'a>(
        state_stream: &mut parsable::StateStream<'a>,
        _arg: Self::Arg,
    ) -> parsable::ParseResult<'a, Self> {
        choice((
            string("eq").map(|_| ICmpPredicateAttr::EQ),
            string("ne").map(|_| ICmpPredicateAttr::NE),
            string("slt").map(|_| ICmpPredicateAttr::SLT),
            string("sle").map(|_| ICmpPredicateAttr::SLE),
            string("sgt").map(|_| ICmpPredicateAttr::SGT),
            string("sge").map(|_| ICmpPredicateAttr::SGE),
            string("ult").map(|_| ICmpPredicateAttr::ULT),
            string("ule").map(|_| ICmpPredicateAttr::ULE),
            string("ugt").map(|_| ICmpPredicateAttr::UGT),
            string("uge").map(|_| ICmpPredicateAttr::UGE),
        ))
        .parse_stream(state_stream)
        .into()
    }
}

impl_verify_succ!(ICmpPredicateAttr);
