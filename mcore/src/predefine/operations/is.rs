use crate::{
    predefine::symbol_by_name,
    term::{
        func_symbol::{FuncSymbol, SymbolAttr, SymbolAttrValue, TruthResult},
        symbol::Symbol,
        TermNode,
    },
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("is")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(800))
        .with_attr(
            SymbolAttr::Display,
            SymbolAttrValue::UStr(" is ".to_owned()),
        )
        .with_truth_checker(Box::new(is))
        .build()
}

pub fn is(root: &TermNode) -> TruthResult {
    if !root.data().is_symbol_name("is") {
        return TruthResult::Unknown;
    }

    if let (Symbol::Number(_), Symbol::FuncSymbol(known_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if *known_id == symbol_by_name("known").unwrap() {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }
    if let (Symbol::Variable(_), Symbol::FuncSymbol(sym_varible_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if *sym_varible_id == symbol_by_name("variable").unwrap() {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }

    TruthResult::Unknown
}
