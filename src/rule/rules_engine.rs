use super::rule::Rule;
use crate::statement::{symbols::symbol_by_name, MarkedStatement};
use std::{collections::HashMap, iter::once, sync::Arc};

use parking_lot::RwLock;

// TODO: move to correct place
type SymbolId = u64;
type Level = usize;

pub type SharedRule = Arc<RwLock<Rule>>;
type LevelRules = HashMap<SymbolId, Vec<SharedRule>>;
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

    pub fn register_raw_rule(&mut self, rule: Rule) {
        self.register_rule(Arc::new(RwLock::new(rule)))
    }

    pub fn register_rule(&mut self, rule: SharedRule) {
        self.last_id += 1;
        let (level, symbol_id) = {
            let rule = rule.read();
            (rule.level, rule.symbol_id)
        };
        rule.write().id = self.last_id as usize;
        self.all_rules
            .entry(level)
            .or_insert_with(LevelRules::new)
            .entry(symbol_id)
            .or_insert_with(Vec::new)
            .push(rule);
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
        once(&symbol_by_name("AnySymbol").unwrap().id)
            .chain(statement.symbols.iter())
            .flat_map(|symbol_id| level.get(symbol_id).clone().into_iter())
            .flat_map(|i| i.iter())
            .filter(|rule| {
                let rule = rule.read();
                rule.is_statement_suitable(&statement).is_ok() &&
                    rule.is_target_suitable(&target).is_ok()
            })
            .cloned()
            .collect()
    }
}
