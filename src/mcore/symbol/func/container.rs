use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use parking_lot::RwLock;

use crate::CompactString;

use super::{base as ops, FuncSymbol, SymbolAttr, SymbolAttrValue};

pub(super) fn add_symbol_impl(
    symbols: &mut HashMap<CompactString, Arc<FuncSymbol>>,
    symbol: FuncSymbol,
) -> Arc<FuncSymbol> {
    if let Some(s) = symbols.get_mut(&symbol.name) {
        s.attrs.write().extend(symbol.attrs.read().clone());
        return s.clone();
    }

    let symbol = Arc::new(symbol);
    symbols.insert(symbol.name.clone(), symbol.clone());
    symbol
}

pub(super) fn all_func_symbols() -> &'static RwLock<HashMap<CompactString, Arc<FuncSymbol>>> {
    static INSTANCE: OnceLock<RwLock<HashMap<CompactString, Arc<FuncSymbol>>>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut result = HashMap::new();

        add_symbol_impl(&mut result, ops::equal::symbol());

        add_symbol_impl(&mut result, ops::plus::symbol());
        add_symbol_impl(&mut result, ops::minus::symbol());
        add_symbol_impl(&mut result, ops::inequal::symbol());
        add_symbol_impl(&mut result, ops::more::symbol());
        add_symbol_impl(&mut result, ops::less::symbol());

        add_symbol_impl(&mut result, ops::mul::symbol());
        add_symbol_impl(&mut result, ops::divide::symbol());
        add_symbol_impl(&mut result, ops::less_or_equal::symbol());
        add_symbol_impl(&mut result, ops::more_or_equal::symbol());
        add_symbol_impl(&mut result, ops::power::symbol()); // 11
        add_symbol_impl(&mut result, ops::is::symbol());
        FuncSymbol::add_with_name(&mut result, "known"); // 13
        FuncSymbol::add_with_name(&mut result, "in"); // 14
        FuncSymbol::add_with_name(&mut result, "find");
        FuncSymbol::add_with_name(&mut result, "AnySymbol");
        add_symbol_impl(
            &mut result,
            FuncSymbol::builder()
                .name("=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .with_attr(SymbolAttr::Display, SymbolAttrValue::UStr(" ⟹  ".into()))
                .build(),
        );
        add_symbol_impl(
            &mut result,
            FuncSymbol::builder()
                .name("<=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .with_attr(SymbolAttr::Display, SymbolAttrValue::UStr(" ⟺  ".into()))
                .build(),
        );
        FuncSymbol::add_with_name(&mut result, "&&");
        FuncSymbol::add_with_name(&mut result, "||");

        add_symbol_impl(&mut result, ops::op_true::symbol());
        FuncSymbol::add_with_name(&mut result, "false");

        add_symbol_impl(&mut result, ops::sqrt::symbol());

        FuncSymbol::add_with_name(&mut result, "find");
        FuncSymbol::add_with_name(&mut result, "proof");
        FuncSymbol::add_with_name(&mut result, "transform");
        add_symbol_impl(&mut result, ops::replace::symbol());
        FuncSymbol::add_with_name(&mut result, "replace");
        FuncSymbol::add_with_name(&mut result, "variable");

        FuncSymbol::add_with_name(&mut result, "answer");

        FuncSymbol::add_with_name(&mut result, "set");
        FuncSymbol::add_with_name(&mut result, "empty_set");
        add_symbol_impl(&mut result, ops::symbolic_eq::symbol());
        add_symbol_impl(&mut result, ops::op_not::symbol());
        RwLock::new(result)
    })
}
