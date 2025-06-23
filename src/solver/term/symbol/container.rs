use std::{collections::HashMap, sync::OnceLock};

use parking_lot::RwLock;

use crate::CompactString;

use super::{base as ops, Symbol, SymbolAttr, SymbolAttrValue, SymbolProgram};

pub(super) fn add_symbol_impl(
    symbols: &mut HashMap<CompactString, SymbolProgram>,
    symbol: SymbolProgram,
) -> Symbol {
    let name = symbol.name.clone();
    if let Some(s) = symbols.get_mut(&symbol.name) {
        s.attrs.extend(symbol.attrs.clone());
    } else {
        symbols.insert(name.clone(), symbol);
    }
    Symbol(name)
}

pub(super) fn all_func_symbols() -> &'static RwLock<HashMap<CompactString, SymbolProgram>> {
    static ALL_SYMBOLS: OnceLock<RwLock<HashMap<CompactString, SymbolProgram>>> = OnceLock::new();
    ALL_SYMBOLS.get_or_init(|| {
        let mut result = HashMap::new();

        add_symbol_impl(&mut result, ops::equal::symbol());

        add_symbol_impl(&mut result, ops::plus::symbol());
        add_symbol_impl(&mut result, ops::inequal::symbol());
        add_symbol_impl(&mut result, ops::more::symbol());
        add_symbol_impl(&mut result, ops::less::symbol());

        add_symbol_impl(&mut result, ops::mul::symbol());
        add_symbol_impl(&mut result, ops::divide::symbol());
        add_symbol_impl(&mut result, ops::less_or_equal::symbol());
        add_symbol_impl(&mut result, ops::more_or_equal::symbol());
        add_symbol_impl(&mut result, ops::power::symbol());
        add_symbol_impl(&mut result, ops::is::symbol());
        SymbolProgram::add_with_name(&mut result, "known");
        SymbolProgram::add_with_name(&mut result, "in");
        SymbolProgram::add_with_name(&mut result, "find");
        SymbolProgram::add_with_name(&mut result, "AnySymbol");
        add_symbol_impl(
            &mut result,
            SymbolProgram::builder()
                .name("=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .with_attr(SymbolAttr::Display, SymbolAttrValue::UStr(" ⟹  ".into()))
                .build(),
        );
        add_symbol_impl(
            &mut result,
            SymbolProgram::builder()
                .name("<=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .with_attr(SymbolAttr::Display, SymbolAttrValue::UStr(" ⟺  ".into()))
                .build(),
        );
        SymbolProgram::add_with_name(&mut result, "&&");
        SymbolProgram::add_with_name(&mut result, "||");

        add_symbol_impl(&mut result, ops::op_true::symbol());
        SymbolProgram::add_with_name(&mut result, "false");

        add_symbol_impl(&mut result, ops::sqrt::symbol());

        SymbolProgram::add_with_name(&mut result, "find");
        SymbolProgram::add_with_name(&mut result, "proof");
        SymbolProgram::add_with_name(&mut result, "transform");
        add_symbol_impl(&mut result, ops::replace::symbol());
        SymbolProgram::add_with_name(&mut result, "replace");
        SymbolProgram::add_with_name(&mut result, "variable");

        SymbolProgram::add_with_name(&mut result, "answer");

        SymbolProgram::add_with_name(&mut result, "set");
        SymbolProgram::add_with_name(&mut result, "empty_set");
        add_symbol_impl(&mut result, ops::symbolic_eq::symbol());
        add_symbol_impl(&mut result, ops::op_not::symbol());
        RwLock::new(result)
    })
}
