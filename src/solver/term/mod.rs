//#![warn(missing_docs)]

mod atom;
mod buffer;
mod codec;
mod refer;
mod refer_mut;
mod substitution;
mod symbol;

pub use atom::{ArgList, Atom, Param, Variable, param, var};
pub use buffer::{SharedTerm, TermBuf, TermPath};
pub use refer::{Term, TermRef};
pub use refer_mut::TermMut;
pub use substitution::{ParamSubstitution, VariableSubstitution};
pub use symbol::{Symbol, SymbolAttr, SymbolAttrValue, SymbolProgram, Truth, sym, try_sym};

#[cfg(test)]
pub fn term_with_params(text: &'static str) -> TermBuf {
    let states = parser::lang::terms(text).unwrap();
    let term = parser::TermParser::default().try_parse(&states[0]).unwrap();

    unsafe { std::mem::transmute::<_, TermBuf>(term) }
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

    unsafe { std::mem::transmute::<_, TermBuf>(term) }
}
