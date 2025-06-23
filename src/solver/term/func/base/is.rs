use crate::term::{FuncSymbol, Subterm, Symbol, SymbolAttr, SymbolAttrValue, TruthResult};

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

pub fn is(root: Subterm) -> TruthResult {
    if !root.data().is_symbol_name("is") {
        return TruthResult::Unknown;
    }

    if let (Symbol::Number(_), Symbol::FuncSymbol(known_id)) = (
        &root.first_arg().unwrap().data(),
        &root.last_arg().unwrap().data(),
    ) {
        if known_id.name == "known" {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }
    if let (Symbol::Variable(_), Symbol::FuncSymbol(sym_varible_id)) = (
        &root.first_arg().unwrap().data(),
        &root.last_arg().unwrap().data(),
    ) {
        if sym_varible_id.name == "variable" {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }

    TruthResult::Unknown
}
