use std::{collections::HashSet, str::FromStr};

use solver::{
    CompactString,
    rule::{Rule, RuleAttr, RuleAttrValue, RuleBuilder},
    term::{Atom, Term, TermBuf, TermMut, TermRef, var},
};

use crate::ParserError;

use super::{Node, Tree, term::TermParser};

pub struct RuleParser<'a> {
    syntax_tree: &'a Tree,
}

impl<'a> From<&'a Tree> for RuleParser<'a> {
    fn from(syntax_tree: &'a Tree) -> Self {
        Self { syntax_tree }
    }
}

impl<'a> RuleParser<'a> {
    pub fn parse(self) -> Result<Vec<Rule>, ParserError> {
        if self.syntax_tree.root().data().symbol != "Rule" {
            return Err(ParserError {
                loc: self.syntax_tree.root().data().location.clone(),
                msg: "expected 'rule'".to_owned(),
            });
        }
        let mut parser = TermParser::default();

        let mut pattern_term: Option<TermBuf> = None;
        let mut pattern_loc = None;
        let mut requirements: Vec<TermBuf> = vec![];
        let mut attributes: Vec<(RuleAttr, RuleAttrValue)> = vec![];

        for child in self.syntax_tree.iter() {
            match child.data().symbol.as_str() {
                "=>" | "<=>" => {
                    if pattern_term.replace(parser.try_parse(child)?).is_some() {
                        return Err(ParserError {
                            loc: child.data().location.clone(),
                            msg: "only one rule template is allowed".to_owned(),
                        });
                    }
                    pattern_loc = Some(child.data().location.clone());
                }
                "Predicates" => {
                    for req in child.iter() {
                        requirements.push(parser.try_parse(req)?);
                    }
                }
                "Attributes" => {
                    for attr in child.iter() {
                        attributes.push(RuleParser::parse_attribute(attr)?);
                    }
                }
                _ => {
                    error!("{:?}", child.data().symbol);
                    return Err(ParserError {
                        loc: child.data().location.clone(),
                        msg: "unexpected word".to_owned(),
                    });
                }
            }
        }

        if let Some(pat) = pattern_term.as_mut() {
            promote_body_find_targets(pat, &mut requirements);
        }

        let mut builder = RuleBuilder::default();
        if let Some(pat) = pattern_term {
            let loc = pattern_loc.unwrap_or_else(|| self.syntax_tree.data().location.clone());
            builder = builder.with_term(pat).map_err(|e| ParserError {
                loc,
                msg: e.to_string(),
            })?;
        }
        for req in requirements {
            builder = builder.with_require(req);
        }
        for (attr, value) in attributes {
            builder = builder.with_attribute(attr, value);
        }

        builder.build().map_err(|e| ParserError {
            loc: self.syntax_tree.data().location.clone(),
            msg: e.to_string(),
        })
    }

    fn parse_attribute(attr: &Node) -> Result<(RuleAttr, RuleAttrValue), ParserError> {
        match attr.data().symbol.as_str() {
            "subtree" | "equivalence" | "replace" => Ok((
                RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                RuleAttrValue::None,
            )),
            "level" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::Level,
                    RuleAttrValue::UInt(data.symbol.parse::<u64>().map_err(|_| ParserError {
                        loc: data.location.clone(),
                        msg: "must be u64".to_owned(),
                    })?),
                ))
            }
            "goal" | "zero" | "one" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }

                let goal = TermParser::default().try_parse(attr.front().unwrap())?;
                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Goal(goal),
                ))
            }
            "id" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Str(CompactString::from(data.symbol.as_str())),
                ))
            }
            "block" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::from_str(attr.data().symbol.as_str()).unwrap(),
                    RuleAttrValue::Str(CompactString::from(data.symbol.as_str())),
                ))
            }
            "normalize" => {
                if attr.degree() != 1 {
                    return Err(ParserError {
                        loc: attr.data().location.clone(),
                        msg: "must have one argument".to_owned(),
                    });
                }
                let data = attr.front().unwrap().data();

                Ok((
                    RuleAttr::Normalize,
                    RuleAttrValue::UInt(data.symbol.parse::<u64>().map_err(|_| ParserError {
                        loc: data.location.clone(),
                        msg: "must be u64".to_owned(),
                    })?),
                ))
            }
            _ => Err(ParserError {
                loc: attr.data().location.clone(),
                msg: "unknown attribute".to_owned(),
            }),
        }
    }
}

/// Promotes body `find(...)` targets from Param to Variable unless also matched
/// as Params on the LHS — avoids the Param↔Variable shuffle at sub-solve edges.
fn promote_body_find_targets(template: &mut TermBuf, requirements: &mut [TermBuf]) {
    let mut lhs_params = HashSet::new();
    if let Some(lhs) = template.term().first_arg() {
        collect_params(lhs, &mut lhs_params);
    }

    let mut find_targets = HashSet::new();
    if let Some(rhs) = template.term().last_arg() {
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

    if let Some(mut rhs) = template.term_mut().last_arg_mut() {
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
