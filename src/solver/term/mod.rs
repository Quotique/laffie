//#![warn(missing_docs)]

mod atom;
mod buffer;
mod refer;
mod refer_mut;
mod substitution;
mod symbol;

pub use atom::{ArgList, Atom, Param, Variable, param, var};
pub use buffer::{SharedTerm, TermBuf, TermPath};
pub(crate) use refer::match_term;
pub use refer::{Term, TermRef};
pub use refer_mut::TermMut;
pub use substitution::{ParamSubstitution, Substitute, VariableSubstitution};
pub use symbol::{
    Symbol, SymbolAttr, SymbolAttrValue, SymbolProgram, Truth, TruthCtx, sym, symbol_names, try_sym,
};

#[cfg(test)]
pub fn term_with_params(text: &'static str) -> TermBuf {
    let states = parser::lang::terms(text).unwrap();
    let term = parser::TermParser::default().try_parse(&states[0]).unwrap();

    rebridge(&term)
}

#[cfg(test)]
pub fn term_with_vars(text: &'static str) -> TermBuf {
    let states = parser::lang::terms(text)
        .map_err(|e| println!("parsing error {text}: {e}"))
        .unwrap();
    let term = parser::TermParser::default()
        .with_variables()
        .try_parse(&states[0])
        .unwrap();

    rebridge(&term)
}

/// Bridge a value produced by the `parser` dev-dependency into this crate's own
/// type. The dev-dependency cycle (solver tests → parser → solver) links a
/// second instance of this crate, so `parser`'s `TermBuf` is a nominally
/// distinct type from ours. A serde roundtrip crosses the boundary safely: both
/// instances are the same source, hence share the wire format.
#[cfg(test)]
fn rebridge<A, B>(value: &A) -> B
where
    A: serde::Serialize,
    B: serde::de::DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
}
