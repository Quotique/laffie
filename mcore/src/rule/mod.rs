mod builder;
mod rule;
mod rules_engine;

pub use self::{
    builder::RuleBuilder,
    rule::{Rule, RuleAttr, RuleAttrValue, RuleDeclineReason, Suppose},
    rules_engine::{RulesEngine, SharedRule},
};

#[cfg(test)]
pub fn parse_rule(text: &'static str) -> Rule {
    let mut rules = parser::RuleParser::with(&parser::ra::lang_rule(text).unwrap())
        .parse()
        .unwrap();
    assert_eq!(rules.len(), 1);
    let rule = rules.pop().unwrap();

    unsafe { std::mem::transmute::<_, Rule>(rule) }
}
