pub mod node;
use self::node::{Node, NodeType};
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
    state.visual();
    rule.visual();
    let stdout = io::stdout();
    match rule.map(&state) {
        Some(res) => {
            for (k, v) in res.iter() {
                stdout
                    .lock()
                    .write(format!("{} = {}\n", k.to_string(), v.to_string()).as_bytes());
            }
        }
        None => {
            stdout.lock().write(format!("Not mapped\n").as_bytes());
        }
    }
}
