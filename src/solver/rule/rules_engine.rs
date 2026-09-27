use std::{collections::HashMap, fmt};

use indexmap::IndexMap;
use itertools::Itertools;

use super::{Rule, RuleAttr, RuleAttrValue, RuleId, SharedRule, TermFilters, rule::Level};
use crate::{
    CompactString,
    term::{Symbol, TermBuf},
};

type LevelRules = IndexMap<Symbol, Vec<SharedRule>>;

/// A rule the base could not file. Returned instead of the base, so nothing
/// runs against a rule set that is not what the `.sym` files say.
#[derive(Debug)]
pub enum RuleRejection {
    UndefinedId {
        rule: Rule,
        ids:  Vec<CompactString>,
    },
    /// Rejected rather than tried against every term.
    PatternWithoutSymbols { rule: Rule },
}

#[derive(Default)]
pub struct RulesEngine {
    all_rules: IndexMap<Level, LevelRules>,
    max_level: Level,
}

impl RulesEngine {
    pub fn max_level(&self) -> Level {
        self.max_level
    }

    /// Same order every run: both maps are `IndexMap`.
    pub fn iter(&self) -> impl Iterator<Item = SharedRule> + '_ {
        self.all_rules
            .values()
            .flat_map(|x| x.values().flat_map(|x| x.iter().cloned()))
    }

    pub fn suggest_rules(&self, filters: &TermFilters, goal: &TermBuf) -> Vec<SharedRule> {
        let empty_level = LevelRules::new();
        let level = self.all_rules.get(&filters.level).unwrap_or(&empty_level);
        let result: Vec<_> = filters
            .symbols()
            .iter()
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
            Self::PatternWithoutSymbols { rule } => {
                write!(f, "rule {rule} has no function symbol in its pattern")
            }
        }
    }
}

/// On failure, each rejection comes with the rule's position in the input.
impl FromIterator<Rule> for Result<RulesEngine, Vec<(usize, RuleRejection)>> {
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

        let mut uses: HashMap<&Symbol, usize> = HashMap::new();
        for rule in &rules {
            for s in rule.pattern_symbols.iter() {
                *uses.entry(s).or_default() += 1;
            }
        }
        let uses: HashMap<Symbol, usize> = uses.into_iter().map(|(s, n)| (s.clone(), n)).collect();

        let mut engine = RulesEngine::default();
        let mut rejected = Vec::new();
        for (at, mut rule) in rules.into_iter().enumerate() {
            let ids: Vec<CompactString> = rule
                .attribute(&RuleAttr::Block)
                .filter_map(RuleAttrValue::str)
                .filter(|x| !id_map.contains_key(*x))
                .map(CompactString::from)
                .collect();
            if !ids.is_empty() {
                rejected.push((at, RuleRejection::UndefinedId { rule, ids }));
                continue;
            }
            let Some(cell) = cell_of(&rule, &uses) else {
                rejected.push((at, RuleRejection::PatternWithoutSymbols { rule }));
                continue;
            };
            debug!("New rule on `{cell}`: {rule}");
            rule.block = rule
                .attribute(&RuleAttr::Block)
                .filter_map(RuleAttrValue::str)
                .map(|x| {
                    *id_map
                        .get(x)
                        .expect("an undefined id rejected the rule above")
                })
                .unique()
                .collect();
            engine.max_level = std::cmp::max(engine.max_level, rule.level);
            engine
                .all_rules
                .entry(rule.level)
                .or_default()
                .entry(cell)
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

/// The pattern symbol the fewest rules use, ties broken by name.
///
/// Any pattern symbol is sound, since a term without it cannot match; the
/// rarest wastes the fewest scans. The name keeps search order deterministic.
fn cell_of(rule: &Rule, uses: &HashMap<Symbol, usize>) -> Option<Symbol> {
    rule.pattern_symbols
        .iter()
        .min_by_key(|s| (uses.get(*s).copied().unwrap_or(0), s.as_str()))
        .cloned()
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
