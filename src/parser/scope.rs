use std::{collections::HashMap, str::FromStr};

use solver::{
    NormLevel, Rational, from_decimal_str,
    term::{ArgList, Atom, Param, Substitute, Term, TermBuf, Variable, try_sym},
};

use crate::{
    ParserError,
    source::Source,
    syntax::{BinOp, Expr, ExprKind, Span},
};

/// What an unregistered name becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Names {
    Params,
    Variables,
}

/// Resolves the names of parsed expressions against the symbol registry.
///
/// INVARIANT: one scope spans one task term or a whole rule (action and
/// requirements share `as` params and `..` ids); task ids depend on it.
pub(crate) struct TermScope<'s> {
    src:             &'s Source<'s>,
    names:           Names,
    params:          HashMap<Param, TermBuf>,
    last_arglist_id: u64,
}

impl<'s> TermScope<'s> {
    pub fn new(src: &'s Source<'s>, names: Names) -> Self {
        Self {
            src,
            names,
            params: HashMap::new(),
            last_arglist_id: 0,
        }
    }

    /// One term of the scope, with every `as` param bound so far substituted.
    pub fn term(&mut self, e: &Expr) -> Result<TermBuf, ParserError> {
        let mut term = self.node(e)?;
        term.substitute_iter(self.params.clone());
        Ok(term.normalize(NormLevel::Off))
    }

    fn node(&mut self, e: &Expr) -> Result<TermBuf, ParserError> {
        match &e.kind {
            ExprKind::Num(n) => Ok(TermBuf::from(Atom::Number(n.clone()))),
            ExprKind::Name(s) => Ok(TermBuf::from(self.atom(s))),
            ExprKind::Str(s) => Ok(TermBuf::from(self.atom(s))),
            ExprKind::ArgList => Ok(TermBuf::from(self.atom(".."))),
            ExprKind::Call { name, args } => {
                let args = args
                    .iter()
                    .map(|a| self.node(a))
                    .collect::<Result<_, _>>()?;
                self.apply(name, args, e.span)
            }
            ExprKind::Solve { vars, givens } => {
                let vars = vars
                    .iter()
                    .map(|v| self.node(v))
                    .collect::<Result<_, _>>()?;
                let find = self.apply("find", vars, e.span)?;
                let mut args = vec![find];
                for g in givens {
                    args.push(self.node(g)?);
                }
                self.apply("solve", args, e.span)
            }
            ExprKind::Group(inner) => self.node(inner),
            ExprKind::Neg(inner) => self.negated(inner),
            ExprKind::Pos(inner) => {
                let inner = self.node(inner)?;
                self.apply("+", vec![inner], e.span)
            }
            ExprKind::Not(inner) => {
                let inner = self.node(inner)?;
                self.apply("!", vec![inner], e.span)
            }
            ExprKind::Bin { op, lhs, rhs } => {
                let lhs = self.node(lhs)?;
                let rhs = match op {
                    BinOp::Sub => self.negated(rhs)?,
                    _ => self.node(rhs)?,
                };
                self.apply(op.symbol(), vec![lhs, rhs], e.span)
            }
            ExprKind::Bind { expr, name } => {
                let value = self.node(expr)?;
                let param = Param::from_str(name).expect("a name is a valid param");
                if self.params.insert(param.clone(), value.clone()).is_some() {
                    return Err(self
                        .src
                        .error(e.span, format!("Multiple definition of param {param}")));
                }
                Ok(value)
            }
        }
    }

    /// `-e`: a number leading a product takes the sign (`a - 25*x^2` has `-25`), else `-1*e`.
    fn negated(&mut self, e: &Expr) -> Result<TermBuf, ParserError> {
        match &e.kind {
            ExprKind::Neg(inner) => self.node(inner),
            ExprKind::Num(n) => Ok(TermBuf::from(Atom::Number(-n.clone()))),
            ExprKind::Bin {
                op: op @ (BinOp::Mul | BinOp::Div | BinOp::Juxt),
                lhs,
                rhs,
            } if leads_with_number(lhs) => {
                let lhs = self.negated(lhs)?;
                let rhs = self.node(rhs)?;
                self.apply(op.symbol(), vec![lhs, rhs], e.span)
            }
            _ => {
                let minus_one = TermBuf::from(Atom::Number(-Rational::from_integer(1.into())));
                let term = self.node(e)?;
                self.apply("*", vec![minus_one, term], e.span)
            }
        }
    }

    /// `name(args..)`; only a symbol takes arguments.
    fn apply(
        &mut self,
        name: &str,
        args: Vec<TermBuf>,
        span: Span,
    ) -> Result<TermBuf, ParserError> {
        let mut term = TermBuf::from(self.atom(name));
        if term.term().data().symbol().is_some() {
            for arg in args {
                term.term_mut().push_last_arg(arg);
            }
        } else if !args.is_empty() {
            return Err(self
                .src
                .error(span, format!("Node {name} can't contains children!")));
        }
        Ok(term)
    }

