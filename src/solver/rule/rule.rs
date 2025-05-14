use std::{collections::HashSet, fmt, hash::Hash, rc::Rc, sync::Arc};

use eyre::Result;
use multimap::MultiMap;

use utils::VecDisplay;

use super::{
    hypothesis::Hypothesis,
    rule_attribute::{RuleAttr, RuleAttrValue},
};
use crate::{
    symbol::{FuncSymbol, SymbolNode},
    term::{NodePosition, ParamsMapping, Term, TermProps},
    NormalizationLevel, RuleId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleDeclineReason {
    LevelMissmatch,
    PurposeMissmatch,
    AlreadyApplied,
    Blocked,
    ParamsMappingErr(String),
}

pub type SharedRule = Rc<Rule>;
#[derive(Clone, Debug)]
pub struct Rule {
    pub id:          RuleId,
    pub level:       usize,
    pub func_symbol: Arc<FuncSymbol>,

    pub attrs: MultiMap<RuleAttr, RuleAttrValue>,
    pub block: Vec<RuleId>,

    pub term:    Term,
    pub pattern: NodePosition,
    pub replace: NodePosition,
    pub binds:   ParamsMapping,

    pub requirements: Vec<Term>,

    pub pattern_symbols: HashSet<Arc<FuncSymbol>>,
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Rule {}

impl Hash for Rule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}](L:{}) {} => {}",
            self.id,
            self.level,
            VecDisplay(&self.requirements),
            self.term
        )
    }
}

impl Rule {
    pub fn attribute(&self, attr: &RuleAttr) -> impl Iterator<Item = &RuleAttrValue> {
        self.attrs.iter_key(attr)
    }

    pub fn contains_attribute(&self, attr: &RuleAttr) -> bool {
        self.attrs.iter_key(attr).next().is_some()
    }

    pub fn norm_level(&self) -> NormalizationLevel {
        self.attrs
            .iter_key(&RuleAttr::Normalize)
            .filter_map(RuleAttrValue::uint)
            .max()
            .map_or(NormalizationLevel::max(), NormalizationLevel)
    }

    pub fn is_tautology(&self) -> bool {
        self.pattern_node() == self.replace_node()
    }

    pub fn is_term_suitable(&self, term: &TermProps) -> Result<(), RuleDeclineReason> {
        if self.level != term.weight {
            return Err(RuleDeclineReason::LevelMissmatch);
        } else if term.applied_rules.contains(&self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        } else if term.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        for s in self.pattern_symbols.iter() {
            if !term.func_symbols.contains(s) {
                return Err(RuleDeclineReason::ParamsMappingErr(format!(
                    "symbol: {} not found",
                    s.name
                )));
            }
        }

        Ok(())
    }

    pub fn purpose_mapping(
        &self,
        purpose: &TermProps,
    ) -> Result<Vec<ParamsMapping>, RuleDeclineReason> {
        // TODO: multiple purposes
        if let Some(RuleAttrValue::Target(pattern)) = self.attribute(&RuleAttr::Purpose).next() {
            return ParamsMapping::try_map(purpose.term.root(), pattern.root()).map_err(|_| {
                debug!(target: "rule_selection", "no match purpose: {}, required: {}", purpose, pattern);
                RuleDeclineReason::PurposeMissmatch
            });
        }
        if (*purpose.term).root().data().is_symbol_name("transform") {
            // Only transform rules for transform
            return Err(RuleDeclineReason::PurposeMissmatch);
        }
        Ok(vec![])
    }

    #[inline]
    pub fn pattern_node(&self) -> &SymbolNode {
        &self.term[&self.pattern]
    }

    #[inline]
    pub fn replace_node(&self) -> &SymbolNode {
        &self.term[&self.replace]
    }
}

pub trait ApplyRule {
    fn apply(
        &self,
        arg: &TermProps,
        purpose: &TermProps,
    ) -> Result<Vec<Hypothesis>, RuleDeclineReason>;
}

