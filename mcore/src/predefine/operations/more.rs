use crate::statement::{
    symbols::Symbol,
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name(">")
        .with_truth_checker(Box::new(more))
        .build()
        .unwrap()
}

pub fn more(root: &StatementNode) -> bool {
    if !root.data().is_symbol_name(">") {
        return false;
    }

    if let (Term::Number(d1), Term::Number(d2)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        return d1 > d2;
    }

    false
}
