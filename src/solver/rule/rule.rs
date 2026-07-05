use std::{collections::HashSet, fmt, hash, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::{Display, From, Into};
use eyre::Result;
use itertools::Itertools;
use multimap::MultiMap;

use super::{Hypothesis, RuleAttr, RuleAttrValue, TermFilters};
use crate::{
    NormalizationLevel,
    term::{ParamSubstitution, Substitute, Symbol, Term, TermBuf, TermPath, TermRef},
};

/// Unique identifier for a rule.
#[derive(Clone, Copy, Debug, Default, Display)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(From)]
#[derive(Decode, Encode)]
pub struct RuleId(u64);

/// Represents the level (or priority) of a rule.
/// Higher levels are applied later.
#[derive(Clone, Copy, Debug, Default, Display)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(From, Into)]
#[derive(Decode, Encode)]
pub struct Level(u64);

/// Reasons why a rule could not be applied.
/// Used in `Result<…, RuleDeclineReason>`.
#[derive(Clone, Debug, Display, PartialEq, Eq)]
pub enum RuleDeclineReason {
    /// The term’s level does not match the rule’s required level.
    LevelMismatch,
    /// The task goal does not match the rule’s goal pattern.
    GoalMismatch,
    /// The rule has already been applied to the term.
    AlreadyApplied,
    /// The rule is explicitly blocked for the term.
    Blocked,
    /// An error occurred while trying to map parameters;
    /// contains a human‑readable message.
    ParamSubstitutionErr(String),
}

/// Convenient alias for a reference‑counted rule.
pub type SharedRule = Arc<Rule>;

/// A rewriting rule.
#[derive(Clone, Debug)]
pub struct Rule {
    pub id:     RuleId,
    pub level:  Level,
    pub symbol: Symbol,

    pub attrs: MultiMap<RuleAttr, RuleAttrValue>,
    pub block: Vec<RuleId>,

    pub term:    TermBuf,
    pub pattern: TermPath,
    pub replace: TermPath,

    pub requirements: Vec<TermBuf>,

    pub pattern_symbols: HashSet<Symbol>,
}

impl RuleId {
    /// Constructs a new `RuleId` from a mask and a raw identifier.
    pub fn new(mask: u64, id: u64) -> Self {
        Self(mask | id)
    }

    /// Increments the inner identifier by one.
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl Level {
    /// Returns the next higher level.
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Increments the level in‑place.
    pub fn increment(&mut self) {
        self.0 += 1;
    }
}

impl Eq for Rule {}
impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl hash::Hash for Rule {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}](L:{}) [{}] => {}",
            self.id,
            self.level,
            self.requirements.iter().format(", "),
            self.term
        )
    }
}

impl Rule {
    /// Returns an iterator over the values of a given attribute.
    pub fn attribute(&self, attr: &RuleAttr) -> impl Iterator<Item = &RuleAttrValue> {
        static EMPTY_VEC: Vec<RuleAttrValue> = Vec::new();
        self.attrs.get_vec(attr).unwrap_or(&EMPTY_VEC).iter()
    }

    /// Checks whether the rule contains at least one value for the given
    /// attribute.
    pub fn contains_attribute(&self, attr: &RuleAttr) -> bool {
        self.attrs.get(attr).is_some()
    }

    /// Determines the maximal normalization level requested by the rule’s
    /// `normalize` attributes. Max level by default.
    pub fn norm_level(&self) -> NormalizationLevel {
        self.attribute(&RuleAttr::Normalize)
            .filter_map(RuleAttrValue::uint)
            .max()
            .map_or(NormalizationLevel::max(), NormalizationLevel)
    }

    /// Checks if the pattern and replacement are syntactically identical.
    pub fn is_tautology(&self) -> bool {
        self.pattern_node() == self.replace_node()
    }

    /// Checks whether the rule can be applied under the given term filters and
    /// goal.
    ///
    /// Returns `Ok(())` if the rule passes all checks, otherwise returns the
    /// appropriate `RuleDeclineReason`.
    pub fn try_filter(
        &self,
        filters: &TermFilters,
        goal: &TermBuf,
    ) -> Result<(), RuleDeclineReason> {
        self.is_term_suitable(filters).inspect_err(|e| {
            trace!(
                target: "rule_selection",
                "Rule {self} rejected for filters {filters:?} by reason {e:?}",
            );
        })?;
        self.goal_mapping(goal).inspect_err(|e| {
            trace!(
                target: "rule_selection",
                "Rule {self} rejected for goal {goal} by reason {e:?}",
            );
        })?;
        Ok(())
    }

