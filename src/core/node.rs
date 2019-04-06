use std::collections::HashMap;
use std::io::{self, Write};
use std::fmt;

#[derive(Clone)]
pub enum NodeType {
    Symbol,
    Param,
    Varible,
}

#[derive(Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub id: u64,
    pub childs: Vec<Box<Node>>,
}

type ParamsMap = HashMap<u64, Node>;

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	match self {
	    NodeType::Symbol => write!(f, "s"),
	    NodeType::Param => write!(f, "p"),
	    NodeType::Varible =>write!(f, "v"),
	}
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.node_type.to_string(), self.id.to_string())
    }
}

impl Node {
    fn sort_childs(&mut self) {
	// TODO: effective insert
	self.childs.sort_by(|a, b| a.id.cmp(&b.id));
    }

    pub fn map(&self, target: &Node) -> Option<ParamsMap> {
	let mut params_mapping = ParamsMap::new();
	if self.map_impl(target, &mut params_mapping) {
	    return Option::from(params_mapping);
	} else {
	    return None;
	}
    }

    fn map_impl(&self, other: &Node, mapping: &mut ParamsMap) -> bool {
	match self.node_type {
	    NodeType::Symbol => {
		if self.id != other.id || self.childs.len() != other.childs.len() {
		    return false;
		}
		for i in 0..self.childs.len() {
		    if self.childs[i].map_impl(&other.childs[i], mapping) {
			return false;
		    }
		}
		return true;
	    }
	    NodeType::Param => {
		if mapping.contains_key(&self.id) {
		    let node = mapping.get(&self.id);
		    return self.map(node.unwrap()).is_some();
		} else {
		    mapping.insert(self.id, other.clone());
		}
		return true;
	    }
	    NodeType::Varible => return false,
	}
    }

    pub fn visual(&self) -> io::Result<()> {
	let mut ident = String::from("");
	self.visual_ident(&mut ident)
    }

    fn visual_ident(&self, ident: &mut String) -> io::Result<()> {
	let ident_len = ident.len();
	let stdout = io::stdout();
	stdout.lock().write(b"\"")?;
	stdout.lock().write(self.to_string().as_bytes())?;
	stdout.lock().write(b"\"\n")?;
	for (i, item) in self.childs.iter().enumerate() {
	    ident.truncate(ident_len);
	    stdout.lock().write(ident.as_bytes())?;
	    if i != self.childs.len() - 1 {
		ident.push_str("\u{2502} ");
		stdout
		    .lock()
		    .write(String::from("\u{251C}\u{2500}").as_bytes())?;
	    } else {
		ident.push_str("  ");
		stdout
		    .lock()
		    .write(String::from("\u{2514}\u{2500}").as_bytes())?;
	    }
	    item.visual_ident(ident)?;
	}
	ident.truncate(ident_len);
	Ok(())
    }
}