impl ApplyRule for SharedRule {
    fn apply(
        &self,
        arg: &TermProps,
        purpose: &TermProps,
    ) -> Result<Vec<Hypothesis>, RuleDeclineReason> {
        self.is_term_suitable(arg)?;
        let mut mapping = self.purpose_mapping(purpose)?;
        if mapping.is_empty() {
            mapping.push(Default::default());
        }

        if arg.applied_rules.contains(&self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        }
        if arg.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        let maps: Vec<_> = mapping
            .into_iter()
            .flat_map(|m| {
                ParamsMapping::subtree_map_extend(arg.term.root(), self.pattern_node(), m)
                    .into_iter()
            })
            .collect();
        if maps.is_empty() {
            return Err(RuleDeclineReason::ParamsMappingErr("no match".into()));
        }

        let mut result = vec![];
        for (maps, pos) in maps.into_iter() {
            for i in maps.into_iter() {
                let replace = Term::from(self.replace_node().deep_clone());

                let mut replace = replace.apply_map(&self.binds).apply_map(&i);
                let mut src = (*arg.term).clone();
                replace.swap_node(&mut src[&pos]);
                src = src.normalize(self.norm_level());
                let mut resolution = TermProps::from(Rc::new(src))
                    .with_rule(self.clone())
                    .with_parent(arg.id);
                resolution.blocked_rules.extend(self.block.iter().cloned());

                let hypothesis = Hypothesis {
                    requirements: self
                        .requirements
                        .iter()
                        .map(|r| Rc::new(r.apply_map(&self.binds).apply_map(&i)))
                        .collect(),
                    resolution,
                    params: i,
                };
                trace!(target: "rule_selection", "New hypothesis: {}", hypothesis);
                result.push(hypothesis);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
pub mod tests {
    use std::rc::Rc;

    use crate::{
        rule::{parse_rule, RuleDeclineReason},
        term::{term_with_params, term_with_vars, TermProps},
        NormalizationLevel,
    };

    use super::{ApplyRule, SharedRule};

    fn base_rule() -> SharedRule {
        Rc::new(parse_rule(
            r#"rule {
                attr level(1);
                a + x == 0 => x == -a;
                a!=0;
            }"#,
        ))
    }

    fn subtree_rule() -> SharedRule {
        Rc::new(parse_rule(
            r#"rule {
                attr subtree,level(1);
                --a <=> a;
            }"#,
        ))
    }

    fn rule_with_binds() -> SharedRule {
        Rc::new(parse_rule(
            r#"rule {
                attr level(1);
                a/((b + c) as D) == 0 <=> a == 0 && D != 0;
            }"#,
        ))
    }

    fn test_term_fraction() -> TermProps {
        TermProps::from(Rc::new(term_with_vars(r#"2/(x + 1) == 0"#)))
    }

    fn test_term() -> TermProps {
        TermProps::from(Rc::new(term_with_vars(r#"2 + x == 0"#)))
    }

    fn test_term_subtree() -> TermProps {
        TermProps::from(Rc::new(term_with_vars(r#"x + (-(-2)) == 0"#)))
    }

    fn test_purpose() -> TermProps {
        TermProps::from(Rc::new(term_with_params(r#"find(x)"#)))
    }

    #[test]
    fn level_comparsion_test() {
        let rule = base_rule();
        let term = test_term();
        let purpose = test_purpose();

        assert_eq!(
            rule.apply(&term, &purpose).err(),
            Some(RuleDeclineReason::LevelMissmatch)
        );
    }

    #[test]
    fn apply_test() {
        let rule = base_rule();
        let mut term = test_term();
        let purpose = test_purpose();

        term.weight = 1;
        let hypothesis = rule.apply(&term, &purpose);
        assert!(hypothesis.is_ok());
        let mut hypothesis = hypothesis.unwrap();
        hypothesis.sort_by_key(|x| x.requirements[0].to_string());

        assert_eq!(hypothesis.len(), 2);
        assert_eq!(hypothesis[0].requirements.len(), 1);
        assert_eq!(*hypothesis[0].requirements[0], term_with_vars("2 != 0"));
        assert_eq!(
            *hypothesis[0].resolution.term,
            term_with_vars("x == -2").normalize(NormalizationLevel::max())
        );
        assert_eq!(hypothesis[1].requirements.len(), 1);
        assert_eq!(*hypothesis[1].requirements[0], term_with_vars("x != 0"));
        assert_eq!(*hypothesis[1].resolution.term, term_with_vars("2 == -x"));
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn subtree_apply_test() {
        let rule = subtree_rule();
        let mut term = test_term_subtree();
        let purpose = test_purpose();

        term.weight = 1;
        let hypothesis = rule.apply(&term, &purpose);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 1);
        assert_eq!(hypothesis[0].requirements.len(), 0);
        assert_eq!(*hypothesis[0].resolution.term, term_with_vars("x + 2 == 0"));
    }

    #[test]
    fn subtree_apply_test_2() {
        let rule = Rc::new(parse_rule(
            r#"rule {
                attr level(0),purpose(transform(x)),replace;
                a && b <=> b;

                a is true;
            }"#,
        ));

        let test_term = r#"(x^4 - 25*x^2 + 60*x -36 != 0) && ((3600 < 0 && x in empty_set) || (3600 >= 0 && x in set(1, 2)))"#;
        let mut term = TermProps::from(Rc::new(term_with_vars(test_term)));

        let purpose = TermProps::from(Rc::new(term_with_vars(r#"transform(a)"#)));
        term.weight = 0;

        let hypothesis = rule.apply(&term, &purpose);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 3);
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn twice_apply_test() {
        let rule = subtree_rule();
        let mut term = test_term_subtree();
        let purpose = test_purpose();

        term.weight = 1;
        assert!(rule.apply(&term, &purpose).is_ok());
        assert_eq!(
            rule.apply(&term, &purpose).err(),
            Some(RuleDeclineReason::AlreadyApplied)
        );
    }

    #[test]
    fn bind_apply_test() {
        let rule = rule_with_binds();
        let mut term = test_term_fraction();
        let purpose = test_purpose();

        term.weight = 1;
        let hypothesis = rule.apply(&term, &purpose).unwrap();
        assert_eq!(hypothesis[0].requirements.len(), 0);
        assert_eq!(
            *hypothesis[0].resolution.term,
            term_with_vars("2 == 0 && x + 1 != 0")
        );
    }

    #[test]
    fn purpose_mapping_test() {
        let rule = Rc::new(parse_rule(
            r#"rule {
                attr level(0),purpose(find(x));
                a + x == 0 => x == -a;
            }"#,
        ));

        let test_term = r#"1 + a + 2 == 0"#;
        let mut term = TermProps::from(Rc::new(term_with_vars(test_term)));

        let purpose = TermProps::from(Rc::new(term_with_vars(r#"find(a+2)"#)));
        term.weight = 0;

        let hypothesis = rule.apply(&term, &purpose);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 1);
    }
}
