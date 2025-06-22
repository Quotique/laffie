#![allow(clippy::module_inception)]

#[macro_use]
extern crate log;

pub mod rule;
pub mod task;
pub mod term;

use bincode::{Decode, Encode};
use derive_more::{Display, From};

pub use bigdecimal::{BigDecimal as Decimal, Signed};
pub use smartstring::alias::String as CompactString;

#[derive(Clone, Copy, Debug, Default, Display)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(From)]
#[derive(Decode, Encode)]
pub struct RuleId(u64);

#[derive(Clone, Copy, Debug, Default, Display)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(From)]
#[derive(Decode, Encode)]
pub struct NormalizationLevel(u64);

impl RuleId {
    pub fn new(mask: u64, id: u64) -> Self {
        Self(mask | id)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl NormalizationLevel {
    pub fn max() -> Self {
        Self(u64::MAX)
    }
}

pub fn version_str() -> String {
    std::env!("CARGO_PKG_VERSION").to_owned()
}
