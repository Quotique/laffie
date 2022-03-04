use crate::{
    predefine::symbol_by_name,
    statement::{
        symbols::{Symbol, TruthResult},
        term::{StatementNode, Term},
    },
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("is")
        .with_truth_checker(Box::new(is))
        .build()
        .unwrap()
}

pub fn is(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("is") {
        return TruthResult::Unknown;
    }

    if let (Term::Number(_), Term::Symbol(known_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if *known_id == symbol_by_name("known").unwrap().id {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }
    if let (Term::Variable(_), Term::Symbol(sym_varible_id)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if *sym_varible_id == symbol_by_name("variable").unwrap().id {
            return TruthResult::True;
        } else {
            return TruthResult::Unknown;
        }
    }

    TruthResult::Unknown
}