    /// Validates that the rule matches the supplied term filters.
    ///
    /// Checks level compatibility, already‑applied/blocked status, and that all
    /// symbols required by the pattern are present in the filters.
    pub fn is_term_suitable(&self, filters: &TermFilters) -> Result<(), RuleDeclineReason> {
        if self.level != filters.level {
            return Err(RuleDeclineReason::LevelMismatch);
        } else if filters.applied_rules.contains(&self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        } else if filters.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        for s in self.pattern_symbols.iter() {
            if !filters.symbols.contains(s) {
                return Err(RuleDeclineReason::ParamSubstitutionErr(format!(
                    "symbol: {s} not found"
                )));
            }
        }

        Ok(())
    }

    /// Attempts to map the rule’s goal attribute against the supplied goal
    /// term.
    ///
    /// Returns a vector of `ParamsMapping` on success, or an appropriate
    /// `RuleDeclineReason` if the goal does not match.
    pub fn goal_mapping(
        &self,
        goal: &TermBuf,
    ) -> Result<Vec<ParamSubstitution>, RuleDeclineReason> {
        // TODO: multiple goals
        if let Some(RuleAttrValue::Goal(pattern)) = self.attribute(&RuleAttr::Goal).next() {
            return goal.term().try_match(pattern.term()).map_err(|_| {
                debug!(target: "rule_selection", "no match goal: {goal}, required: {pattern}");
                RuleDeclineReason::GoalMismatch
            });
        }
        if goal.term().data().is_symbol_name("transform") {
            // Only transform rules for transform
            return Err(RuleDeclineReason::GoalMismatch);
        }
        Ok(vec![])
    }

    /// Retrieves the pattern subterm of the rule.
    #[inline]
    pub fn pattern_node(&self) -> TermRef<'_> {
        self.term.get(&self.pattern).unwrap()
    }

    /// Retrieves the replacement subterm of the rule.
    #[inline]
    pub fn replace_node(&self) -> TermRef<'_> {
        self.term.get(&self.replace).unwrap()
    }
}

pub trait ApplyRule {
    /// Attempts to apply the rule to `term` under the given `term_filters` and
    /// `goal`.
    ///
    /// Returns a vector of generated `Hypothesis` values on success, or a
    /// `RuleDeclineReason` explaining why the rule could not be applied.
    fn apply(
        &self,
        term: &TermBuf,
        term_filters: &TermFilters,
        goal: &TermBuf,
    ) -> Result<Vec<Hypothesis>, RuleDeclineReason>;
}

impl ApplyRule for SharedRule {
    /// Implements the actual rewriting logic for a shared rule.
    fn apply(
        &self,
        term: &TermBuf,
        term_filters: &TermFilters,
        goal: &TermBuf,
    ) -> Result<Vec<Hypothesis>, RuleDeclineReason> {
        self.is_term_suitable(term_filters)?;
        let mut mapping = self.goal_mapping(goal)?;
        if mapping.is_empty() {
            mapping.push(Default::default());
        }

        if term_filters.applied_rules.contains(&self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        }
        if term_filters.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        let maps: Vec<_> = mapping
            .into_iter()
            .flat_map(|m| {
                term.term()
                    .find_matching_subterms_extend(self.pattern_node(), m)
                    .into_iter()
            })
            .collect();
        if maps.is_empty() {
            return Err(RuleDeclineReason::ParamSubstitutionErr("no match".into()));
        }

        let mut result = vec![];
        for (maps, pos) in maps.into_iter() {
            for i in maps.into_iter() {
                let mut replace = self.replace_node().to_owned();
                replace.substitute(&i);
                let mut resolution = term.clone();
                replace.swap(&mut resolution.get_mut(&pos).unwrap());
                resolution = resolution.normalize(self.norm_level());

                let requirements: Vec<TermBuf> = self
                    .requirements
                    .iter()
                    .map(|r| r.clone().substituted(&i))
                    .collect();
                let free_params = requirements
                    .iter()
                    .flat_map(|term| {
                        term.term()
                            .bfs()
                            .filter_map(|v| v.data.param())
                            .cloned()
                            .collect::<Vec<_>>()
                            .into_iter()
                    })
                    .collect();

                let hypothesis = Hypothesis {
                    rule: self.clone(),
                    blocked_rules: self.block.clone(),
                    free_params,
                    requirements,
                    resolution,
                    params: i,
                    pos: pos.clone(),
                };
                trace!(target: "rule_selection", "New hypothesis: {hypothesis}");
                result.push(hypothesis);
            }
        }

        Ok(result)
    }
}

