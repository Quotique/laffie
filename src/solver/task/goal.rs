use std::{
    fmt,
    hash::{Hash, Hasher},
};

use super::Answer;
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
