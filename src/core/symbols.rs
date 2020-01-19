use std::{io, path::Path, sync::Mutex};

use super::{dir_parser::load_dir, multi_map::MultiMap, trees::Tree};

lazy_static! {
    static ref ALL_SYMBOLS: Mutex<MultiMap<u64, String, Symbol>> = Mutex::new(MultiMap::new());
    static ref LAST_ID: Mutex<u64> = Mutex::new(0);
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub id:   u64,
    pub name: String,
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
    ALL_SYMBOLS
        .lock()
        .expect("Unable to lock symbols")
        .insert(symbol.id, symbol.name.clone(), symbol);
}

pub fn load_symbols(dir: &Path) -> io::Result<()> {
    load_dir(dir, &mut |s: &Tree<String>| {
        if s.root().data == "Declare" && s.degree() > 1 && s.last().unwrap().data == "Symbol" {
            let s = Symbol {
                id:   0,
                name: s.first().unwrap().data.clone(),
            };
            add_symbol(s);
        }
    })
}