impl std::error::Error for RuleDeclineReason {}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::{
        NormalizationLevel,
        rule::{RuleDeclineReason, TermFilters, parse_rule},
        term::{TermBuf, term_with_params, term_with_vars},
    };

    use super::{ApplyRule, SharedRule};

    fn base_rule() -> SharedRule {
        Arc::new(parse_rule(
            r#"rule {
                attr level(1);
                a + x == 0 => x == -a;
                a!=0;
            }"#,
        ))
    }

    fn subtree_rule() -> SharedRule {
        Arc::new(parse_rule(
            r#"rule {
                attr subtree,level(1);
                --a <=> a;
            }"#,
        ))
    }

    fn rule_with_binds() -> SharedRule {
        Arc::new(parse_rule(
            r#"rule {
                attr level(1);
                a/((b + c) as D) == 0 <=> a == 0 && D != 0;
            }"#,
        ))
    }

    fn test_term_fraction() -> TermBuf {
        term_with_vars(r#"2/(x + 1) == 0"#)
    }

    fn test_term() -> TermBuf {
        term_with_vars(r#"2 + x == 0"#)
    }

    fn test_term_subtree() -> TermBuf {
        term_with_vars(r#"x + (-(-2)) == 0"#)
    }

    fn test_goal() -> TermBuf {
        term_with_params(r#"find(x)"#)
    }

    #[test]
    fn level_comparsion_test() {
        let rule = base_rule();
        let term = test_term();
        let goal = test_goal();
        let filters = TermFilters::from(term.symbols());

        assert_eq!(
            rule.apply(&term, &filters, &goal).err(),
            Some(RuleDeclineReason::LevelMismatch)
        );
    }

    #[test]
    fn apply_test() {
        let rule = base_rule();
        let term = test_term();
        let goal = test_goal();
        let mut filters = TermFilters::from(term.symbols());

        filters.level = 1.into();
        let hypothesis = rule.apply(&term, &filters, &goal);
        assert!(hypothesis.is_ok());
        let mut hypothesis = hypothesis.unwrap();
        hypothesis.sort_by_key(|x| x.requirements[0].to_string());

        assert_eq!(hypothesis.len(), 2);
        assert_eq!(hypothesis[0].requirements.len(), 1);
        assert_eq!(hypothesis[0].requirements[0], term_with_vars("2 != 0"));
        assert_eq!(
            hypothesis[0].resolution,
            term_with_vars("x == -2").normalize(NormalizationLevel::max())
        );
        assert_eq!(hypothesis[1].requirements.len(), 1);
        assert_eq!(hypothesis[1].requirements[0], term_with_vars("x != 0"));
        assert_eq!(hypothesis[1].resolution, term_with_vars("2 == -x"));
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn subtree_apply_test() {
        let rule = subtree_rule();
        let term = test_term_subtree();
        let goal = test_goal();
        let mut filters = TermFilters::from(term.symbols());

        filters.level = 1.into();
        let hypothesis = rule.apply(&term, &filters, &goal);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 1);
        assert_eq!(hypothesis[0].requirements.len(), 0);
        assert_eq!(hypothesis[0].resolution, term_with_vars("x + 2 == 0"));
    }

    #[test]
    fn subtree_apply_test_2() {
        let rule = Arc::new(parse_rule(
            r#"rule {
                attr level(0),goal(transform(x)),replace;
                a && b <=> b;

                a is true;
            }"#,
        ));

        let test_term = r#"(x^4 - 25*x^2 + 60*x -36 != 0) && ((3600 < 0 && x in empty_set) || (3600 >= 0 && x in set(1, 2)))"#;
        let term = term_with_vars(test_term);
        let goal = term_with_vars(r#"transform(a)"#);
        let mut filters = TermFilters::from(term.symbols());
        filters.level = 0.into();

        let hypothesis = rule.apply(&term, &filters, &goal);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 3);
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn twice_apply_test() {
        let rule = subtree_rule();
        let term = test_term_subtree();
        let goal = test_goal();
        let mut filters = TermFilters::from(term.symbols());

        filters.level = 1.into();
        assert!(rule.apply(&term, &filters, &goal).is_ok());
        assert_eq!(
            rule.apply(&term, &filters, &goal).err(),
            Some(RuleDeclineReason::AlreadyApplied)
        );
    }

    #[test]
    fn bind_apply_test() {
        let rule = rule_with_binds();
        let term = test_term_fraction();
        let goal = test_goal();
        let mut filters = TermFilters::from(term.symbols());

        filters.level = 1.into();
        let hypothesis = rule.apply(&term, &filters, &goal).unwrap();
        assert_eq!(hypothesis[0].requirements.len(), 0);
        assert_eq!(
            hypothesis[0].resolution,
            term_with_vars("2 == 0 && x + 1 != 0")
        );
    }

    #[test]
    fn goal_mapping_test() {
        let rule = Arc::new(parse_rule(
            r#"rule {
                attr level(0),goal(find(x));
                a + x == 0 => x == -a;
            }"#,
        ));

        let test_term = r#"1 + a + 2 == 0"#;
        let term = term_with_vars(test_term);
        let goal = term_with_vars(r#"find(a+2)"#);
        let mut filters = TermFilters::from(term.symbols());
        filters.level = 0.into();

        let hypothesis = rule.apply(&term, &filters, &goal);
        assert!(hypothesis.is_ok());
        let hypothesis = hypothesis.unwrap();
        assert_eq!(hypothesis.len(), 1);
    }
}
