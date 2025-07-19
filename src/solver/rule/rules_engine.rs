use std::{
    collections::{HashMap, VecDeque},
    iter::once,
};

use itertools::Itertools;

use utils::VecDisplay;

use super::{Rule, RuleAttr, RuleAttrValue, RuleId, SharedRule, TermFilters};
use crate::{
    term::{Symbol, Term},
    CompactString,
};

// TODO: move to correct place
type Level = usize;

type LevelRules = HashMap<Symbol, Vec<SharedRule>>;

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
    pub fn iter(&self) -> impl Iterator<Item = SharedRule> + '_ {
        self.all_rules
            .values()
            .flat_map(|x| x.values().flat_map(|x| x.iter().cloned()))
    }

    pub fn register_rule(&mut self, mut rule: Rule) {
        self.last_id.increment();
        rule.id = self.last_id;
        debug!("New rule: {rule}");
        if let Some(RuleAttrValue::Str(s)) = rule.attribute(&RuleAttr::Id).next() {
            self.id_map.insert(s.clone(), rule.id);
        }
        self.add_rule(rule);
        self.process_queue();
    }

    pub fn suggest_rules(&self, filters: &TermFilters, purpose: &Term) -> Vec<SharedRule> {
        assert!(self.rule_queue.is_empty());

        let empty_level = LevelRules::new();
        let level = self.all_rules.get(&filters.weight).unwrap_or(&empty_level);
        let result: Vec<_> = once(&Symbol::by_name("AnySymbol").unwrap())
            .chain(filters.func_symbols.iter())
            .flat_map(|symbol| level.get(symbol).into_iter())
            .flat_map(|i| i.iter())
            .filter(|rule| rule.try_filter(filters, purpose).is_ok())
            .cloned()
            .collect();
        if !result.is_empty() {
            trace!(
                "[{}] Suggested rules: [{}]",
                filters.weight,
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
                .entry(rule.func_symbol.clone())
                .or_default()
                .push(SharedRule::new(rule));
        }
    }
}
