use std::collections::HashMap;

use trees::{Node, Tree};

use super::term::{StatementTree, Term};

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
                    target.append(replace.abandon());
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

pub fn params_map(target: &TreeNode, pattern: &TreeNode) -> Result<ParamsMap, String> {
    params_map_impl(target, pattern, ParamsMap::new())
}

fn params_map_impl(
    target: &TreeNode,
    pattern: &TreeNode,
    mut params: ParamsMap,
) -> Result<ParamsMap, String> {
    trace!("Pattern: {}, traget: {}", pattern, target);
    match &pattern.data {
        Term::Symbol(id) => match &target.data {
            &Term::Symbol(other_id) => {
                if *id != other_id {
                    return Err(format!("Expect symbol {}, found: {}", id, other_id));
                }
                if pattern.degree() != target.degree() {
                    return Err(format!(
                        "Argument size missmatch: {} {}",
                        pattern.degree(),
                        target.degree()
                    ));
                }

                for (x, y) in pattern.iter().zip(target.iter()) {
                    params = params_map_impl(y, x, params)?;
                }
            }
            _ => {
                return Err(format!(
                    "Expect symbol id: {}, found target: {:?}",
                    id, &target.data
                ));
            }
        },
        Term::Param(id) => {
            if params.contains_key(id) {
                let node = params.get(id).unwrap();
                let _ = params_map(node, target)?;
            } else {
                params.insert(*id, target.to_owned()); // subtree_clone(target));
            }
        }
        Term::Number(value) => match &target.data {
            Term::Number(other_value) => {
                if value != other_value {
                    return Err(format!("Expect Number {}, found {:?}", value, target.data));
                }

                return Ok(params);
            }
            _ => {
                return Err(format!("Expect Number, found: {:?}", target.data));
            }
        },
        Term::Variable(value) => match &target.data {
            Term::Variable(other_value) => {
                if value != other_value {
                    return Err(format!("Expect Varible {}, found {:?}", value, target.data));
                }

                return Ok(params);
            }
            _ => {
                return Err(format!("Expect Varible, found: {:?}", target.data));
            }
        },
    }
    Ok(params)
}

#[cfg(test)]
mod tree_utils_tests {
    use super::*;
    use core::trees::linked::fully::tr;

    #[test]
    fn simple_param_map_test() {
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Symbol(3)) / tr(Term::Symbol(4))) /
            tr(Term::Variable(5));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Symbol(4))) /
            tr(Term::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(*map.get(&1).unwrap(), (tr(Term::Symbol(3))));
                assert_eq!(*map.get(&2).unwrap(), (tr(Term::Variable(5))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn same_param_map_test() {
        let tree = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Symbol(3)) / tr(Term::Symbol(4))) /
            tr(Term::Symbol(3));
        let pattern = tr(Term::Symbol(1)) /
            (tr(Term::Symbol(2)) / tr(Term::Param(1)) / tr(Term::Symbol(4))) /
            tr(Term::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(*map.get(&1).unwrap(), (tr(Term::Symbol(3))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn subtree_param_map_test() {
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
                assert_eq!(
                    *map.get(&1).unwrap(),
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
                let mut test = tr(Term::Symbol(1)) / tr(Term::Param(1));
                apply_map(&mut test, &map);

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
