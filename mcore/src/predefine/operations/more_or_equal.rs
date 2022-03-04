use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::{StatementNode, Term},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name(">=")
        .with_truth_checker(Box::new(more_or_equal))
        .build()
        .unwrap()
}

pub fn more_or_equal(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name(">=") {
        return TruthResult::Unknown;
    }

    if let (Term::Number(d1), Term::Number(d2)) =
        (&root.front().unwrap().data(), &root.back().unwrap().data())
    {
        if d1 >= d2 {
            return TruthResult::True;
        } else {
            return TruthResult::False;
        }
    }

    TruthResult::Unknown
}
