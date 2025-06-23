//#![warn(missing_docs)]

mod codec;
mod index;
mod props;
mod subterm;
mod subterm_mut;
mod symbol;
mod term_node;
mod term_tree;

pub use index::NodePosition;
pub use props::TermProps;
pub use subterm::{ParamsMapping, Subterm};
pub use subterm_mut::{SubtermMut, VariablesMap};
pub use symbol::{Symbol, SymbolAttr, SymbolAttrValue, SymbolProgram, Truth};
pub use term_node::{Param, Placeholder, TermNode, Variable};
pub use term_tree::{SymbolTree, Term};

#[cfg(test)]
pub fn term_with_params(text: &'static str) -> Term {
    let states = parser::lang::terms(text).unwrap();
    let term = parser::TermParser::new(&states[0]).parse().unwrap();

    unsafe { std::mem::transmute::<_, Term>(term) }
}

#[cfg(test)]
pub fn term_with_vars(text: &'static str) -> Term {
    let states = parser::lang::terms(text)
        .map_err(|e| println!("parsing error {text}: {e}"))
        .unwrap();
    let term = parser::TermParser::new(&states[0])
        .with_variables()
        .parse()
        .unwrap();

    unsafe { std::mem::transmute::<_, Term>(term) }
}
