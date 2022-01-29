#![allow(clippy::module_inception)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub mod parser;
pub mod predefine;
pub mod problem;
pub mod rule;
pub mod statement;
pub mod utils;

use std::env;

pub fn version_str() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
