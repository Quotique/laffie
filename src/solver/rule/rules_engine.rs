use std::{collections::HashMap, fmt, iter::once};

use itertools::Itertools;

use super::{Rule, RuleAttr, RuleAttrValue, RuleId, SharedRule, TermFilters, rule::Level};
use crate::{
    CompactString,
    term::{Symbol, TermBuf, sym},
};

type LevelRules = HashMap<Symbol, Vec<SharedRule>>;

/// A rule the base could not file. Returned instead of the base, so nothing
/// runs against a rule set that is not what the `.sym` files say.
#[derive(Debug)]
pub enum RuleRejection {
    UndefinedId {
        rule: Rule,
        ids:  Vec<CompactString>,
    },
}

#[derive(Default)]
pub struct RulesEngine {
    all_rules: HashMap<Level, LevelRules>,
    max_level: Level,
}

impl RulesEngine {
    pub fn max_level(&self) -> Level {
        self.max_level
    }

    pub fn iter(&self) -> impl Iterator<Item = SharedRule> + '_ {
        self.all_rules
            .values()
            .flat_map(|x| x.values().flat_map(|x| x.iter().cloned()))
    }

    pub fn suggest_rules(&self, filters: &TermFilters, goal: &TermBuf) -> Vec<SharedRule> {
        let empty_level = LevelRules::new();
        let level = self.all_rules.get(&filters.level).unwrap_or(&empty_level);
        let result: Vec<_> = once(&sym("AnySymbol"))
            .chain(filters.symbols.iter())
            .flat_map(|symbol| level.get(symbol).into_iter())
            .flat_map(|i| i.iter())
            .filter(|rule| rule.try_filter(filters, goal).is_ok())
            .cloned()
            .collect();
        if !result.is_empty() {
            trace!(
                "[{}] Suggested rules: [{}]",
                filters.level,
                result.iter().format(", ")
            );
        }
        result
    }
}

impl fmt::Display for RuleRejection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UndefinedId { rule, ids } => write!(
                f,
                "rule {rule} references undefined block(s): {}",
                ids.iter().format(", ")
            ),
        }
    }
}

impl FromIterator<Rule> for Result<RulesEngine, Vec<RuleRejection>> {
    fn from_iter<I: IntoIterator<Item = Rule>>(iter: I) -> Self {
        let mut rules: Vec<Rule> = iter.into_iter().collect();

        let mut id_map: HashMap<CompactString, RuleId> = HashMap::new();
        let mut last_id = RuleId::default();
        for rule in &mut rules {
            last_id.increment();
            rule.id = last_id;
            if let Some(RuleAttrValue::Str(s)) = rule.attribute(&RuleAttr::Id).next() {
                id_map.insert(s.clone(), rule.id);
            }
        }

        let mut engine = RulesEngine::default();
        let mut rejected = Vec::new();
        for mut rule in rules {
            let ids: Vec<CompactString> = rule
                .attribute(&RuleAttr::Block)
                .filter_map(RuleAttrValue::str)
                .filter(|x| !id_map.contains_key(*x))
                .map(CompactString::from)
                .collect();
            if !ids.is_empty() {
                rejected.push(RuleRejection::UndefinedId { rule, ids });
                continue;
            }
            debug!("New rule: {rule}");
            rule.block = rule
                .attribute(&RuleAttr::Block)
                .filter_map(RuleAttrValue::str)
                .map(|x| *id_map.get(x).unwrap())
                .unique()
                .collect();
            engine.max_level = std::cmp::max(engine.max_level, rule.level);
            engine
                .all_rules
                .entry(rule.level)
                .or_default()
                .entry(rule.symbol.clone())
                .or_default()
                .push(SharedRule::new(rule));
        }

        if rejected.is_empty() {
            Ok(engine)
        } else {
            Err(rejected)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Send + Sync>() {}

    #[test]
    fn rules_engine_is_send_sync() {
        is_send_sync::<RulesEngine>();
    }
}
