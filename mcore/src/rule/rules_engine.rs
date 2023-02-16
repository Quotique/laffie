use std::{
    collections::{HashMap, HashSet},
    iter::once,
    sync::Arc,
};

use multimap::MultiMap;

use crate::{
    predefine::symbol_by_name, statement::MarkedStatement, utils::VecDisplay, CompactString,
    RuleId, SymbolId,
};

use super::rule::{Rule, RuleAttr, RuleAttrValue};

// TODO: move to correct place
type Level = usize;

pub type SharedRule = Arc<Rule>;
type LevelRules = HashMap<SymbolId, Vec<SharedRule>>;

#[derive(Default)]
pub struct RulesEngine {
    all_rules:   HashMap<Level, LevelRules>,
    id_requires: MultiMap<CompactString, SharedRule>,
    id_map:      HashMap<CompactString, RuleId>,
    last_id:     RuleId,
}

unsafe impl Send for RulesEngine {}
unsafe impl Sync for RulesEngine {}

impl RulesEngine {
    fn update_rule_blocklist(&mut self, mut s_rule: SharedRule) {
        let mut blocklist: HashSet<RuleId> = Default::default();

        // TODO: iter?
        for i in s_rule.attribute(&RuleAttr::Block) {
            if let RuleAttrValue::Str(s) = i {
                if let Some(id) = self.id_map.get(s) {
                    blocklist.insert(*id);
                } else {
                    self.id_requires.insert(s.clone(), s_rule.clone());
                }
            }
        }
        let rule = Arc::make_mut(&mut s_rule);
        rule.block = blocklist.into_iter().collect();
    }

    fn register_id(&mut self, rule: SharedRule) {
        if let Some(RuleAttrValue::Str(s)) = rule.attribute(&RuleAttr::Id).next() {
            self.id_map.insert(s.clone(), rule.id);
            if let Some(reqs) = self.id_requires.remove(s) {
                for r in reqs.into_iter() {
                    self.update_rule_blocklist(r);
                }
            }
        };
    }

    pub fn register_raw_rule(&mut self, rule: Rule) {
        self.register_rule(Arc::new(rule))
    }

    pub fn register_rule(&mut self, mut rule: SharedRule) {
        self.last_id.increment();
        Arc::make_mut(&mut rule).id = self.last_id;
        self.update_rule_blocklist(rule.clone());
        self.register_id(rule.clone());
        self.all_rules
            .entry(rule.level)
            .or_insert_with(LevelRules::new)
            .entry(rule.symbol_id)
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
        let result: Vec<_> = once(&symbol_by_name("AnySymbol").unwrap().id)
            .chain(statement.symbols.iter())
            .flat_map(|symbol_id| level.get(symbol_id).into_iter())
            .flat_map(|i| i.iter())
            .filter(|rule| {
                rule.is_statement_suitable(statement).map_err(|e| {
                    trace!(target: "rule_selection", "Rule {} rejected for statement {} by reason {:?}", rule, statement, e);
                }).is_ok() &&
                    rule.is_target_suitable(target).map_err(|e| {
                    trace!(target: "rule_selection", "Rule {} rejected for statement {} by reason {:?}", rule, statement, e);
                }).is_ok()
            })
            .cloned()
            .collect();
        if !result.is_empty() {
            trace!(
                target: "rule_selection", "[{}] Suggested rules: [{}]",
                statement.weight, VecDisplay(&result)
            );
        }
        result
    }
}
