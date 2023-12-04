use std::{
    collections::{HashMap, VecDeque},
    iter::once,
    sync::Arc,
};

use itertools::Itertools;

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
    all_rules:  HashMap<Level, LevelRules>,
    rule_queue: VecDeque<Rule>,
    id_map:     HashMap<CompactString, RuleId>,
    last_id:    RuleId,
}

unsafe impl Send for RulesEngine {}
unsafe impl Sync for RulesEngine {}

impl RulesEngine {
    pub fn register_rule(&mut self, mut rule: Rule) {
        self.last_id.increment();
        rule.id = self.last_id;
        debug!("New rule: {}", rule);
        if let Some(RuleAttrValue::Str(s)) = rule.attribute(&RuleAttr::Id).next() {
            self.id_map.insert(s.clone(), rule.id);
        }
        self.add_rule(rule);
        self.process_queue();
    }

    pub fn suggest_rules(
        &self,
        statement: &MarkedStatement,
        target: &MarkedStatement,
    ) -> Vec<SharedRule> {
        assert!(self.rule_queue.is_empty());

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
                "[{}] Suggested rules: [{}]",
                statement.weight,
                VecDisplay(&result)
            );
        }
        result
    }

    fn process_queue(&mut self) {
        let mut queue_len = self.rule_queue.len();

        while let Some(rule) = self.rule_queue.pop_front() {
            self.add_rule(rule);
            queue_len -= 1;
            if queue_len == 0 {
                break;
            }
        }
    }

    fn add_rule(&mut self, mut rule: Rule) {
        if !rule
            .attribute(&RuleAttr::Block)
            .filter_map(RuleAttrValue::str)
            .all(|x| self.id_map.contains_key(x))
        {
            self.rule_queue.push_back(rule);
        } else {
            rule.block = rule
                .attribute(&RuleAttr::Block)
                .filter_map(RuleAttrValue::str)
                .map(|x| *self.id_map.get(x).unwrap())
                .unique()
                .collect();
            self.all_rules
                .entry(rule.level)
                .or_default()
                .entry(rule.symbol_id)
                .or_default()
                .push(Arc::new(rule));
        }
    }
}
