use multi_map::MultiMap;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;

use crate::{
    statement::symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    SymbolId,
};

use super::operations::{
    divide, equal, inequal, is, less, less_or_equal, minus, more, more_or_equal, mul, op_not,
    op_true, plus, power, replace, sqrt, symbolic_eq,
};

fn all_symbols() -> &'static RwLock<MultiMap<SymbolId, String, Symbol>> {
    static INSTANCE: OnceCell<RwLock<MultiMap<SymbolId, String, Symbol>>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut result = MultiMap::new();

        add_symbol_impl(&mut result, equal::symbol());

        add_symbol_impl(&mut result, plus::symbol());
        add_symbol_impl(&mut result, minus::symbol());
        add_symbol_impl(&mut result, inequal::symbol());
        add_symbol_impl(&mut result, more::symbol());
        add_symbol_impl(&mut result, less::symbol());

        add_symbol_impl(&mut result, mul::symbol());
        add_symbol_impl(&mut result, divide::symbol());
        add_symbol_impl(&mut result, less_or_equal::symbol());
        add_symbol_impl(&mut result, more_or_equal::symbol());
        add_symbol_impl(&mut result, power::symbol()); // 11
        add_symbol_impl(&mut result, is::symbol());
        add_with_name(&mut result, "known"); // 13
        add_with_name(&mut result, "in"); // 14
        add_with_name(&mut result, "find");
        add_with_name(&mut result, "AnySymbol");
        add_symbol_impl(
            &mut result,
            Symbol::builder()
                .name("=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .build()
                .unwrap(),
        );
        add_symbol_impl(
            &mut result,
            Symbol::builder()
                .name("<=>")
                .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(900))
                .build()
                .unwrap(),
        );
        add_with_name(&mut result, "&&");
        add_with_name(&mut result, "||");

        add_symbol_impl(&mut result, op_true::symbol());
        add_with_name(&mut result, "false");

        add_symbol_impl(&mut result, sqrt::symbol());

        add_with_name(&mut result, "find");
        add_with_name(&mut result, "proof");
        add_with_name(&mut result, "transform");
        add_symbol_impl(&mut result, replace::symbol());
        add_with_name(&mut result, "replace");
        add_with_name(&mut result, "variable");

        add_with_name(&mut result, "answer");

        add_with_name(&mut result, "set");
        add_with_name(&mut result, "empty_set");
        add_symbol_impl(&mut result, symbolic_eq::symbol());
        add_symbol_impl(&mut result, op_not::symbol());
        RwLock::new(result)
    })
}

pub fn symbol_by_id(id: SymbolId) -> Option<Symbol> {
    all_symbols().read().get(&id).cloned()
}

pub fn symbol_by_name(name: &str) -> Option<Symbol> {
    all_symbols().read().get_alt(&name.to_owned()).cloned()
}

pub fn add_symbol(symbol: Symbol) -> Symbol {
    add_symbol_impl(&mut all_symbols().write(), symbol)
}

pub fn add_with_name(symbols: &mut MultiMap<SymbolId, String, Symbol>, name: &str) {
    add_symbol_impl(symbols, Symbol::builder().name(name).build().unwrap());
}

fn add_symbol_impl(symbols: &mut MultiMap<SymbolId, String, Symbol>, mut symbol: Symbol) -> Symbol {
    if let Some(s) = symbols.get_mut_alt(&symbol.name) {
        s.attrs.extend(symbol.attrs.into_iter());
        return s.clone();
    }

    static LAST_ID: OnceCell<RwLock<SymbolId>> = OnceCell::new();
    let last_id = LAST_ID.get_or_init(|| RwLock::new(SymbolId::default()));

    last_id.write().increment();
    symbol.id = *last_id.read();
    symbols.insert(symbol.id, symbol.name.clone(), symbol.clone());
    symbol
}
