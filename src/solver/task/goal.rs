use std::{
    fmt,
    hash::{Hash, Hasher},
};

use super::{
    Answer,
    answer::{is_answer_form, is_answer_leaf},
};
use crate::term::{SharedTerm, Term, TermBuf, TermRef};

/// Why a term cannot be a goal.
#[derive(Clone, Debug)]
pub enum GoalError {
    /// Head symbol is none of `find` / `prove` / `transform`.
    NotAGoal(String),
    /// `prove` and `transform` take exactly one argument, `find` at least one.
    WrongArity(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalKind {
    Find,
    Prove,
    Transform,
}

/// What a task asks for, carrying the term it was written as.
#[derive(Clone)]
pub struct Goal {
    body: GoalBody,
}

#[derive(Clone, Debug)]
enum GoalBody {
    Find(SharedTerm),
    Prove(SharedTerm),
    Transform(SharedTerm),
}

impl Hash for Goal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.term().hash(state);
    }
}

impl Eq for Goal {}
impl PartialEq for Goal {
    fn eq(&self, other: &Self) -> bool {
        self.term() == other.term()
    }
}

impl fmt::Debug for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let subject = self.subject();
        match self.kind() {
            GoalKind::Find => write!(f, "Find: {subject:?}"),
            GoalKind::Prove => write!(f, "Prove: {subject:?}"),
            GoalKind::Transform => write!(f, "Transform: {subject:?}"),
        }
    }
}

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let subject = self.subject();
        match self.kind() {
            GoalKind::Find => write!(f, "Find: {subject}"),
            GoalKind::Prove => write!(f, "Prove: {subject}"),
            GoalKind::Transform => write!(f, "Transform: {subject}"),
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
    pub fn parse(value: TermBuf) -> Result<Self, GoalError> {
        // Checked before the term is consumed, so the error renders the goal as
        // written.
        let kind = match value.term().data().symbol().map(|s| match s.as_str() {
            "find" => Some(GoalKind::Find),
            "prove" => Some(GoalKind::Prove),
            "transform" => Some(GoalKind::Transform),
            _ => None,
        }) {
            Some(Some(kind)) => kind,
            _ => return Err(GoalError::NotAGoal(value.to_string())),
        };
        let degree = value.term().degree();
        let arity_ok = match kind {
            GoalKind::Find => degree > 0,
            GoalKind::Prove | GoalKind::Transform => degree == 1,
        };
        if !arity_ok {
            return Err(GoalError::WrongArity(value.to_string()));
        }

        let written = SharedTerm::new(value);
        let body = match kind {
            GoalKind::Find => GoalBody::Find(written),
            GoalKind::Prove => GoalBody::Prove(written),
            GoalKind::Transform => GoalBody::Transform(written),
        };
        Ok(Self { body })
    }

    /// The goal as written: `find(a, b)` / `prove(x)` / `transform(x)`.
    #[inline]
    pub fn term(&self) -> &SharedTerm {
        match &self.body {
            GoalBody::Find(t) | GoalBody::Prove(t) | GoalBody::Transform(t) => t,
        }
    }

    /// Always the first argument of the written form: the term to prove or
    /// transform, the first target of a `find`.
    pub fn subject(&self) -> TermRef<'_> {
        self.term()
            .term()
            .first_arg()
            .expect("a parsed goal has at least one argument")
    }

    pub(crate) fn parts(&self) -> usize {
        match &self.body {
            GoalBody::Find(written) => written.term().degree(),
            GoalBody::Prove(_) | GoalBody::Transform(_) => 1,
        }
    }

    pub(crate) fn recognize(
        &self,
        term: TermRef,
        known: &mut dyn FnMut(TermRef) -> bool,
    ) -> Option<usize> {
        let GoalBody::Find(written) = &self.body else {
            return None;
        };
        let written = written.term();
        let mut unknowns = written.args_iter();
        if written.degree() == 1 {
            let only = unknowns.next()?;
            return is_answer_form(term, only, known).then_some(0);
        }
        unknowns.position(|u| is_answer_leaf(term, u, known))
    }

    /// `None` for anything but a `find`.
    pub(crate) fn answer(&self) -> Option<Answer> {
        match &self.body {
            GoalBody::Find(written) => Some(Answer::new(
                written.term().args_iter().map(|a| a.to_owned()),
            )),
            GoalBody::Prove(_) | GoalBody::Transform(_) => None,
        }
    }

    #[inline]
    pub fn kind(&self) -> GoalKind {
        match &self.body {
            GoalBody::Find { .. } => GoalKind::Find,
            GoalBody::Prove(_) => GoalKind::Prove,
            GoalBody::Transform(_) => GoalKind::Transform,
        }
    }

    pub(crate) fn prove(inner: TermBuf) -> Self {
        Self {
            body: GoalBody::Prove(SharedTerm::new(TermBuf::symbol("prove").arg(inner))),
        }
    }

    pub(crate) fn transform(inner: TermBuf) -> Self {
        Self {
            body: GoalBody::Transform(SharedTerm::new(TermBuf::symbol("transform").arg(inner))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Goal, GoalError};
    use crate::term::{TermBuf, TermRef, term_with_vars};

    fn goal(src: &'static str) -> Goal {
        Goal::parse(term_with_vars(src)).expect("a goal")
    }

    /// A value counts as known unless it mentions one of `unknown`.
    fn ask(src: &'static str, term: &'static str, unknown: &[&str]) -> Option<usize> {
        let term = term_with_vars(term);
        let mut is_known = |x: TermRef| {
            let rendered = x.to_string();
            !unknown.iter().any(|name| rendered.contains(name))
        };
        goal(src).recognize(term.term(), &mut is_known)
    }

    #[test]
    fn a_goal_has_one_part_per_unknown() {
        assert_eq!(goal("find(x, y)").parts(), 2);
        assert_eq!(goal("find(x)").parts(), 1);
        assert_eq!(goal("prove(x > 0)").parts(), 1);
        assert_eq!(goal("transform(1 + 2)").parts(), 1);
    }

    #[test]
    fn a_flat_binding_is_an_answer() {
        assert_eq!(ask("find(x)", "x == 1", &[]), Some(0));
        assert_eq!(ask("find(x)", "x in set(1, 2)", &[]), Some(0));
    }

    #[test]
    fn a_binding_to_an_unknown_value_is_not() {
        assert_eq!(ask("find(x)", "x == y", &["y"]), None);
    }

    #[test]
    fn a_binding_of_another_unknown_is_not() {
        assert_eq!(ask("find(x)", "y == 1", &[]), None);
    }

    #[test]
    fn a_branch_may_carry_known_guards_beside_one_binding() {
        assert_eq!(ask("find(x)", "a != 0 && x == 1", &[]), Some(0));
        // Two bindings in one branch are not one answer.
        assert_eq!(ask("find(x)", "x == 1 && x == 2", &[]), None);
        // A guard that is not known leaves the branch unresolved.
        assert_eq!(ask("find(x)", "b != 0 && x == 1", &["b"]), None);
    }

    #[test]
    fn a_piecewise_answer_needs_every_branch_to_resolve() {
        assert_eq!(ask("find(x)", "x == 1 || x == 2", &[]), Some(0));
        assert_eq!(ask("find(x)", "x == 1 || x == y", &["y"]), None);
    }

    #[test]
    fn several_unknowns_are_answered_one_binding_at_a_time() {
        // A flat binding names which part it resolves.
        assert_eq!(ask("find(x, y)", "y == 2", &[]), Some(1));
        // A piecewise form is not a binding: with several unknowns only flat ones count.
        assert_eq!(ask("find(x, y)", "x == 1 || x == 2", &[]), None);
        // Nor is a binding of something nobody asked for.
        assert_eq!(ask("find(x, y)", "z == 3", &[]), None);
        // Nor one to an unknown value.
        assert_eq!(ask("find(x, y)", "x == z", &["z"]), None);
    }

    #[test]
    fn a_goal_that_is_not_a_find_recognizes_nothing() {
        assert_eq!(ask("prove(x > 0)", "x == 1", &[]), None);
        assert_eq!(ask("transform(1 + 2)", "x == 1", &[]), None);
    }

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
    fn the_written_form_is_the_term_it_was_parsed_from() {
        for src in ["find(x)", "find(x, y)", "prove(x > 0)", "transform(1 + 2)"] {
            let term = term_with_vars(src);
            let goal = Goal::parse(term.clone()).expect("a goal");
            assert_eq!(**goal.term(), term, "{src}");
        }
    }

    #[test]
    fn the_subject_is_the_first_argument_of_the_written_form() {
        for (src, subject) in [
            ("find(x, y)", "x"),
            ("prove(x > 0)", "x > 0"),
            ("transform(1 + 2)", "1 + 2"),
        ] {
            let goal = Goal::parse(term_with_vars(src)).expect("a goal");
            assert_eq!(goal.subject().to_owned(), term_with_vars(subject), "{src}");
        }
    }

    #[test]
    fn an_answer_is_asked_for_only_by_a_find() {
        let goal = Goal::parse(term_with_vars("find(x, y)")).expect("a goal");
        assert_eq!(goal.answer().expect("a find asks").bindings().count(), 0);
        assert!(Goal::prove(term_with_vars("x > 0")).answer().is_none());
        assert!(Goal::transform(term_with_vars("1 + 2")).answer().is_none());
    }

    #[test]
    fn direct_constructors_need_no_parsing() {
        assert_eq!(
            **Goal::prove(term_with_vars("x > 0")).term(),
            term_with_vars("prove(x > 0)")
        );
        assert_eq!(
            **Goal::transform(term_with_vars("1 + 2")).term(),
            term_with_vars("transform(1 + 2)")
        );
    }
}
