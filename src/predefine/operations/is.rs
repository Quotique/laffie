use statement::{
    symbols::{symbol_by_name, Symbol},
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("is")
        .with_truth_checker(Box::new(is))
        .build()
        .unwrap()
}

pub fn is(root: &StatementNode) -> bool {
    if !root.data().is_symbol_name("is") {
        return false;
    }

    if let (Term::Number(_), Term::Symbol(known_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        return *known_id == symbol_by_name("known").unwrap().id;
    }
    if let (Term::Variable(_), Term::Symbol(sym_varible_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        return *sym_varible_id == symbol_by_name("variable").unwrap().id;
    }

    false
}
