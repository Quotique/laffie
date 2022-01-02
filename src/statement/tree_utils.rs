use std::{
    collections::{HashMap, HashSet, VecDeque},
    iter::Iterator,
    ops::Deref,
};

use eyre::{bail, ensure, Result};
use trees::{tr, Node, Tree};

use utils::SubsetIterator;

use super::{
    index::NodePosition,
    symbols::{symbol_by_id, SymbolAttr},
    term::{Param, StatementNode, StatementTree, Term, Variable},
};

pub type ParamsMap = HashMap<Param, StatementTree>;
pub type VariablesMap = HashMap<Variable, StatementTree>;

pub fn swap_node<F: Clone>(l: &mut Node<F>, r: &mut Node<F>) {
    let mut l_childs: Vec<Tree<F>> = vec![];
    let mut r_childs: Vec<Tree<F>> = vec![];

    while let Some(t) = l.pop_front() {
        l_childs.push(t);
    }
    while let Some(t) = r.pop_front() {
        r_childs.push(t);
    }

    let r_data = r.data().clone();
    *r.data_mut() = l.data().clone();
    *l.data_mut() = r_data;
    //(l.root_mut().data, r.root_mut().data) = (r.root().data, l.root().data);
    for i in l_childs {
        r.push_back(i);
    }
    for i in r_childs {
        l.push_back(i);
    }
}

pub fn replace<F: Clone + PartialEq + Unpin>(arg: &mut Node<F>, src: &Node<F>, dst: &Node<F>) {
    arg.iter_mut().for_each(|child| {
        replace(child.get_mut(), src, dst);
    });
    arg.iter_mut().for_each(|child| {
        if child.deref() == src {
            swap_node(child.get_mut(), &mut dst.deep_clone().root_mut());
        }
    });
}

pub trait NodeMapping {
    fn find_subtree_map(&self, target: &Self) -> Vec<(Vec<ParamsMap>, NodePosition)>;

    fn apply_variable_map(&mut self, variables: &VariablesMap);

    fn apply_param_map(&mut self, params: &ParamsMap);

    fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = StatementTree> + '_>;

    fn map(&self, pattern: &StatementNode) -> Result<Vec<ParamsMap>>;

    fn evaluate(&mut self) -> bool;

    fn check_truth(&self) -> bool;

    fn symbols(&self) -> HashSet<u64>;
}

impl NodeMapping for StatementNode {
    fn symbols(&self) -> HashSet<u64> {
        self.bfs()
            .iter
            .filter_map(|x| x.data.symbol_id())
            .collect::<HashSet<u64>>()
    }

    fn find_subtree_map(&self, target: &Self) -> Vec<(Vec<ParamsMap>, NodePosition)> {
        let mut result = vec![];
        let mut queue = VecDeque::new();
        queue.push_back((target, NodePosition::root()));

        while let Some((node, pos)) = queue.pop_front() {
            if let Ok(mapping) = self
                .map(node)
                .map_err(|_| trace!(target: "pattern_match", "No match for {} to {}", self, node))
            {
                result.push((mapping, pos.clone()));
            }

            for (num, i) in node.iter().enumerate() {
                queue.push_back((i, pos.clone().child(num)));
            }
        }
        result
    }

    fn apply_variable_map(&mut self, variables: &VariablesMap) {
        if let Some(mut v) = self
            .data()
            .variable()
            .map(|x| variables.get(x))
            .flatten()
            .cloned()
        {
            swap_node(self, &mut v.root_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_variable_map(variables);
            }
        }
    }

    fn apply_param_map(&mut self, params: &ParamsMap) {
        if let Some(mut v) = self
            .data()
            .param()
            .map(|x| params.get(x))
            .flatten()
            .cloned()
        {
            swap_node(self, &mut v.root_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_param_map(params);
            }
        }
    }

    fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = StatementTree> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().symbol().unwrap();
            let mut parts = vec![tr(Term::Symbol(s.id)); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]].push_back(child.deep_clone());
            }

            for p in parts.iter_mut() {
                if p.degree() == 1 {
                    let mut child = p.pop_front().unwrap();
                    swap_node(&mut p.root_mut(), &mut child.root_mut());
                }
            }
            let mut result = tr(Term::Symbol(s.id));
            for p in parts.into_iter() {
                result.push_back(p);
            }
            result
        }))
    }

    fn map(&self, pattern: &StatementNode) -> Result<Vec<ParamsMap>> {
        params_map_impl(pattern, self, ParamsMap::new())
    }

    fn evaluate(&mut self) -> bool {
        if let Some(symbol) = &self.data().symbol() {
            return symbol.evaluate(self);
        }
        false
    }

    fn check_truth(&self) -> bool {
        if let Some(symbol) = &self.data().symbol() {
            return symbol.check_truth(self);
        }
        false
    }
}

fn display_mapping(map: &[ParamsMap]) -> String {
    format!(
        "[{}]",
        map.iter()
            .map(|m| {
                format!(
                    "{{ {} }}",
                    m.iter()
                        .map(|(x, y)| format!("{}: {}", x, y))
                        .collect::<Vec<String>>()
                        .join(",")
                )
            })
            .collect::<Vec<String>>()
            .join(",")
    )
}

fn params_map_arguments(
    target: &StatementNode,
    pattern: &StatementNode,
    result: &mut Vec<ParamsMap>,
) -> Result<()> {
    let placeholder_pos = pattern.iter().enumerate().find_map(|(pos, x)| {
        if x.data().is_placeholder() {
            Some(pos)
        } else {
            None
        }
    });
    let args_delta = target.degree() as i64 - pattern.degree() as i64;
    ensure!(
        placeholder_pos.is_some() && args_delta >= 0 ||
            placeholder_pos.is_none() && args_delta == 0,
        "Argument size missmatch: {} {}",
        pattern.degree(),
        target.degree()
    );

    for (p, t) in pattern.iter().zip(target.iter()) {
        let mut new_result = vec![];
        if p.data().is_placeholder() {
            continue;
        }
        for r in result.drain(..) {
            if let Ok(mut p) = params_map_impl(t, p, r) {
                trace!(target: "pattern_match", "New mapping: [{}]", display_mapping(&p));
                new_result.append(&mut p);
            }
        }
        *result = new_result;
    }

    Ok(())
}

