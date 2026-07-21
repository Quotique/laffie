#![allow(clippy::module_inception)]

#[macro_use]
extern crate log;

mod rational;
pub mod rule;
pub mod task;
pub mod term;

pub use num::Signed;
pub use num_rational::BigRational as Rational;
pub use rational::{from_decimal_str, number_to_string};
pub use smartstring::alias::String as CompactString;

/// Depth of term normalization, ordered `Off < Units < ConstFold < Full`.
#[derive(Clone, Copy, Debug, Default)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NormLevel {
    /// No rewriting.
    #[default]
    Off,
    /// Drop identity elements (`+0`, `*1`, `/1`, `^1`, …).
    Units,
    /// Additionally fold numeric constants.
    ConstFold,
    /// Full canonicalization: like terms, powers, commutative order.
    Full,
}

impl From<u64> for NormLevel {
    fn from(n: u64) -> Self {
        match n {
            0 => Self::Off,
            1 => Self::Units,
            2 => Self::ConstFold,
            _ => Self::Full,
        }
    }
}

impl std::fmt::Display for NormLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rank = match self {
            Self::Off => 0,
            Self::Units => 1,
            Self::ConstFold => 2,
            Self::Full => 3,
        };
        write!(f, "{rank}")
    }
}

pub fn version_str() -> String {
    std::env!("CARGO_PKG_VERSION").to_owned()
}
