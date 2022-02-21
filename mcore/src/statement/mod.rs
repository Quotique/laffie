mod codec;
mod index;
mod mapping;
mod marked_statement;
mod statement;
mod statement_display;
pub mod symbols;
pub mod term;
pub mod tree_utils;

pub use self::{
    index::NodePosition,
    mapping::ParamsMapping,
    marked_statement::MarkedStatement,
    statement::Statement,
    symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    term::{CompactString, StatementNode, StatementTree},
    tree_utils::NodeMapping,
};

#[cfg(test)]
pub fn statement_with_params(text: &'static str) -> Statement {
    let states = parser::lang::statements(text).unwrap();
    let statement = parser::StatementParser::new(&states[0]).parse().unwrap();

    unsafe { std::mem::transmute::<_, Statement>(statement) }
}

#[cfg(test)]
pub fn statement_with_vars(text: &'static str) -> Statement {
    let states = parser::lang::statements(text).unwrap();
    let statement = parser::StatementParser::new(&states[0])
        .with_variables()
        .parse()
        .unwrap();

    unsafe { std::mem::transmute::<_, Statement>(statement) }
}
