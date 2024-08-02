use std::{collections::HashSet, fmt, rc::Rc, str::FromStr, sync::Arc};

use eyre::{bail, Result};
use multimap::MultiMap;

use crate::{
    term::{FuncSymbol, NodePosition, ParamsMapping, Term, TermNode, TermProps},
    utils::VecDisplay,
    CompactString, NormalizationLevel, RuleId,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Equivalence,
    Replace,
    Purpose,
    Level,
    Zero,
    One,
    Block,
    Id,
    Normalize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Str(CompactString),
    Target(Term),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleDeclineReason {
    LevelMissmatch,
    PurposeMissmatch,
    AlreadyApplied,
    Blocked,
    ParamsMappingErr(String),
}

#[derive(Debug)]
pub struct Suppose {
    pub requirements: Vec<Rc<Term>>,
    pub resolution:   TermProps,
}

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

impl FromStr for RuleAttr {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "equivalence" => Ok(RuleAttr::Equivalence),
            "replace" => Ok(RuleAttr::Replace),
            "level" => Ok(RuleAttr::Level),
            "purpose" => Ok(RuleAttr::Purpose),
            "zero" => Ok(RuleAttr::Zero),
            "one" => Ok(RuleAttr::One),
            "id" => Ok(RuleAttr::Id),
            "block" => Ok(RuleAttr::Block),
            "normalize" => Ok(RuleAttr::Normalize),
            _ => bail!(""),
        }
    }
}

impl fmt::Display for Suppose {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}] => {}",
            VecDisplay(&self.requirements),
            self.resolution,
        )
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

