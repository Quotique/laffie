mod builder;
mod rule;
mod rule_attribute;
mod rules_engine;
mod suppose;

pub use self::{
    builder::RuleBuilder,
    rule::{ApplyRule, Rule, RuleDeclineReason, SharedRule},
    rule_attribute::{RuleAttr, RuleAttrValue},
    rules_engine::RulesEngine,
    suppose::{Suppose, SupposesIterator},
};

#[cfg(test)]
pub fn parse_rule(text: &'static str) -> Rule {
    let mut rules = parser::RuleParser::with(&parser::lang::lang_rule(text).unwrap())
        .parse()
        .unwrap();
    assert_eq!(rules.len(), 1);
    let rule = rules.pop().unwrap();

    unsafe { std::mem::transmute::<_, Rule>(rule) }
}
