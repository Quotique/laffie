use std::{collections::HashMap, sync::Once};

use core::symbols::{add_symbol, Symbol, SymbolAttr, SymbolAttrValue};

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        Symbol::add_with_name("=="); // 1

        let mut attr = HashMap::new();
        attr.insert(SymbolAttr::Infix, SymbolAttrValue::UInt(10));
        attr.insert(SymbolAttr::Associative, SymbolAttrValue::None);
        attr.insert(SymbolAttr::Commutative, SymbolAttrValue::None);

        add_symbol(Symbol {
            id:    0,
            name:  "+".into(), // 2
            attrs: attr,
        });
        Symbol::add_with_name("-"); // 3
        Symbol::add_with_name("!="); // 4
        Symbol::add_with_name(">"); // 5
        Symbol::add_with_name("<"); // 6
        Symbol::add_with_name("*"); // 7
        Symbol::add_with_name("/"); // 8
        Symbol::add_with_name("<="); // 9
        Symbol::add_with_name(">="); // 10
        Symbol::add_with_name("^"); // 11
        Symbol::add_with_name("is"); // 12
        Symbol::add_with_name("known"); // 13
        Symbol::add_with_name("in"); // 14
    });
}