    fn atom(&mut self, name: &str) -> Atom {
        if name == ".." {
            self.last_arglist_id += 1;
            Atom::ArgList(ArgList::from(self.last_arglist_id))
        } else if let Some(value) = from_decimal_str(name) {
            Atom::Number(value)
        } else if let Some(symbol) = try_sym(name) {
            Atom::Symbol(symbol)
        } else {
            match self.names {
                Names::Variables => {
                    Atom::Variable(Variable::from_str(name).expect("a name is a valid variable"))
                }
                Names::Params => {
                    Atom::Param(Param::from_str(name).expect("a name is a valid param"))
                }
            }
        }
    }
}

fn leads_with_number(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Num(_) => true,
        ExprKind::Bin {
            op: BinOp::Mul | BinOp::Div | BinOp::Juxt,
            lhs,
            ..
        } => leads_with_number(lhs),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use solver::term::{SymbolProgram, TermBuf, try_sym};

    use crate::{Names, legacy, term};

    /// Corpus forms that must give the reference's terms.
    const CORPUS_FORMS: &[&str] = &[
        "a - b",
        "a - 25*x^2",
        "2*x-5",
        "- 5",
        "-2*x/y",
        "-(2)*x",
        "-2^2",
        "-a/b",
        "-a+b",
        "x - 2^2*y",
        "-2/3",
        "2x",
        "2 x",
        "2sqrt(x)",
        "-2x",
        "-2x^2",
        "x^2",
        "x^(1/2)",
        "f()",
        "f(x, ..)",
        "set(a, b) as S",
        "set(5, x as S) is known <=> set(S) is known",
        "find(x) { x + 1 == 0 }",
        "!f(x)",
        "!(a is atom && b is known)",
        "+x",
        "a => b",
        "a <=> b",
        "x in Reals",
        "a is known",
        "a == c && b <=> b",
        "(a > 0 && sqrt(b) <= -c/a) || (a < 0 && sqrt(b) >= -c/a)",
        "a*x^2 + b*x + c > d <=> true",
        "x^4 - 25*x^2 != 0",
        "\"x\"",
    ];

    #[test]
    fn corpus_forms_give_the_reference_terms() {
        register(&[
            "f", "set", "find", "solve", "sqrt", "known", "atom", "Reals", "true",
        ]);
        for form in CORPUS_FORMS {
            let want = reference(form);
            assert!(want.is_some(), "the reference rejects {form}");
            assert_eq!(parsed(form), want, "{form}");
        }
    }

    #[test]
    fn a_power_binds_right_and_takes_a_signed_exponent() {
        same("x^2y", "(x^2)*y");
        same("x^-1", "x^(-1)");
        same("x^-y", "x^(-1*y)");
        same("2^3^2", "2^(3^2)");
        same("2x^2", "2*(x^2)");
        same("-2^2", "-1*(2^2)");
        same("-x^2", "-1*(x^2)");
        fails("x^");
        fails("2^^3");
    }

    #[test]
    fn juxtaposition_takes_a_number_or_a_parenthesis_on_the_left() {
        register(&["sqrt"]);
        same("2(x+1)", "2*(x+1)");
        same("(x+1)(x-1)", "(x+1)*(x-1)");
        same("2x", "2*x");
        same("2 x", "2*x");
        same("2sqrt(x)", "2*sqrt(x)");
        same("-2x", "(-2)*x");
        fails("x y");
        fails("(x+1)x");
        fails("(x+1)2");
        fails("2 3");
        fails("-5 7");
    }

    #[test]
    fn a_negation_sits_between_and_and_the_comparisons() {
        same("!a == b", "!(a == b)");
        same("!a && b", "(!a) && b");
    }

    #[test]
    fn a_minus_folds_into_a_literal_or_cancels() {
        same("x - -2", "x + 2");
        same("--2*x", "2*x");
        same("--x", "x");
        same("a * -b", "a * (-1*b)");
        same("a / -b", "a / (-1*b)");
        same("2 * -3", "2*(-3)");
        same("x + -2*x", "x + (-2)*x");
        same("x - 2^2", "x + -1*(2^2)");
    }

    #[test]
    fn a_keyword_needs_a_word_boundary() {
        fails("x inside");
        same("x in S", "in(x, S)");
    }

    #[test]
    fn deduction_and_comparison_do_not_chain() {
        fails("a == b == c");
        fails("a => b => c");
        fails("(a => b)");
    }

    fn parsed(text: &str) -> Option<TermBuf> {
        term(text, Names::Params).ok()
    }

    fn reference(text: &str) -> Option<TermBuf> {
        let trees = legacy::lang::terms(text).ok()?;
        legacy::TermParser::default().try_parse(&trees[0]).ok()
    }

    #[track_caller]
    fn same(text: &str, explicit: &str) {
        let (got, want) = (parsed(text), parsed(explicit));
        assert!(got.is_some(), "{text} does not parse");
        assert_eq!(got, want, "{text} vs {explicit}");
    }

    #[track_caller]
    fn fails(text: &str) {
        assert!(term(text, Names::Params).is_err(), "{text} parsed");
    }

    /// Symbols the forms name; a registered one keeps its program.
    fn register(names: &[&str]) {
        for name in names {
            if try_sym(name).is_none() {
                SymbolProgram {
                    name: (*name).into(),
                    ..Default::default()
                }
                .register();
            }
        }
    }
}
