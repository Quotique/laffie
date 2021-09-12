use std::sync::Once;

use statement::symbols::{add_symbol, Symbol};

use super::operations::{
    divide, equal, inequal, is, less, less_or_equal, minus, more, more_or_equal, mul, op_true,
    plus, power, replace, sqrt,
};

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        add_symbol(equal::symbol());

        add_symbol(plus::symbol());
        add_symbol(minus::symbol());
        add_symbol(inequal::symbol());
        add_symbol(more::symbol());
        add_symbol(less::symbol());

        add_symbol(mul::symbol());
        add_symbol(divide::symbol());
        add_symbol(less_or_equal::symbol());
        add_symbol(more_or_equal::symbol());
        add_symbol(power::symbol()); // 11
        add_symbol(is::symbol());
        Symbol::add_with_name("known"); // 13
        Symbol::add_with_name("in"); // 14
        Symbol::add_with_name("find");
        Symbol::add_with_name("AnySymbol");
        Symbol::add_with_name("=>");
        Symbol::add_with_name("<=>");
        Symbol::add_with_name("&&");
        Symbol::add_with_name("||");

        add_symbol(op_true::symbol());
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
