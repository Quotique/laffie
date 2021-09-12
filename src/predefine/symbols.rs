use std::{collections::HashMap, sync::Once};

use statement::symbols::{add_symbol, Symbol, SymbolAttr, SymbolAttrValue};

use super::operations::{divide, minus, mul, plus, power, replace, sqrt};

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        Symbol::add_with_name("=="); // 1

        let mut attr = HashMap::new();
        attr.insert(SymbolAttr::Associative, SymbolAttrValue::None);
        attr.insert(SymbolAttr::Commutative, SymbolAttrValue::None);

        add_symbol(plus::symbol());
        add_symbol(minus::symbol());
        Symbol::add_with_name("!="); // 4
        Symbol::add_with_name(">"); // 5
        Symbol::add_with_name("<"); // 6

        add_symbol(mul::symbol());
        add_symbol(divide::symbol());
        Symbol::add_with_name("<="); // 9
        Symbol::add_with_name(">="); // 10
        add_symbol(power::symbol()); // 11
        Symbol::add_with_name("is"); // 12
        Symbol::add_with_name("known"); // 13
        Symbol::add_with_name("in"); // 14
        Symbol::add_with_name("find");
        Symbol::add_with_name("AnySymbol");
        Symbol::add_with_name("==");
        Symbol::add_with_name("=>");
        Symbol::add_with_name("<=>");
        Symbol::add_with_name("&&");
        Symbol::add_with_name("||");

        Symbol::add_with_name("true");
        Symbol::add_with_name("false");

        add_symbol(sqrt::symbol());

        Symbol::add_with_name("find");
        Symbol::add_with_name("proof");
        Symbol::add_with_name("transform");
        add_symbol(replace::symbol());
        Symbol::add_with_name("replace");
        Symbol::add_with_name("variable");

        Symbol::add_with_name("answer");

        Symbol::add_with_name("set");
        Symbol::add_with_name("empty_set");
    });
}
