use std::collections::HashMap;

use super::trees::{tr, Node, Tree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeData {
    Symbol(u64),
    Param(u64),
    Varible(u64),
}

type ParamsNameMap = HashMap<String, u64>;
type ParamsMap = HashMap<u64, Tree<NodeData>>;
type TreeNode = Node<NodeData>;
type ParserTree = Tree<String>;
type ParserNode = Node<String>;

type DataTree = Tree<NodeData>;

pub fn parse_node(
    src_node: &ParserNode,
    params: &mut ParamsNameMap,
    last_param_id: &mut u64,
) -> Result<DataTree, String> {
    let mut result = match super::symbols::symbol_by_name(&src_node.data) {
        Some(symbol) => tr(NodeData::Symbol(symbol.id)),
        None => {
            if !src_node.is_leaf() {
                return Err(format!("Node type Param({}) can't contains childs!", &src_node.data));
            }
            tr(NodeData::Param(if params.contains_key(&src_node.data) {
                *params.get(&src_node.data).unwrap()
            } else {
                *last_param_id += 1;
                params.insert(src_node.data.clone(), *last_param_id);
                *last_param_id
            }))
        }
    };
    for child in src_node.iter() {
        result.push_back(parse_node(&child, params, last_param_id)?);
    }
    Ok(result)
}

pub fn apply_map(target: &mut TreeNode, params: &ParamsMap) {
    match target.data {
        NodeData::Param(id) => {
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

fn params_map_impl(target: &TreeNode, pattern: &TreeNode, mut params: ParamsMap) -> Result<ParamsMap, String> {
    println!("Pattern: {:?}", pattern);
    match pattern.data {
        NodeData::Symbol(id) => match &target.data {
            &NodeData::Symbol(other_id) => {
                if id != other_id {
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
                return Err(format!("Expect symbol id: {}, found target: {:?}", id, &target.data));
            }
        },
        NodeData::Param(id) => {
            println!("Id: {}", id);
            if params.contains_key(&id) {
                let node = params.get(&id).unwrap();
                println!("Node: {:?}", node);
                let _ = params_map(node, target)?;
            } else {
                params.insert(id, target.to_owned()); // subtree_clone(target));
            }
        }
        NodeData::Varible(id) => match &target.data {
            &NodeData::Varible(other_id) => {
                if id != other_id {
                    return Err(format!("Expect Varible {}, found {:?}", id, target.data));
                }

                return Ok(params);
            }
            _ => {
                return Err(format!("Expect varible, found: {:?}", target.data));
            }
        },
    }
    Ok(params)
}

// fn subtree_clone(src: &TreeNode) -> Tree<NodeData> {
//    let mut result = tr(src.data.clone());
//    result.append(src.forest().clone());
//    result
//}

#[cfg(test)]
mod tree_utils_tests {
    use super::*;
    use core::trees::linked::fully::tr;

    #[test]
    fn simple_param_map_test() {
        let tree = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Varible(5));
        let pattern = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Param(1)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(*map.get(&1).unwrap(), (tr(NodeData::Symbol(3))));
                assert_eq!(*map.get(&2).unwrap(), (tr(NodeData::Varible(5))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn same_param_map_test() {
        let tree = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Symbol(3));
        let pattern = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Param(1)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(*map.get(&1).unwrap(), (tr(NodeData::Symbol(3))));
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn subtree_param_map_test() {
        let tree = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) /
                (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2))) /
                tr(NodeData::Symbol(4))) /
            (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2)));
        let pattern = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Param(1)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                assert_eq!(
                    *map.get(&1).unwrap(),
                    (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2)))
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
        let tree = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) /
                (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2))) /
                tr(NodeData::Symbol(4))) /
            (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2)));
        let pattern = tr(NodeData::Symbol(1)) /
            (tr(NodeData::Symbol(2)) / tr(NodeData::Param(1)) / tr(NodeData::Symbol(4))) /
            tr(NodeData::Param(2));
        match params_map(&tree, &pattern) {
            Ok(map) => {
                let mut test = tr(NodeData::Symbol(1)) / tr(NodeData::Param(1));
                apply_map(&mut test, &map);

                assert_eq!(
                    test,
                    tr(NodeData::Symbol(1)) /
                        (tr(NodeData::Symbol(3)) / tr(NodeData::Symbol(1)) / tr(NodeData::Symbol(2)))
                );
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }
}
