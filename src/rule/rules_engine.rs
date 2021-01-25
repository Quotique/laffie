use super::rule::{Rule, RuleAttr, RuleAttrValue};
use crate::{
    core::symbols::symbol_by_name, solver::problem::ProblemType, statement::MarkedStatement,
};
use std::{
    collections::HashMap,
    iter::once,
    sync::{Arc, RwLock},
};

// TODO: move to correct place
type SymbolId = u64;
type Level = usize;

pub type SharedRule = Arc<RwLock<Rule>>;
type LevelRules = HashMap<SymbolId, SharedRule>;
type RuleId = u64;

pub struct RulesEngine {
    all_rules: HashMap<Level, LevelRules>,
    last_id:   RuleId,
}

impl RulesEngine {
    pub fn new() -> RulesEngine {
        RulesEngine {
            all_rules: HashMap::new(),
            last_id:   0,
        }
    }

    pub fn register_rule(&mut self, rule: SharedRule) {
        self.last_id += 1;
        let (level, symbol_id) = {
            let rule = rule.read().expect("Can't lock rule");
            (rule.level.clone(), rule.symbol_id.clone())
        };
        self.all_rules
            .entry(level)
            .or_insert(LevelRules::new())
            .insert(symbol_id, rule);
    }

    pub fn suggest_rules(
        &self,
        statement: &MarkedStatement,
        target: &MarkedStatement,
    ) -> Vec<SharedRule> {
        let empty_level = LevelRules::new();
        let level = self
            .all_rules
            .get(&statement.weight)
            .unwrap_or(&empty_level);
        once(&symbol_by_name(&"AnySymbol".into()).unwrap().id)
            .chain(statement.symbols.iter())
            .flat_map(|symbol_id| level.get(symbol_id).clone().into_iter())
            .filter(|rule| {
                let rule = rule.read().expect("Can't lock rule");
                rule.is_statement_suitable(&statement) && rule.is_target_suitable(&target)
            })
            .cloned()
            .collect()
    }

    // pub fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
    //     if !dir.is_dir() {
    //         panic!(dir
    //             .to_string_lossy()
    //             .to_string()
    //             .push_str(" is not directory!"));
    //     }
    //     for entry in fs::read_dir(dir)? {
    //         let entry = entry?;
    //         let path = entry.path();
    //         if path.is_dir() {
    //             self.load_dir(&path)?;
    //         } else if path.extension().unwrap() == "sym" {
    //             self.load_file(&path)?;
    //         }
    //     }
    //     Ok(())
    // }
    //
    // fn load_file(&mut self, file: &Path) -> io::Result<()> {
    //     info!("Processing file: {}", file.to_string_lossy());
    //     let content = fs::read_to_string(file)?;
    //     let states = lang::StatementsParser::new().parse(&content[..]).unwrap();
    //     let mut symbol_id: u64 = 0;
    //     for state in states {
    //         let s = state.root();
    //         if let Ok(sym) = Symbol::try_from(&state) {
    //             symbol_id = sym.id;
    //             if !self.rules_by_sym.contains_key(&sym.id) {
    //                 self.rules_by_sym.insert(sym.id, Vec::new());
    //             }
    //         } else {
    //             trace!("Processing: {:?} ({})", s, symbol_id);
    //             self.last_rule_id += 1;
    //             match RuleBuilder::new()
    //                 .with_id(self.last_rule_id.clone())
    //                 .with_statement(&state)
    //             {
    //                 Ok(r) => {
    //                     trace!("New rule: {:?}", r);
    //                     self.rules_by_sym
    //                         .get_mut(&symbol_id)
    //                         .unwrap()
    //                         .push(Arc::new(RwLock::new(r)))
    //                 }
    //                 Err(e) => trace!("Not rule!: {}", e),
    //             }
    //         }
    //     }
    //
    //     Ok(())
    // }
}
