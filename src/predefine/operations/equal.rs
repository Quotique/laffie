use statement::{
    symbols::{Symbol, SymbolAttr, SymbolAttrValue},
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("==")
        .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(500))
        .with_truth_checker(Box::new(equal))
        .build()
        .unwrap()
}

pub fn equal(root: &StatementNode) -> bool {
    if !root.data().is_symbol_name("==") {
        return false;
    }

    if let (Term::Number(d1), Term::Number(d2)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        return d1 == d2;
    }

    false
}
