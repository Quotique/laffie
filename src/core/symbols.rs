use std::{collections::HashMap, convert::TryFrom, fmt, io, path::Path, str::FromStr, sync::Mutex};

use multi_map::MultiMap;
use trees::{Node, Tree};

use super::dir_parser::load_dir;
use crate::parser::Tree as ParserTree;

lazy_static! {
    static ref ALL_SYMBOLS: Mutex<MultiMap<u64, String, Symbol>> = Mutex::new(MultiMap::new());
    static ref LAST_ID: Mutex<u64> = Mutex::new(0);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SymbolAttr {
    Infix,
    Display,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolAttrValue {
    UInt(u64),
    UStr(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub id:    u64,
    pub name:  String,
    pub attrs: HashMap<SymbolAttr, SymbolAttrValue>,
}

pub fn symbol_by_id(id: u64) -> Option<Symbol> {
    ALL_SYMBOLS
        .lock()
        .expect("Unable to lock symbols")
        .get(&id)
        .map(|x| x.clone())
}

pub fn symbol_by_name(name: &String) -> Option<Symbol> {
    ALL_SYMBOLS
        .lock()
        .expect("Unable to lock symbols")
        .get_alt(name)
        .map(|x| x.clone())
}

pub fn add_symbol(mut symbol: Symbol) {
    if ALL_SYMBOLS
        .lock()
        .expect("Unable to lock symbols")
        .contains_key_alt(&symbol.name)
    {
        trace!("Duplicate symbol: {}. Skipping", symbol.name);
        return;
    }
    *LAST_ID.lock().expect("Unable to lock symbols") += 1;
    symbol.id = *LAST_ID.lock().expect("Unable to lock symbols");
    ALL_SYMBOLS.lock().expect("Unable to lock symbols").insert(
        symbol.id,
        symbol.name.clone(),
        symbol,
    );
}

pub fn load_symbols(dir: &Path) -> io::Result<()> {
    load_dir(dir, &mut |s: &Tree<String>| {
        if let Ok(sym) = Symbol::try_from(s) {
            add_symbol(sym);
        }
    })
}

impl Symbol {
    pub fn display_weight(&self) -> Option<u64> {
        if let Some(attr) = self.attrs.get(&SymbolAttr::Infix) {
            if let SymbolAttrValue::UInt(v) = attr {
                return Some(*v);
            }
        }
        None
    }

    fn parse_attr(data: &Node<String>) -> Result<(SymbolAttr, SymbolAttrValue), String> {
        match data.data.as_str() {
            "infix" => {
                let c = data
                    .first()
                    .ok_or(String::from("infix(w) argument is required!"))?;
                let w = u64::from_str(&c.data)
                    .map_err(|_| String::from("Infix argument must be u64"))?;
                Ok((SymbolAttr::Infix, SymbolAttrValue::UInt(w)))
            }
            "display" => {
                let s = data
                    .first()
                    .ok_or(String::from("display(s) argument is required!"))?
                    .data
                    .clone();
                Ok((SymbolAttr::Display, SymbolAttrValue::UStr(s)))
            }
            _ => Err(format!("Unknown symbol attribute: {}", data.data)),
        }
    }

    #[allow(dead_code)]
    fn add_with_name(name: &str) {
        add_symbol(Symbol {
            id:    0,
            name:  String::from(name),
            attrs: HashMap::new(),
        });
    }
}

impl TryFrom<&ParserTree> for Symbol {
    type Error = String;

    fn try_from(data: &ParserTree) -> Result<Self, String> {
        if data.root().data == "Declare" {
            let mut symbol = Symbol {
                id:    0,
                name:  String::default(),
                attrs: HashMap::new(),
            };

            for sym_child in data.iter() {
                if sym_child.data == "Symbol" {
                    symbol.name = sym_child.first().unwrap().data.clone();
                } else if sym_child.data == "Attrs" {
                    for attr in sym_child.iter() {
                        let a = Symbol::parse_attr(attr)
                            .expect(&format!("Bad symbol attribute: {:?}", attr));
                        symbol.attrs.insert(a.0, a.1);
                    }
                }
            }
            if let Some(s) = symbol_by_name(&symbol.name) {
                symbol.id = s.id;
            }
            return Ok(symbol);
        }
        Err("Not symbol tree".into())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(attr) = self.attrs.get(&SymbolAttr::Display) {
            if let SymbolAttrValue::UStr(s) = attr {
                return write!(f, "{}", s);
            }
        }
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
pub mod symbols_tests {
    use super::*;
    use crate::parser::lang;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            Symbol::add_with_name("==");
            Symbol::add_with_name("+");
            Symbol::add_with_name("-");
            Symbol::add_with_name("!=");
            Symbol::add_with_name(">");
            Symbol::add_with_name("<");
            Symbol::add_with_name("*");
        });
    }

    #[test]
    fn parser_test() {
        setup();

        let test_str = "symbol + { attr infix(10) }";
        let states = lang::StatementsParser::new().parse(test_str).unwrap();
        let sym = Symbol::try_from(&states[0]).unwrap();
        let mut expect_attr = HashMap::new();
        expect_attr.insert(SymbolAttr::Infix, SymbolAttrValue::UInt(10));
        assert_eq!(
            sym,
            Symbol {
                id:    2,
                name:  "+".into(),
                attrs: expect_attr,
            }
        );
    }

    #[test]
    fn by_id_test() {
        setup();

        let sym = symbol_by_id(1).unwrap();
        assert_eq!(
            sym,
            Symbol {
                id:    1,
                name:  "==".into(),
                attrs: HashMap::new(),
            }
        )
    }

    #[test]
    fn by_name_test() {
        setup();

        let sym = symbol_by_name(&String::from("==")).unwrap();
        assert_eq!(
            sym,
            Symbol {
                id:    1,
                name:  "==".into(),
                attrs: HashMap::new(),
            }
        )
    }
}
