pub use bigdecimal::{BigDecimal as Decimal, Signed};
use std::{collections::HashMap, fmt, hash::Hash, str::FromStr};
use trees::{tr, Node, Tree};

use super::symbols::{symbol_by_id, symbol_by_name, Symbol};

pub type StatementTree = Tree<Term>;
// type ParamsMap = HashMap<u64, StatementTree>;
pub type ParamsNameMap = HashMap<String, u64>;
type ParserNode = Node<String>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    Symbol(u64),
    Param(u64),
    Variable(u64),
    Number(Decimal),
}

#[derive(Clone)]
pub enum NodeType {
    Statement,
    Rule,
}

pub fn parse_statement_node(
    src_node: &ParserNode,
    params: &mut ParamsNameMap,
    last_param_id: &mut u64,
) -> Result<StatementTree, String> {
    parse_node(src_node, params, last_param_id, NodeType::Statement)
}

pub fn parse_rule_node(
    src_node: &ParserNode,
    params: &mut ParamsNameMap,
    last_param_id: &mut u64,
) -> Result<StatementTree, String> {
    parse_node(src_node, params, last_param_id, NodeType::Rule)
}

fn parse_node(
    src_node: &ParserNode,
    params: &mut ParamsNameMap,
    last_param_id: &mut u64,
    node_type: NodeType,
) -> Result<StatementTree, String> {
    let mut result = tr(Term::parse(
        src_node.data().clone(),
        params,
        last_param_id,
        &node_type,
    ));
    if result.root().data().is_symbol() {
        for child in src_node.iter() {
            result.push_back(parse_node(child, params, last_param_id, node_type.clone())?);
        }
    } else if !src_node.degree() == 0 {
        return Err(format!("Node {} can't contains childs!", &src_node.data()));
    }

    Ok(result)
}

impl Term {
    fn parse(
        data: String,
        params: &mut ParamsNameMap,
        last_param_id: &mut u64,
        node_type: &NodeType,
    ) -> Self {
        if let Ok(value) = Decimal::from_str(&data) {
            Term::Number(value)
        } else if let Some(symbol) = symbol_by_name(&data) {
            Term::Symbol(symbol.id)
        } else {
            let id = *params.entry(data).or_insert_with(|| {
                *last_param_id += 1;
                *last_param_id
            });

            match node_type {
                NodeType::Rule => Term::Param(id),
                NodeType::Statement => Term::Variable(id),
            }
        }
    }

    pub fn with_symbol_name(name: &str) -> Option<Self> {
        symbol_by_name(name).map(|s| Self::Symbol(s.id))
    }

    pub fn symbol(&self) -> Option<Symbol> {
        if let Term::Symbol(id) = self {
            return symbol_by_id(*id);
        }
        None
    }

    pub fn symbol_id(&self) -> Option<u64> {
        if let Term::Symbol(id) = self {
            return Some(*id);
        }
        None
    }

    #[allow(dead_code)]
    pub fn is_symbol(&self) -> bool {
        if let Term::Symbol(_) = &self {
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_param(&self) -> bool {
        if let Term::Param(_) = &self {
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_number(&self) -> bool {
        if let Term::Number(_) = &self {
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_variable(&self) -> bool {
        if let Term::Variable(_) = &self {
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_param_id(&self, id: u64) -> bool {
        if let Term::Param(s_id) = &self {
            return *s_id == id;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_symbol_id(&self, id: u64) -> bool {
        if let Term::Symbol(s_id) = &self {
            return *s_id == id;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_variable_id(&self, id: u64) -> bool {
        if let Term::Variable(s_id) = &self {
            return *s_id == id;
        }
        false
    }

    #[allow(dead_code)]
    pub fn is_symbol_name(&self, name: &str) -> bool {
        if let Some(s) = symbol_by_name(name) {
            return self.is_symbol_id(s.id);
        }
        false
    }

    pub fn is_number_value(&self, value: &Decimal) -> bool {
        if let Term::Number(num) = &self {
            return num == value;
        }
        false
    }
}

pub fn display_string(node: &Node<Term>) -> String {
    match node.data() {
        Term::Symbol(id) => {
            let symbol = symbol_by_id(*id).unwrap();
            match symbol.display_weight() {
                Some(weight) => {
                    if node.degree() < 2 {
                        return format!(
                            "{}{}",
                            symbol,
                            node.iter()
                                .map(|x| display_string(x))
                                .collect::<Vec::<String>>()
                                .join(", ")
                        );
                    }
                    node.iter()
                        .map(|x| {
                            if let Term::Symbol(id) = x.data() {
                                let symbol = symbol_by_id(*id).unwrap();
                                if let Some(other_weight) = symbol.display_weight() {
                                    if weight <= other_weight {
                                        return format!("({})", display_string(x));
                                    }
                                }
                            }
                            display_string(x)
                        })
                        .collect::<Vec<String>>()
                        .join(symbol.to_string().as_str())
                }
                None => {
                    // Prefix notation is default
                    format!(
                        "{}({})",
                        symbol,
                        node.iter()
                            .map(|x| display_string(x))
                            .collect::<Vec::<String>>()
                            .join(", ")
                    )
                }
            }
        }
        _ => node.data().to_string(),
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Term::Symbol(id) => {
                let s = symbol_by_id(*id).unwrap();
                write!(f, "{}", s.name)
            }
            Term::Param(id) => write!(f, "p{}", id),
            Term::Number(value) => {
                if value.is_negative() {
                    write!(f, "({})", value)
                } else {
                    write!(f, "{}", value)
                }
            }
            Term::Variable(id) => write!(f, "x{}", id),
        }
    }
}
