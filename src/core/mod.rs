pub mod node;
pub mod rule;
pub mod rules_engine;
pub mod symbols;

use self::node::{Node, NodeType};
use self::rule::Rule;
use std::io::{self, Write};

#[test]
fn node() {
    let state = Node {
        node_type: NodeType::Symbol,
        id: 1,
        childs: vec![Box::new(Node {
            node_type: NodeType::Symbol,
            id: 1,
            childs: vec![Box::new(Node {
                node_type: NodeType::Symbol,
                id: 2,
                childs: vec![],
            })],
        })],
    };
    let rule = Node {
        node_type: NodeType::Symbol,
        id: 1,
        childs: vec![Box::new(Node {
            node_type: NodeType::Symbol,
            id: 1,
            childs: vec![Box::new(Node {
                node_type: NodeType::Param,
                id: 1,
                childs: vec![],
            })],
        })],
    };
    state.visual().unwrap();
    rule.visual().unwrap();
    let stdout = io::stdout();
    match rule.map(&state) {
        Some(res) => {
            for (k, v) in res.iter() {
                stdout
                    .lock()
                    .write(format!("{} = {}\n", k.to_string(), v.to_string()).as_bytes())
                    .unwrap();
            }
        }
        None => {
            stdout
                .lock()
                .write(format!("Not mapped\n").as_bytes())
                .unwrap();
        }
    }
}

#[test]
fn rule() {
    let rule = Rule {
        pattern: Node {
            node_type: NodeType::Symbol,
            id: 1,
            childs: vec![Box::new(Node {
                node_type: NodeType::Symbol,
                id: 1,
                childs: vec![Box::new(Node {
                    node_type: NodeType::Param,
                    id: 1,
                    childs: vec![],
                })],
            })],
        },
        replace: Node {
            node_type: NodeType::Param,
            id: 1,
            childs: vec![],
        },
    };
    let mut source = Node {
        node_type: NodeType::Symbol,
        id: 1,
        childs: vec![Box::new(Node {
            node_type: NodeType::Symbol,
            id: 1,
            childs: vec![Box::new(Node {
                node_type: NodeType::Symbol,
                id: 2,
                childs: vec![],
            })],
        })],
    };
    let dest = rule.apply(&source);
    //let stdout = io::stdout();
    source.visual();
    dest.unwrap().visual();
}