fn params_map_impl(
    target: &StatementNode,
    pattern: &StatementNode,
    mut params: ParamsMap,
) -> Result<Vec<ParamsMap>> {
    trace!(target: "pattern_match", "Pattern: {}, traget: {}, mapping: {:?}", pattern, target, params);
    let mut result = vec![];

    match (&pattern.data(), &target.data()) {
        (Term::Symbol(p_id), Term::Symbol(t_id)) => {
            ensure!(p_id == t_id, "Expect symbol {}, found: {}", p_id, t_id);
            let sym = symbol_by_id(*p_id).unwrap();

            if sym.attrs.contains_key(&SymbolAttr::Associative) &&
                sym.attrs.contains_key(&SymbolAttr::Commutative)
            {
                for parts in target.subsets(pattern.degree()) {
                    let mut loc_result = vec![params.clone()];
                    params_map_arguments(&parts, pattern, &mut loc_result).expect("must match");
                    result.append(&mut loc_result);
                }

                ensure!(!result.is_empty(), "No mapping found");
                return Ok(result);
            } else {
                result.push(params);

                params_map_arguments(target, pattern, &mut result)?;

                ensure!(!result.is_empty(), "No mapping found");
                return Ok(result);
            }
        }
        (Term::Symbol(p_id), _) => {
            bail!(
                "Expect symbol id: {}, found target: {:?}",
                p_id,
                &target.data()
            );
        }
        (Term::Param(p), _) => {
            if params.contains_key(p) {
                let node = params.get(p).unwrap();
                let _ = target.map(node)?;
            } else {
                params.insert(p.clone(), target.deep_clone());
            }

            result.push(params);
        }
        (Term::Number(value), Term::Number(other_value)) => {
            ensure!(
                value == other_value,
                "Expect Number {}, found {:?}",
                value,
                target.data()
            );

            result.push(params);
        }
        (Term::Number(_), _) => {
            bail!("Expect Number, found: {:?}", target.data());
        }
        (Term::Variable(value), Term::Variable(other_value)) => {
            ensure!(
                value == other_value,
                "Expect Varible {}, found {:?}",
                value,
                target.data()
            );

            result.push(params);
        }
        (Term::Variable(_), _) => {
            bail!("Expect Varible, found: {:?}", target.data());
        }
        (Term::Placeholder, _) => {
            bail!("Mapping placeholder")
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use parser::{ra, statement_with_params, statement_with_vars, StatementParser};
    use predefine::setup;
    use statement::Statement;

    use super::*;

    fn dump_mapping(map: &[ParamsMap]) -> String {
        let mut result: String = Default::default();
        result += "[";
        for m in map {
            let mut v: Vec<_> = m.iter().collect();
            v.sort_by(|x, y| x.0.cmp(y.0));
            result += "{";
            for x in v {
                result += &format!(" {}: {},", x.0, x.1);
            }
            result += "}";
        }

        result += "]";

        result
    }

    #[test]
    fn replace_test() {
        setup();

        let mut test_state = StatementParser::new(&ra::statements("a/(a+b)").unwrap()[0])
            .parse()
            .unwrap();
        let test_pattern = StatementParser::new(&ra::statements("(a+b)").unwrap()[0])
            .parse()
            .unwrap();
        let test_replace = StatementParser::new(&ra::statements("c").unwrap()[0])
            .parse()
            .unwrap();

        replace(
            test_state.root_mut().get_mut(),
            test_pattern.root(),
            test_replace.root(),
        );
        assert_eq!(
            test_state,
            Statement::from(
                tr(Term::with_symbol_name("/").unwrap()) /
                    tr(Term::Param("a".parse().unwrap())) /
                    tr(Term::Param("c".parse().unwrap()))
            )
        );
    }

    #[test]
    fn simple_param_map_test() {
        setup();

        let statement = statement_with_vars("x + 1 == 0");
        let pattern = statement_with_params("a + b == 0");

        let maps = pattern
            .root()
            .map(statement.root())
            .map_err(|e| println!("Error: {}", e))
            .unwrap();
        insta::assert_debug_snapshot!(dump_mapping(&maps));
    }

    #[test]
    fn same_param_map_test() {
        setup();

        let statement = statement_with_vars("x + 1 == x");
        let pattern = statement_with_params("a + 1 == a");

        let maps = pattern
            .root()
            .map(statement.root())
            .map_err(|e| println!("Error: {}", e))
            .unwrap();
        insta::assert_debug_snapshot!(dump_mapping(&maps));
    }

    #[test]
    fn subtree_param_map_test() {
        setup();

        let statement = statement_with_vars("(x - 1) + 4 == x - 1");
        let pattern = statement_with_params("a + 4 == b");

        let maps = pattern
            .root()
            .map(statement.root())
            .map_err(|e| println!("Error: {}", e))
            .unwrap();
        insta::assert_debug_snapshot!(dump_mapping(&maps));
    }

    #[test]
    fn apply_param_map_test() {
        setup();

        let statement = statement_with_vars("(x - 1) + 4 == x - 1");
        let pattern = statement_with_params("a + 4 == b");

        let maps = pattern
            .root()
            .map(statement.root())
            .map_err(|e| println!("Error: {}", e))
            .unwrap();
        assert_eq!(maps.len(), 1);

        let mut test = statement_with_params("a + 1");
        test.root_mut().apply_param_map(&maps[0]);
        insta::assert_debug_snapshot!(test);
    }

    #[test]
    fn placeholder_test() {
        setup();

        let pattern = statement_with_params("set(a, ..) is known");
        let statement = statement_with_vars("set(3, 5, 7) is known");

        let maps = pattern
            .root()
            .map(statement.root())
            .map_err(|e| println!("Error: {}", e))
            .unwrap();
        insta::assert_debug_snapshot!(dump_mapping(&maps));
    }
}
