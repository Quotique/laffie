use crate::{
    predefine::symbol_by_name,
    statement::{
        symbols::{Symbol, TruthResult},
        term::{StatementNode, Term},
        tree_utils::{swap_node, NodeMapping},
    },
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("!")
        .with_truth_checker(Box::new(is_not))
        .with_calculator(Box::new(not_replace))
        .build()
        .unwrap()
}

fn not_replace(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("!") || root.degree() != 1 {
        return false;
    }

    match root.front().unwrap().data().symbol().map(|x| x.name) {
        Some(name) if name == "==" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
            *root.data_mut() = Term::Symbol(symbol_by_name("!=").unwrap().id);
            return true;
        }
        Some(name) if name == "!=" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
            *root.data_mut() = Term::Symbol(symbol_by_name("==").unwrap().id);
            return true;
        }
        _ => {
            return false;
        }
    }
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
