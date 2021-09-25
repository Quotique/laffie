use std::{collections::HashMap, fmt, sync::Arc};

use derive_builder::Builder;
use multi_map::MultiMap;
use parking_lot::RwLock;

use state_macro::FuncAttr;

use super::term::StatementNode;

#[derive(FuncAttr)]
pub struct Ordering(Box<dyn Fn(&StatementNode, &StatementNode) -> std::cmp::Ordering>);
#[derive(FuncAttr)]
pub struct Calculator(Box<dyn Fn(&mut StatementNode) -> bool>);
#[derive(FuncAttr)]
pub struct TruthChecker(Box<dyn Fn(&StatementNode) -> bool>);

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

#[derive(Builder, Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    #[builder(default = "0")]
    pub id:            u64,
    #[builder(setter(into))]
    pub name:          String,
    #[builder(default)]
    pub attrs:         HashMap<SymbolAttr, SymbolAttrValue>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub arg_order:     Arc<Option<Ordering>>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub calculator:    Arc<Option<Calculator>>,
    #[builder(default = "Arc::new(None)", setter(into))]
    pub truth_checker: Arc<Option<TruthChecker>>,
}

impl SymbolBuilder {
    pub fn with_attr(&mut self, name: SymbolAttr, value: SymbolAttrValue) -> &mut Self {
        if self.attrs.is_none() {
            self.attrs = Some(HashMap::default());
        }
        self.attrs.as_mut().unwrap().insert(name, value);

        self
    }

    pub fn with_calculator(
        &mut self,
        calculator: Box<dyn Fn(&mut StatementNode) -> bool>,
    ) -> &mut Self {
        self.calculator = Some(Arc::new(Some(Calculator(calculator))));
        self
    }

    pub fn with_truth_checker(
        &mut self,
        truth_checker: Box<dyn Fn(&StatementNode) -> bool>,
    ) -> &mut Self {
        self.truth_checker = Some(Arc::new(Some(TruthChecker(truth_checker))));
        self
    }

    pub fn with_ordering(
        &mut self,
        ordering: Box<dyn Fn(&StatementNode, &StatementNode) -> std::cmp::Ordering>,
    ) -> &mut Self {
        self.arg_order = Some(Arc::new(Some(Ordering(ordering))));
        self
    }
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
    pub fn builder() -> SymbolBuilder {
        SymbolBuilder::default()
    }

    pub fn display_weight(&self) -> Option<u64> {
        if let Some(SymbolAttrValue::UInt(v)) = self.attrs.get(&SymbolAttr::Infix) {
            return Some(*v);
        }
        None
    }

    pub fn add_with_name(name: &str) {
        add_symbol(Symbol::builder().name(name).build().unwrap());
    }

    pub fn check_truth(&self, node: &StatementNode) -> bool {
        if let Some(c) = self.truth_checker.as_ref() {
            c.0(node)
        } else {
            false
        }
    }

    pub fn evaluate(&self, node: &mut StatementNode) -> bool {
        if let Some(c) = self.calculator.as_ref() {
            c.0(node)
        } else {
            false
        }
    }

    pub fn arg_order(
        &self,
        left: &StatementNode,
        right: &StatementNode,
    ) -> Option<std::cmp::Ordering> {
        self.arg_order.as_ref().as_ref().map(|o| o.0(left, right))
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

            add_symbol(
                Symbol::builder()
                    .name("+")
                    .with_attr(SymbolAttr::Infix, SymbolAttrValue::UInt(10))
                    .with_attr(SymbolAttr::Associative, SymbolAttrValue::None)
                    .with_attr(SymbolAttr::Commutative, SymbolAttrValue::None)
                    .build()
                    .unwrap(),
            );
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
            Symbol::builder()
                .id(1)
                .name("==")
                .with_truth_checker(Box::new(|_| false))
                .build()
                .unwrap()
        )
    }

    #[test]
    fn by_name_test() {
        setup();

        let sym = symbol_by_name(&String::from("==")).unwrap();
        assert_eq!(
            sym,
            Symbol::builder()
                .id(1)
                .name("==")
                .with_truth_checker(Box::new(|_| false))
                .build()
                .unwrap()
        )
    }
}
