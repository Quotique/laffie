use crate::statement::{
    symbols::{Symbol, TruthResult},
    term::StatementNode,
    tree_utils::NodeMapping,
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("!")
        .with_truth_checker(Box::new(is_not))
        .build()
        .unwrap()
}

pub fn is_not(root: &StatementNode) -> TruthResult {
    if !root.data().is_symbol_name("!") {
        return TruthResult::Unknown;
    }

    if let Some(child) = root.front() {
        return child.check_truth().reverse();
    }

    TruthResult::Unknown
}
