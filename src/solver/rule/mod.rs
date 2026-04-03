mod builder;
mod hypothesis;
mod rule;
mod rule_attribute;
mod rules_engine;
mod term_filters;

pub use builder::RuleBuilder;
pub use hypothesis::{GroundedHypothesis, Hypothesis, HypothesisIterator};
pub use rule::{ApplyRule, Level, Rule, RuleDeclineReason, RuleId, SharedRule};
pub use rule_attribute::{RuleAttr, RuleAttrValue};
pub use rules_engine::RulesEngine;
pub use term_filters::TermFilters;

#[cfg(test)]
pub fn parse_rule(text: &'static str) -> Rule {
    let mut rules = parser::RuleParser::from(&parser::lang::lang_rule(text).unwrap())
        .parse()
        .unwrap();
    assert_eq!(rules.len(), 1);
    let rule = rules.pop().unwrap();

    unsafe { std::mem::transmute::<_, Rule>(rule) }
}
