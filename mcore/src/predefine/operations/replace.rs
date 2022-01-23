use trees::Tree;

use crate::statement::{
    symbols::Symbol,
    term::{StatementNode, Term},
    tree_utils::{swap_node, NodeMapping, VariablesMap},
};

pub fn symbol() -> Symbol {
    Symbol::builder()
        .name("replace")
        .with_calculator(Box::new(replace))
        .build()
        .unwrap()
}

pub fn replace(root: &mut StatementNode) -> bool {
    if !root.data().is_symbol_name("replace") || root.degree() != 2 {
        return false;
    }
    if root.bfs().iter.any(|x| x.data.param().is_some()) {
        return false;
    }

    let map = root.pop_front().expect("must be");
    let map = into_variable_map(map);

    let mut statement = root.pop_front().expect("must be");

    statement.root_mut().apply_variable_map(&map);

    swap_node(root, &mut statement.root_mut());
    true
}

fn into_variable_map(mut state: Tree<Term>) -> VariablesMap {
    let mut result = VariablesMap::default();

    if !state.data().is_symbol_name("==") || state.degree() != 2 {
        return result;
    }
    let var = state.front().expect("must be");

    if let Some(v) = var.data().variable() {
        result.insert(v.clone(), state.pop_back().unwrap());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::{ra, StatementParser},
        predefine::setup,
    };

    #[test]
    fn replace_test() {
        setup();

        let test_statement = r#"replace(x == 5, x^4 - 25*x^2 + 60*x -36 != 0)"#;
        let state = ra::statements(test_statement).unwrap();

        let mut state = StatementParser::new(&state[0])
            .with_variables()
            .parse()
            .unwrap();

        assert!(replace(&mut state.root_mut()));
        state.inpl_normalize();
        insta::assert_debug_snapshot!(state);
    }
}
