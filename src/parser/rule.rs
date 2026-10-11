use std::{collections::HashSet, str::FromStr};

use solver::{
    rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder},
    term::{Atom, Term, TermBuf, TermMut, TermRef, var},
};

use crate::{
    Names, ParserError,
    scope::TermScope,
    source::Source,
    syntax::{self, BinOp, Expr},
};

impl syntax::Rule {
    pub(crate) fn build(&self, src: &Source) -> Result<Vec<Rule>, ParserError> {
        let action = Expr::bin(
            BinOp::Deduction(self.op),
            self.pattern.clone(),
            self.resolution.clone(),
            self.op_span,
        );
        let mut scope = TermScope::new(src, Names::Params);
        let mut action = scope.term(&action)?;
        let mut requirements = self
            .requirements
            .iter()
            .map(|e| scope.term(e))
            .collect::<Result<Vec<_>, _>>()?;
        promote_body_find_targets(&mut action, &mut requirements);

        let mut builder = RuleBuilder::from(action);
        for req in requirements {
            builder = builder.with_require(req);
        }
        for e in &self.attrs {
            let (key, value) = attr(e, src)?;
            builder = builder.with_attribute(key, value);
        }
        builder.build().map_err(|e| src.error(self.span, e))
    }
}

fn attr(e: &Expr, src: &Source) -> Result<(RuleAttr, RuleAttrValue), ParserError> {
    let (name, args) = e
        .as_call()
        .ok_or_else(|| src.error(e.span, "unknown attribute"))?;
    let key = RuleAttr::from_str(name).map_err(|_| src.error(e.span, "unknown attribute"))?;
    let arg = match args {
        [arg] => Ok(arg),
        _ => Err(src.error(e.span, "must have one argument")),
    };
    let value = match key {
        RuleAttr::Equivalence | RuleAttr::Replace => RuleAttrValue::None,
        RuleAttr::Level | RuleAttr::Normalize => RuleAttrValue::UInt(src.uint(arg?.span)?),
        RuleAttr::Goal | RuleAttr::Zero | RuleAttr::One => {
            RuleAttrValue::Goal(TermScope::new(src, Names::Params).term(arg?)?)
        }
        RuleAttr::Id | RuleAttr::Block => RuleAttrValue::Str(src.ident(arg?).into()),
    };
    Ok((key, value))
}

/// Promotes body `find(...)` targets from Param to Variable unless also matched
/// as Params on the LHS — avoids the Param↔Variable shuffle at sub-solve edges.
fn promote_body_find_targets(action: &mut TermBuf, requirements: &mut [TermBuf]) {
    let mut lhs_params = HashSet::new();
    if let Some(lhs) = action.term().first_arg() {
        collect_params(lhs, &mut lhs_params);
    }

    let mut find_targets = HashSet::new();
    if let Some(rhs) = action.term().last_arg() {
        collect_find_targets(rhs, &mut find_targets);
    }
    for req in requirements.iter() {
        collect_find_targets(req.term(), &mut find_targets);
    }

    // Names also matched as pattern Params must stay Param.
    let promote_set: HashSet<String> = find_targets.difference(&lhs_params).cloned().collect();
    if promote_set.is_empty() {
        return;
    }

    if let Some(mut rhs) = action.term_mut().last_arg_mut() {
        promote_params_in_place(&mut rhs, &promote_set);
    }
    for req in requirements.iter_mut() {
        promote_params_in_place(&mut req.term_mut(), &promote_set);
    }
}

fn collect_params(term: TermRef<'_>, out: &mut HashSet<String>) {
    if let Atom::Param(p) = term.data() {
        out.insert(p.as_ref().to_string());
    }
    for child in term.args_iter() {
        collect_params(child, out);
    }
}

fn collect_find_targets(term: TermRef<'_>, out: &mut HashSet<String>) {
    if term.data().is_symbol_name("find") {
        for arg in term.args_iter() {
            if let Atom::Param(p) = arg.data() {
                out.insert(p.as_ref().to_string());
            }
        }
    }
    for child in term.args_iter() {
        collect_find_targets(child, out);
    }
}

fn promote_params_in_place(term: &mut TermMut<'_>, names: &HashSet<String>) {
    if let Atom::Param(p) = term.data().clone() {
        let name: &str = p.as_ref();
        if names.contains(name) {
            *term.data_mut() = Atom::Variable(var(name));
        }
    }
    for mut child in term.iter_mut() {
        promote_params_in_place(&mut child, names);
    }
}

#[cfg(test)]
mod tests {
    use solver::{
        rule::{RuleAttr, RuleAttrValue},
        term::{TermBuf, param},
    };