impl RuleAttrValue {
    pub fn str(&self) -> Option<&str> {
        if let RuleAttrValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn uint(&self) -> Option<u64> {
        if let RuleAttrValue::UInt(u) = self {
            Some(*u)
        } else {
            None
        }
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

    pub fn is_purpose_suitable(&self, purpose: &TermProps) -> Result<(), RuleDeclineReason> {
        // TODO: multiple purposes
        if let Some(RuleAttrValue::Target(pattern)) = self.attribute(&RuleAttr::Purpose).next() {
            if pattern.map(&purpose.term).is_err() {
                trace!(target: "rule_selection", "no match purpose: {}, required: {}", purpose, pattern);
                return Err(RuleDeclineReason::PurposeMissmatch);
            }
            return Ok(());
        }
        if (*purpose.term).root().data().is_symbol_name("transform") {
            // Only transform rules for transform
            return Err(RuleDeclineReason::PurposeMissmatch);
        }
        Ok(())
    }

    pub fn apply(
        &self,
        arg: &mut TermProps,
        purpose: &TermProps,
    ) -> Result<Vec<Suppose>, RuleDeclineReason> {
        self.is_term_suitable(arg)?;
        self.is_purpose_suitable(purpose)?;

        if !arg.applied_rules.insert(self.id) {
            return Err(RuleDeclineReason::AlreadyApplied);
        }
        if arg.blocked_rules.contains(&self.id) {
            return Err(RuleDeclineReason::Blocked);
        }

        self.apply_subtree(arg)
    }

    #[inline]
    pub fn pattern_node(&self) -> &TermNode {
        &self.term[&self.pattern]
    }

    #[inline]
    pub fn replace_node(&self) -> &TermNode {
        &self.term[&self.replace]
    }

    fn apply_subtree(&self, arg: &mut TermProps) -> Result<Vec<Suppose>, RuleDeclineReason> {
        let maps = ParamsMapping::subtree_map(arg.term.root(), self.pattern_node());
        if maps.is_empty() {
            return Err(RuleDeclineReason::ParamsMappingErr("no match".into()));
        }

        let mut result = vec![];
        for (maps, pos) in maps.iter() {
            for i in maps.iter() {
                let replace = Term::from(self.replace_node().deep_clone());

                let mut replace = replace.apply_map(&self.binds).apply_map(i);
                let mut src = (*arg.term).clone();
                replace.swap_node(&mut src[pos]);
                // src = src.normalize(NormalizationLevel::max());
                src = src.normalize(self.norm_level());
                let mut resolution = TermProps::from(Rc::new(src)).with_parent(arg.id);
                resolution.blocked_rules.extend(self.block.iter().cloned());

                let suppose = Suppose {
                    requirements: self
                        .requirements
                        .iter()
                        .map(|r| Rc::new(r.apply_map(&self.binds).apply_map(i)))
                        .collect(),
                    resolution,
                };
                trace!(target: "rule_selection", "New suppose: {}", suppose);
                result.push(suppose);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
pub mod tests {
    use std::rc::Rc;

    use crate::{
        rule::{parse_rule, Rule, RuleDeclineReason},
        term::{term_with_vars, TermProps},
        NormalizationLevel,
    };

    fn base_rule() -> Rule {
        parse_rule(
            r#"rule {
                attr level(1);
                a + x == 0 => x == -a;
                a!=0;
            }"#,
        )
    }

    fn subtree_rule() -> Rule {
        parse_rule(
            r#"rule {
                attr subtree,level(1);
                --a <=> a;
            }"#,
        )
    }

    fn rule_with_binds() -> Rule {
        parse_rule(
            r#"rule {
                attr level(1);
                a/((b + c) as D) == 0 <=> a == 0 && D != 0;
            }"#,
        )
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
        TermProps::from(Rc::new(term_with_vars(r#"find(x)"#)))
    }

    #[test]
    fn level_comparsion_test() {
        let rule = base_rule();
        let mut term = test_term();
        let purpose = test_purpose();

        assert_eq!(
            rule.apply(&mut term, &purpose).err(),
            Some(RuleDeclineReason::LevelMissmatch)
        );
    }

    #[test]
    fn apply_test() {
        let rule = base_rule();
        let mut term = test_term();
        let purpose = test_purpose();

        term.weight = 1;
        let suppose = rule.apply(&mut term, &purpose);
        assert!(suppose.is_ok());
        let mut suppose = suppose.unwrap();
        suppose.sort_by_key(|x| x.requirements[0].to_string());

        assert_eq!(suppose.len(), 2);
        assert_eq!(suppose[0].requirements.len(), 1);
        assert_eq!(*suppose[0].requirements[0], term_with_vars("2 != 0"));
        assert_eq!(
            *suppose[0].resolution.term,
            term_with_vars("x == -2").normalize(NormalizationLevel::max())
        );
        assert_eq!(suppose[1].requirements.len(), 1);
        assert_eq!(*suppose[1].requirements[0], term_with_vars("x != 0"));
        assert_eq!(*suppose[1].resolution.term, term_with_vars("2 == -x"));
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn subtree_apply_test() {
        let rule = subtree_rule();
        let mut term = test_term_subtree();
        let purpose = test_purpose();

        term.weight = 1;
        let suppose = rule.apply(&mut term, &purpose);
        assert!(suppose.is_ok());
        let suppose = suppose.unwrap();
        assert_eq!(suppose.len(), 1);
        assert_eq!(suppose[0].requirements.len(), 0);
        assert_eq!(*suppose[0].resolution.term, term_with_vars("x + 2 == 0"));
    }

    #[test]
    fn subtree_apply_test_2() {
        let rule = parse_rule(
            r#"rule {
                attr level(0),purpose(transform(x)),replace;
                a && b <=> b;

                a is true;
            }"#,
        );

        let test_term = r#"(x^4 - 25*x^2 + 60*x -36 != 0) && ((3600 < 0 && x in empty_set) || (3600 >= 0 && x in set(1, 2)))"#;
        let mut term = TermProps::from(Rc::new(term_with_vars(test_term)));

        let purpose = TermProps::from(Rc::new(term_with_vars(r#"transform(a)"#)));
        term.weight = 0;

        let suppose = rule.apply(&mut term, &purpose);
        assert!(suppose.is_ok());
        let suppose = suppose.unwrap();
        assert_eq!(suppose.len(), 3);
    }

    #[test]
    #[ignore] // TODO: fix double - autoremove
    fn twice_apply_test() {
        let rule = subtree_rule();
        let mut term = test_term_subtree();
        let purpose = test_purpose();

        term.weight = 1;
        assert!(rule.apply(&mut term, &purpose).is_ok());
        assert_eq!(
            rule.apply(&mut term, &purpose).err(),
            Some(RuleDeclineReason::AlreadyApplied)
        );
    }

    #[test]
    fn bind_apply_test() {
        let rule = rule_with_binds();
        let mut term = test_term_fraction();
        let purpose = test_purpose();

        term.weight = 1;
        let suppose = rule.apply(&mut term, &purpose).unwrap();
        assert_eq!(suppose[0].requirements.len(), 0);
        assert_eq!(
            *suppose[0].resolution.term,
            term_with_vars("2 == 0 && x + 1 != 0")
        );
    }
}
