use crate::{
    predefine::symbol_by_name,
    term::{swap_node, FuncSymbol, NodeMapping, Symbol, TermNode, TruthResult},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("!")
        .with_truth_checker(Box::new(is_not))
        .with_calculator(Box::new(not_replace))
        .build()
}

fn not_replace(root: &mut TermNode, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("!") || root.degree() != 1 {
        return false;
    }

    match root
        .front()
        .unwrap()
        .data()
        .func_symbol()
        .map(|x| x.name.clone())
    {
        Some(name) if name == "==" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
            *root.data_mut() = Symbol::FuncSymbol(symbol_by_name("!=").unwrap());
            true
        }
        Some(name) if name == "!=" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
            *root.data_mut() = Symbol::FuncSymbol(symbol_by_name("==").unwrap());
            true
        }
        _ => false,
    }
}

pub fn is_not(root: &TermNode) -> TruthResult {
    if !root.data().is_symbol_name("!") {
        return TruthResult::Unknown;
    }

    if let Some(child) = root.front() {
        return child.check_truth().reverse();
    }

    TruthResult::Unknown
}