    #[test]
    fn a_rule_reads_attributes_action_and_requirements() {
        let test = r#"rule {
                        attr replace,level(1);
                        a + x == 0 => x == -a;
                        a!=0;
                      }"#;

        let result = crate::rule(test);
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().to_owned(),
            TermBuf::symbol("==")
                .arg(TermBuf::symbol("+").arg(param("a")).arg(param("x")))
                .arg(TermBuf::number(0))
        );

        assert_eq!(
            rule.replace_node().to_owned(),
            TermBuf::symbol("==").arg(param("x")).arg(
                TermBuf::symbol("*")
                    .arg(TermBuf::number(-1))
                    .arg(param("a"))
            )
        );
        assert_eq!(rule.requirements.len(), 1);
        assert_eq!(
            rule.requirements[0],
            TermBuf::symbol("!=")
                .arg(param("a"))
                .arg(TermBuf::number(0))
        );

        assert_eq!(rule.attrs.len(), 2);

        assert!(rule.contains_attribute(&RuleAttr::Replace));
        assert_eq!(
            rule.attribute(&RuleAttr::Level).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(1)]
        );
    }

    #[test]
    fn an_equivalence_marks_the_rule_and_keeps_its_goal() {
        let test = r#"rule {
                    attr level(0),goal(prove(a/b is known));
                    a/b is known <=> true;
                    a is known,
                    b is known
                }"#;

        let result = crate::rule(test);
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();

        assert_eq!(
            rule.pattern_node().to_owned(),
            TermBuf::symbol("is")
                .arg(TermBuf::symbol("/").arg(param("a")).arg(param("b")))
                .arg(TermBuf::symbol("known"))
        );

        assert_eq!(rule.replace_node().to_owned(), TermBuf::symbol("true"));
        assert_eq!(rule.requirements.len(), 2);
        assert_eq!(
            rule.requirements[0],
            TermBuf::symbol("is")
                .arg(param("a"))
                .arg(TermBuf::symbol("known"))
        );
        assert_eq!(
            rule.requirements[1],
            TermBuf::symbol("is")
                .arg(param("b"))
                .arg(TermBuf::symbol("known"))
        );

        assert_eq!(rule.attrs.len(), 3);

        assert!(rule.contains_attribute(&RuleAttr::Equivalence));
        assert_eq!(
            rule.attribute(&RuleAttr::Level).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(0)]
        );
    }

    #[test]
    fn zero_and_one_expand_a_rule_into_three() {
        let test = r#"rule {
                        attr replace,level(1),one(a),zero(b);
                        a * x + b == 0 => x == -b / a;
                        a!=0;
                      }"#;

        let result = crate::rule(test);
        assert!(result.is_ok());

        let rules = result.unwrap();
        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn id_block_and_normalize_keep_their_values() {
        let test = r#"rule {
            attr replace,level(1),id(move_left),block(hello),block(world),normalize(4);
            a == b => a - b == 0;
            b != 0;
        }"#;

        let result = crate::rule(test);
        assert!(result.is_ok());

        let mut rules = result.unwrap();
        assert_eq!(rules.len(), 1);
        let rule = rules.pop().unwrap();
        assert_eq!(
            rule.attribute(&RuleAttr::Id).collect::<Vec<_>>(),
            vec![&RuleAttrValue::Str("move_left".into())]
        );
        assert_eq!(
            rule.attribute(&RuleAttr::Block).collect::<Vec<_>>(),
            vec![
                &RuleAttrValue::Str("hello".into()),
                &RuleAttrValue::Str("world".into())
            ]
        );
        assert_eq!(
            rule.attribute(&RuleAttr::Normalize).collect::<Vec<_>>(),
            vec![&RuleAttrValue::UInt(4)]
        );
    }

    #[test]
    fn an_unknown_rule_attribute_is_an_error_at_its_place() {
        let err = crate::rule("rule { attr foo; a <=> a; }").unwrap_err();
        assert_eq!(err.msg, "unknown attribute");
        assert_eq!((err.loc.row, err.loc.col), (1, 12));
        // `subtree` names no attribute.
        assert_eq!(
            crate::rule("rule { attr subtree; a <=> a; }")
                .unwrap_err()
                .msg,
            "unknown attribute"
        );
    }

    #[test]
    fn a_level_is_an_integer_as_written() {
        assert_eq!(
            crate::rule("rule { attr level(1.0); a <=> a; }")
                .unwrap_err()
                .msg,
            "must be u64"
        );
        assert!(crate::rule("rule { attr level(1); a <=> a; }").is_ok());
    }
}
