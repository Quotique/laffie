use std::collections::HashMap;
use std::fs;
use std::io;
use std::mem;
use std::path::Path;
use std::sync::{Arc, Mutex, Once, ONCE_INIT};

use parser::lang;

extern crate log;

#[derive(Clone)]
pub struct Symbol {
    id: u64,
    name: String,
}

struct SymbolsImpl {
    pub symbol_by_id: HashMap<u64, Symbol>,
    pub id_by_name: HashMap<String, u64>,
    pub last_id: u64,
}

#[derive(Clone)]
pub struct Symbols {
    inner: Arc<Mutex<SymbolsImpl>>,
}

pub fn all_symbols() -> Symbols {
    // Initialize it to a null value
    static mut SINGLETON: *const Symbols = 0 as *const Symbols;
    static ONCE: Once = ONCE_INIT;

    unsafe {
        ONCE.call_once(|| {
            let singleton = Symbols {
                inner: Arc::new(Mutex::new(SymbolsImpl {
                    symbol_by_id: HashMap::new(),
                    id_by_name: HashMap::new(),
                    last_id: 0,
                })),
            };

            // Put it in the heap so it can outlive this call
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}

impl Symbols {
    pub fn id_by_name(&self, name: &String) -> Option<u64> {
        let im = self.inner.lock().unwrap();
        let id = im.id_by_name.get(name);
        match id {
            Some(&x) => Some(x),
            None => None,
        }
    }

    pub fn name_by_id(&self, id: u64) -> Option<String> {
        let im = self.inner.lock().unwrap();
        let sym = im.symbol_by_id.get(&id);
        match sym {
            Some(x) => Some(x.name.clone()),
            None => None,
        }
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            panic!(dir
                .to_string_lossy()
                .to_string()
                .push_str("  is not directory!"));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.load_dir(&path)?;
            } else if path.extension().unwrap() == "sym" {
                self.load_file(&path)?;
            }
        }
        Ok(())
    }

    fn load_file(&mut self, file: &Path) -> io::Result<()> {
        info!("Processing file: {}", file.to_string_lossy());
        let content = fs::read_to_string(file)?;
        let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
        for s in states {
            if s.label == "Declare" && s.childs.len() > 1 && s.childs[1].label == "Symbol" {
                let mut s = Symbol {
                    id: 0,
                    name: s.childs[0].label.clone(),
                };
                self.add_symbol(s);
            }
        }

        Ok(())
    }

    fn add_symbol(&mut self, mut sym: Symbol) {
        trace!("Symbol adding: {}", sym.name);
        let mut im = self.inner.lock().unwrap();
        if im.id_by_name.contains_key(&sym.name) {
            trace!("Duplicate symbol: {}. Skipping", sym.name);
            return;
        }
        sym.id = im.last_id + 1;
        im.id_by_name.insert(sym.name.clone(), sym.id);
        im.symbol_by_id.insert(sym.id, sym);
        im.last_id += 1;
    }
}
