use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use parser::lang;

use super::rule::Rule;
use super::symbols::all_symbols;

extern crate log;

pub struct RulesEngine {
    rules_by_sym: HashMap<u64, Vec<Rule>>,
}

impl RulesEngine {
    pub fn new() -> RulesEngine {
        RulesEngine {
            rules_by_sym: HashMap::new(),
        }
    }

    pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if !dir.is_dir() {
            panic!(dir
                .to_string_lossy()
                .to_string()
                .push_str(" is not directory!"));
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
        let mut symbol_id: u64 = 0;
        for s in states {
            if s.label == "Declare" && s.childs.len() > 1 && s.childs[1].label == "Symbol" {
                symbol_id = all_symbols().id_by_name(&s.childs[0].label).unwrap_or(0);
                if !self.rules_by_sym.contains_key(&symbol_id) {
                    self.rules_by_sym.insert(symbol_id, Vec::new());
                }
            } else {
                let rule = Rule::new(&s);
                trace!("Processing: {:?}", s);
                match rule {
                    Some(r) => self.rules_by_sym.get_mut(&symbol_id).unwrap().push(r),
                    None => trace!("Not rule!"), 
                }
            }
        }

        Ok(())
    }
}
