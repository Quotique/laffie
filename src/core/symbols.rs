use std::{fs, io, path::Path, sync::Mutex};

use super::multi_map::MultiMap;

use parser::lang;

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
    ALL_SYMBOLS.lock().expect("Unable to lock symbols").get(&id).map(|x| x.clone())
}

pub fn symbol_by_name(name: &String) -> Option<Symbol> {
    ALL_SYMBOLS.lock().expect("Unable to lock symbols").get_alt(name).map(|x| x.clone())
}

pub fn add_symbol(mut symbol: Symbol) {
    if ALL_SYMBOLS.lock().expect("Unable to lock symbols").contains_key_alt(&symbol.name) {
        trace!("Duplicate symbol: {}. Skipping", symbol.name);
        return;
    }
    *LAST_ID.lock().expect("Unable to lock symbols") += 1;
    symbol.id = *LAST_ID.lock().expect("Unable to lock symbols");
    ALL_SYMBOLS.lock().expect("Unable to lock symbols").insert(symbol.id, symbol.name.clone(), symbol);
}

pub fn load_dir(dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        panic!(dir.to_string_lossy().to_string().push_str("  is not directory!"));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            load_dir(&path)?;
        } else if path.extension().unwrap() == "sym" {
            load_file(&path)?;
        }
    }
    Ok(())
}

fn load_file(file: &Path) -> io::Result<()> {
    info!("Processing file: {}", file.to_string_lossy());
    let content = fs::read_to_string(file)?;
    let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
    for s in states {
        if s.root().data == "Declare" && s.degree() > 1 && s.last().unwrap().data == "Symbol" {
            let mut s = Symbol {
                id:   0,
                name: s.first().unwrap().data.clone(),
            };
            add_symbol(s);
        }
    }

    Ok(())
}
