use crate::{
    term::{swap_node, FuncSymbol, SymbolNodeMut, Term, VariablesMap},
    NormalizationLevel,
};

pub fn symbol() -> FuncSymbol {
    FuncSymbol::builder()
        .name("replace")
        .with_calculator(Box::new(replace))
        .build()
}

pub fn replace(root: &mut SymbolNodeMut, _: NormalizationLevel) -> bool {
    if !root.data().is_symbol_name("replace") || root.degree() != 2 {
        return false;
    }
    // TODO: ?
    // if root.bfs().iter.any(|x| x.data.param().is_some()) {
    //     return false;
    // }

    let map = root
        .pop_front()
        .expect("replace must have a first argument");
    let map = into_variable_map(map);

    let mut term = root
        .pop_front()
        .expect("replace must have a second argument");

    term.root_mut().apply_variable_map(&map);

    swap_node(root, &mut term.root_mut());
    true
}

fn into_variable_map(mut state: Term) -> VariablesMap {
    let mut result = VariablesMap::default();

    if !state.data().is_symbol_name("==") || state.root().degree() != 2 {
        return result;
    }
    let var = state.root().front().expect("must be");

    if let Some(v) = var.data().variable() {
        result.insert(v.clone(), state.root_mut().pop_back().unwrap());
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::{term::term_with_vars, NormalizationLevel};

    #[test]
    fn replace_test() {
        insta::assert_debug_snapshot!(
            term_with_vars(r#"replace(x == 5, x^4 - 25*x^2 + 60*x -36 != 0)"#)
                .normalize(NormalizationLevel::max()),
            @"!=(264, 0)");
    }
}
