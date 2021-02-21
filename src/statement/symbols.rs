use std::{collections::HashMap, fmt};

use multi_map::MultiMap;
use parking_lot::RwLock;

lazy_static! {
    static ref ALL_SYMBOLS: RwLock<MultiMap<u64, String, Symbol>> = RwLock::new(MultiMap::new());
    static ref LAST_ID: RwLock<u64> = RwLock::new(0);
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SymbolAttr {
    Infix,
    Display,
    Associative,
    Commutative,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolAttrValue {
    None,
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
    ALL_SYMBOLS.read().get(&id).cloned()
}

pub fn symbol_by_name(name: &str) -> Option<Symbol> {
    ALL_SYMBOLS.read().get_alt(&name.to_owned()).cloned()
}

pub fn add_symbol(mut symbol: Symbol) -> Symbol {
    if let Some(s) = ALL_SYMBOLS.write().get_mut_alt(&symbol.name) {
        s.attrs.extend(symbol.attrs.into_iter());
        return s.clone();
    }
    *LAST_ID.write() += 1;
    symbol.id = *LAST_ID.read();
    ALL_SYMBOLS
        .write()
        .insert(symbol.id, symbol.name.clone(), symbol.clone());
    symbol
}

impl Symbol {
    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.attrs.get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    #[allow(dead_code)]
    pub fn add_with_name(name: &str) {
        add_symbol(Symbol {
            id:    0,
            name:  String::from(name),
            attrs: HashMap::new(),
        });
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(SymbolAttrValue::UStr(s)) = self.attrs.get(&SymbolAttr::Display) {
            return write!(f, "{}", s);
        }
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            Symbol::add_with_name("=="); // 1

            let mut attr = HashMap::new();
            attr.insert(SymbolAttr::Infix, SymbolAttrValue::UInt(10));
            attr.insert(SymbolAttr::Associative, SymbolAttrValue::None);
            attr.insert(SymbolAttr::Commutative, SymbolAttrValue::None);

            add_symbol(Symbol {
                id:    0,
                name:  "+".into(), // 2
                attrs: attr,
            });
            Symbol::add_with_name("-"); // 3
            Symbol::add_with_name("!="); // 4
            Symbol::add_with_name(">"); // 5
            Symbol::add_with_name("<"); // 6
            Symbol::add_with_name("*"); // 7
            Symbol::add_with_name("/"); // 8
            Symbol::add_with_name("<="); // 9
            Symbol::add_with_name(">="); // 10
            Symbol::add_with_name("^"); // 11
            Symbol::add_with_name("is"); // 12
            Symbol::add_with_name("known"); // 13
            Symbol::add_with_name("in"); // 14
        });
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
