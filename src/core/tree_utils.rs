use super::{
    symbols::{symbol_by_id, SymbolAttr},
    term::{StatementTree, Term},
    utils::SubsetIterator,
};
use std::collections::HashMap;
use trees::{tr, Node, Tree};

type ParamsMap = HashMap<u64, StatementTree>;
type TreeNode = Node<Term>;

pub fn swap_node<F: Clone>(l: &mut Node<F>, r: &mut Node<F>) {
    let mut l_childs: Vec<Tree<F>> = vec![];
    let mut r_childs: Vec<Tree<F>> = vec![];

    while let Some(t) = l.pop_front() {
        l_childs.push(t);
    }
    while let Some(t) = r.pop_front() {
        r_childs.push(t);
    }

    let r_data = r.data.clone();
    r.data = l.data.clone();
    l.data = r_data;
    //(l.root_mut().data, r.root_mut().data) = (r.root().data, l.root().data);
    for i in l_childs {
        r.push_back(i);
    }
    for i in r_childs {
        l.push_back(i);
    }
}

pub fn apply_map(target: &mut TreeNode, params: &ParamsMap) {
    match target.data {
        Term::Param(id) => {
            let replace = params.get(&id);
            match replace {
                Some(r) => {
                    let mut replace = r.clone();
                    target.data = r.root().data.clone();
                    while let Some(_) = target.pop_back() {}
                    while let Some(x) = replace.pop_front() {
                        target.push_back(x);
                    }
                    //target.append(replace.abandon());
                }
                None => {}
            }
        }
        _ => {
            for i in target.iter_mut() {
                apply_map(i, params);
            }
        }
    }
}

pub fn params_map(target: &TreeNode, pattern: &TreeNode) -> Result<Vec<ParamsMap>, String> {
    params_map_impl(target, pattern, ParamsMap::new())
}

fn params_map_impl(
    target: &TreeNode,
    pattern: &TreeNode,
    mut params: ParamsMap,
) -> Result<Vec<ParamsMap>, String> {
    trace!(
        "Pattern: {}, traget: {}, mapping: {:?}",
        pattern,
        target,
        params
    );
    let mut result = vec![];

    match (&pattern.data, &target.data) {
        (Term::Symbol(p_id), Term::Symbol(t_id)) => {
            if p_id != t_id {
                return Err(format!("Expect symbol {}, found: {}", p_id, t_id));
            }
            let sym = symbol_by_id(*p_id).unwrap();

            if sym.attrs.contains_key(&SymbolAttr::Associative) &&
                sym.attrs.contains_key(&SymbolAttr::Commutative)
            {
                for i in SubsetIterator::new(target.degree(), pattern.degree()) {
                    let mut loc_result = vec![params.clone()];
                    let mut parts = vec![tr(Term::Symbol(*p_id)); pattern.degree()];
                    for (id, child) in target.iter().enumerate() {
                        parts[i.as_vec()[id]].push_back(child.to_owned());
                    }

                    for mut p in parts.iter_mut() {
                        if p.degree() == 1 {
                            let mut child = p.pop_front().unwrap();
                            swap_node(&mut p, &mut child);
                        }
                    }

                    for (x, y) in pattern.iter().zip(parts.iter()) {
                        let mut new_result = vec![];
                        for r in loc_result.into_iter() {
                            match params_map_impl(y, x, r) {
                                Ok(mut p) => {
                                    trace!("New mapping: {:?}", p);
                                    new_result.append(&mut p)
                                }
                                Err(e) => trace!("Bad mapping: {}", e),
                            }
                        }
                        loc_result = new_result;
                    }
                    result.append(&mut loc_result);
                }

                if result.len() > 0 {
                    return Ok(result);
                } else {
                    return Err("No mapping found".into());
                }
            } else {
                result.push(params);
                if pattern.degree() != target.degree() {
                    return Err(format!(
                        "Argument size missmatch: {} {}",
                        pattern.degree(),
                        target.degree()
                    ));
                }

                for (x, y) in pattern.iter().zip(target.iter()) {
                    let mut new_result = vec![];
                    for r in result.into_iter() {
                        match params_map_impl(y, x, r) {
                            Ok(mut p) => {
                                trace!("New mapping: {:?}", p);
                                new_result.append(&mut p)
                            }
                            Err(e) => trace!("Bad mapping: {}", e),
                        }
                    }
                    result = new_result;
                }

                if result.len() > 0 {
                    return Ok(result);
                } else {
                    return Err("No mapping found".into());
                }
            }
        }
        (Term::Symbol(p_id), _) => {
            return Err(format!(
                "Expect symbol id: {}, found target: {:?}",
                p_id, &target.data
            ));
        }
        (Term::Param(id), _) => {
            if params.contains_key(id) {
                let node = params.get(id).unwrap();
                let _ = params_map(node, target)?;
            } else {
                params.insert(*id, target.to_owned()); // subtree_clone(target));
            }

            result.push(params);
        }
        (Term::Number(value), Term::Number(other_value)) => {
            if value != other_value {
                return Err(format!("Expect Number {}, found {:?}", value, target.data));
            }

            result.push(params);
        }
        (Term::Number(_), _) => {
            return Err(format!("Expect Number, found: {:?}", target.data));
        }
        (Term::Variable(value), Term::Variable(other_value)) => {
            if value != other_value {
                return Err(format!("Expect Varible {}, found {:?}", value, target.data));
            }

            result.push(params);
        }
        (Term::Variable(_), _) => {
            return Err(format!("Expect Varible, found: {:?}", target.data));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tree_utils_tests {
    use super::*;
    use bigdecimal::BigDecimal as Decimal;
    use core::{symbols::symbols_tests::setup, trees::linked::fully::tr};

    #[test]
    fn simple_param_map_test() {
        setup();
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Variable(1)) / tr(Term::Number(Decimal::from(1)))) /
            tr(Term::Number(Decimal::from(0)));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Param(2))) /
            tr(Term::Number(Decimal::from(0)));
        match params_map(&tree, &pattern) {
            Ok(maps) => {
                assert_eq!(maps.len(), 2);
                let res: Vec<(Tree<Term>, Tree<Term>)> = maps
                    .iter()
                    .map(|x| (x.get(&1).unwrap().clone(), x.get(&2).unwrap().clone()))
                    .collect();
                assert!(res.contains(&(tr(Term::Number(Decimal::from(1))), tr(Term::Variable(1)))));
                assert!(res.contains(&(tr(Term::Number(Decimal::from(1))), tr(Term::Variable(1)))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn same_param_map_test() {
        setup();
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Symbol(3)) / tr(Term::Symbol(4))) /
            tr(Term::Symbol(3));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Symbol(4))) /
            tr(Term::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(*map[0].get(&1).unwrap(), (tr(Term::Symbol(3))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn subtree_param_map_test() {
        setup();
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) /
                (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2))) /
                tr(Term::Symbol(4))) /
            (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2)));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Symbol(4))) /
            tr(Term::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(
                    *map[0].get(&1).unwrap(),
                    (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2)))
                );
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn apply_param_map_test() {
        setup();
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) /
                (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2))) /
                tr(Term::Symbol(4))) /
            (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2)));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Symbol(4))) /
            tr(Term::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(map.len(), 1);
                let mut test = tr(Term::Symbol(1)) / tr(Term::Param(1));
                apply_map(&mut test, &map[0]);

                assert_eq!(
                    test,
                    tr(Term::Symbol(1)) /
                        (tr(Term::Symbol(3)) / tr(Term::Symbol(1)) / tr(Term::Symbol(2)))
                );
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }
}
