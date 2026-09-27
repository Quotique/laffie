//! The parse pipeline as it stood before the expression rewrite, kept verbatim
//! as the reference `corpus_diff` compares the new pipeline against.
//! TODO: delete with `Tree` and `NodeData` once the loader runs on the new path.
#![allow(dead_code)]

pub(crate) mod grammar;
pub(crate) mod lang;
mod rule;
mod symbol;
mod task;
mod term;

pub(crate) use self::{rule::RuleParser, symbol::SymbolParser, task::TaskParser};
use crate::{Node, ParserError, Tree};
