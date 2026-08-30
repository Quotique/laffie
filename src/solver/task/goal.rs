use std::fmt;

use super::TermProps;
use crate::term::{Term, TermBuf};

/// Why a term cannot be a goal.
#[derive(Clone, Debug)]
pub enum GoalError {
    /// Head symbol is none of `find` / `prove` / `transform`.
    NotAGoal(String),
    /// `prove` and `transform` take exactly one argument, `find` at least one.
    WrongArity(String),
}

#[derive(Clone, Debug)]
pub struct FindGoal {
    pub targets: Vec<TermBuf>,
    pub term:    TermProps,
}

#[derive(Clone)]
pub enum Goal {
    Find(FindGoal),
    Prove(TermProps),
    Transform(TermProps),
}

impl fmt::Debug for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(g) => write!(f, "Find: {:?}", g.term),
            Goal::Prove(s) => write!(f, "Prove: {s:?}"),
            Goal::Transform(s) => write!(f, "Transform: {s:?}"),
        }
    }
}

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Goal::Find(g) => write!(f, "Find: {}", g.term),
            Goal::Prove(s) => write!(f, "Prove: {s}"),
            Goal::Transform(s) => write!(f, "Transform: {s}"),
        }
    }
}

impl fmt::Display for GoalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GoalError::NotAGoal(g) => {
                write!(f, "goal {g} is not one of find/prove/transform")
            }
            GoalError::WrongArity(g) => write!(
                f,
                "goal {g} has a wrong argument count: prove/transform take one, find at least one"
            ),
        }
    }
}

impl Goal {
    /// The only fallible `Goal` constructor, so every `Goal` that exists is
    /// well-formed.
    pub fn parse(mut value: TermBuf) -> Result<Self, GoalError> {
        // Checked before anything is popped, so the error renders the goal as
        // written.
        let kind = match value.term().data().symbol().map(|s| s.as_str().to_owned()) {
            Some(head) if matches!(head.as_str(), "find" | "prove" | "transform") => head,
            _ => return Err(GoalError::NotAGoal(value.to_string())),
        };
        let degree = value.term().degree();
        let arity_ok = if kind == "find" {
            degree > 0
        } else {
            degree == 1
        };
        if !arity_ok {
            return Err(GoalError::WrongArity(value.to_string()));
        }

        let mut root = value.term_mut();
        if kind == "find" {
            let mut targets = Vec::with_capacity(degree);
            while let Some(arg) = root.pop_first_arg() {
                targets.push(arg);
            }
            return Ok(Self::Find(FindGoal {
                term: goal_term(targets[0].clone()),
                targets,
            }));
        }

        let inner = goal_term(root.pop_first_arg().unwrap());
        if kind == "prove" {
            Ok(Self::Prove(inner))
        } else {
            Ok(Self::Transform(inner))
        }
    }

    pub(crate) fn prove(inner: TermBuf) -> Self {
        Self::Prove(goal_term(inner))
    }

    pub(crate) fn transform(inner: TermBuf) -> Self {
        Self::Transform(goal_term(inner))
    }

    /// The goal as written: `find(a, b)` / `prove(x)` / `transform(x)`.
    /// Inverse of [`Goal::parse`], argument order preserved.
    pub(crate) fn to_term(&self) -> TermBuf {
        match self {
            Goal::Find(g) => g
                .targets
                .iter()
                .fold(TermBuf::symbol("find"), |acc, t| acc.arg(t.clone())),
            Goal::Prove(t) => TermBuf::symbol("prove").arg((*t.term).clone()),
            Goal::Transform(t) => TermBuf::symbol("transform").arg((*t.term).clone()),
        }
    }

    #[inline]
    pub fn term(&self) -> &TermProps {
        match self {
            Goal::Find(g) => &g.term,
            Goal::Prove(s) => s,
            Goal::Transform(s) => s,
        }
    }

    #[inline]
    pub fn is_transform(&self) -> bool {
        if let Goal::Transform(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_prove(&self) -> bool {
        if let Goal::Prove(_) = self {
            return true;
        }
        false
    }

    #[inline]
    pub fn is_find(&self) -> bool {
        if let Goal::Find(_) = self {
            return true;
        }
        false
    }
}

/// The search tells goal terms from derived ones by the `GOAL` flag alone.
fn goal_term(term: TermBuf) -> TermProps {
    let mut props = TermProps::from(term);
    props.filters.mark_goal();
    props
}

#[cfg(test)]
mod tests {
    use super::{Goal, GoalError};
    use crate::term::{TermBuf, term_with_vars};

    #[test]
    fn rejects_a_head_that_is_not_a_goal() {
        let err = Goal::parse(term_with_vars("sqrt(x)")).expect_err("sqrt is not a goal");
        // The argument must still be attached when the error is built.
        assert!(
            matches!(&err, GoalError::NotAGoal(g) if g.contains('x')),
            "{err}"
        );
        assert!(err.to_string().contains("find/prove/transform"), "{err}");
    }

    #[test]
    fn rejects_find_without_targets() {
        let err = Goal::parse(TermBuf::symbol("find")).expect_err("find needs a target");
        assert!(matches!(err, GoalError::WrongArity(_)), "{err}");
    }

    #[test]
    fn round_trips_through_to_term() {
        for src in ["find(x)", "find(x, y)", "prove(x > 0)", "transform(1 + 2)"] {
            let term = term_with_vars(src);
            let goal = Goal::parse(term.clone()).expect("a goal");
            assert_eq!(goal.to_term(), term, "{src}");
        }
    }

    #[test]
    fn parsed_goal_term_carries_the_goal_flag() {
        for src in ["find(x)", "prove(x > 0)", "transform(1 + 2)"] {
            let goal = Goal::parse(term_with_vars(src)).expect("a goal");
            assert!(goal.term().filters.is_goal(), "{src}");
        }
    }

    #[test]
    fn direct_constructors_need_no_parsing() {
        assert_eq!(
            Goal::prove(term_with_vars("x > 0")).to_term(),
            term_with_vars("prove(x > 0)")
        );
        assert_eq!(
            Goal::transform(term_with_vars("1 + 2")).to_term(),
            term_with_vars("transform(1 + 2)")
        );
    }
}
