use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::term::{SharedTerm, Term, TermBuf, TermRef, match_term};

/// Why a term cannot be a goal.
#[derive(Clone, Debug)]
pub enum GoalError {
    /// Head symbol is none of `find` / `prove` / `transform`.
    NotAGoal(String),
    /// `prove` and `transform` take exactly one argument, `find` at least one.
    WrongArity(String),
}

pub(crate) enum Recognized {
    No,
    Whole,
    Binding(TermBuf),
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
    /// Targets are shared, so taking them costs a refcount rather than a copy
    /// of every target term.
    Find {
        written: SharedTerm,
        targets: Arc<[TermBuf]>,
    },
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
            GoalKind::Find => GoalBody::Find {
                targets: written.term().args_iter().map(|a| a.to_owned()).collect(),
                written,
            },
            GoalKind::Prove => GoalBody::Prove(written),
            GoalKind::Transform => GoalBody::Transform(written),
        };
        Ok(Self { body })
    }

    /// The goal as written: `find(a, b)` / `prove(x)` / `transform(x)`.
    #[inline]
    pub fn term(&self) -> &SharedTerm {
        match &self.body {
            GoalBody::Find { written, .. } => written,
            GoalBody::Prove(t) | GoalBody::Transform(t) => t,
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

    pub(crate) fn recognize(
        &self,
        term: TermRef,
        known: &mut dyn FnMut(TermRef) -> bool,
    ) -> Recognized {
        let GoalBody::Find { targets, .. } = &self.body else {
            return Recognized::No;
        };
        if let [only] = &targets[..] {
            return if is_answer_form(term, only.term(), known) {
                Recognized::Whole
            } else {
                Recognized::No
            };
        }
        match targets
            .iter()
            .find(|t| is_answer_leaf(term, t.term(), known))
        {
            Some(target) => Recognized::Binding(target.clone()),
            None => Recognized::No,
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

    /// `None` for anything but a `find`.
    #[inline]
    pub(crate) fn targets(&self) -> Option<Arc<[TermBuf]>> {
        match &self.body {
            GoalBody::Find { targets, .. } => Some(targets.clone()),
            GoalBody::Prove(_) | GoalBody::Transform(_) => None,
        }
    }
}

fn is_answer_form(term: TermRef, target: TermRef, known: &mut dyn FnMut(TermRef) -> bool) -> bool {
    if term.data().is_symbol_name("||") {
        return term.degree() > 0 && term.args_iter().all(|b| is_answer_branch(b, target, known));
    }
    is_answer_branch(term, target, known)
}

/// `target == <known>` or `target in <known>`.
fn is_answer_leaf(term: TermRef, target: TermRef, known: &mut dyn FnMut(TermRef) -> bool) -> bool {
    let Some((lhs, rhs)) =
        match_term!(term, "=="(lhs, rhs)).or_else(|| match_term!(term, "in"(lhs, rhs)))
    else {
        return false;
    };
    lhs == target && known(rhs)
}

/// An answer leaf, or `&&(guards..., leaf)` with exactly one target-resolving
/// leaf and every other conjunct an `is known` guard.
fn is_answer_branch(
    branch: TermRef,
    target: TermRef,
    known: &mut dyn FnMut(TermRef) -> bool,
) -> bool {
    if is_answer_leaf(branch, target, known) {
        return true;
    }
    if !branch.data().is_symbol_name("&&") {
        return false;
    }
    let mut leaf_seen = false;
    for conjunct in branch.args_iter() {
        if is_answer_leaf(conjunct, target, known) {
            if leaf_seen {
                return false;
            }
            leaf_seen = true;
        } else if !known(conjunct) {
            return false;
        }
    }
    leaf_seen
}

#[cfg(test)]
mod tests {
    use super::{Goal, GoalError, Recognized};
    use crate::term::{TermBuf, term_with_vars};

    /// A value counts as known unless it mentions one of `unknown`.
    fn ask(goal: TermBuf, term: &'static str, unknown: &[&str]) -> Recognized {
        let goal = Goal::parse(goal).expect("a goal");
        let term = term_with_vars(term);
        let mut is_known = |x: crate::term::TermRef| {
            let rendered = x.to_string();
            !unknown.iter().any(|name| rendered.contains(name))
        };
        goal.recognize(term.term(), &mut is_known)
    }

    fn recognized(term: &'static str, target: &'static str, unknown: &[&str]) -> bool {
        let goal = TermBuf::symbol("find").arg(term_with_vars(target));
        matches!(ask(goal, term, unknown), Recognized::Whole)
    }

    #[test]
    fn a_flat_binding_is_an_answer() {
        assert!(recognized("x == 1", "x", &[]));
        assert!(recognized("x in set(1, 2)", "x", &[]));
    }

    #[test]
    fn a_binding_to_an_unknown_value_is_not() {
        assert!(!recognized("x == y", "x", &["y"]));
    }

    #[test]
    fn a_binding_of_another_target_is_not() {
        assert!(!recognized("y == 1", "x", &[]));
    }

    #[test]
    fn a_branch_may_carry_known_guards_beside_one_binding() {
        assert!(recognized("a != 0 && x == 1", "x", &[]));
        // Two bindings in one branch are not one answer.
        assert!(!recognized("x == 1 && x == 2", "x", &[]));
        // A guard that is not known leaves the branch unresolved.
        assert!(!recognized("b != 0 && x == 1", "x", &["b"]));
    }

    #[test]
    fn several_targets_are_answered_one_binding_at_a_time() {
        let goal = || {
            TermBuf::symbol("find")
                .arg(term_with_vars("x"))
                .arg(term_with_vars("y"))
        };

        // A flat binding names which target it resolves.
        assert!(matches!(
            ask(goal(), "y == 2", &[]),
            Recognized::Binding(t) if t == term_with_vars("y")
        ));
        // A piecewise form is not a binding: with several targets only flat ones count.
        assert!(matches!(
            ask(goal(), "x == 1 || x == 2", &[]),
            Recognized::No
        ));
        // Nor is a binding of something that is not a target.
        assert!(matches!(ask(goal(), "z == 3", &[]), Recognized::No));
        // Nor one to an unknown value.
        assert!(matches!(ask(goal(), "x == z", &["z"]), Recognized::No));
    }

    #[test]
    fn a_goal_that_is_not_a_find_recognizes_no_binding() {
        let prove = TermBuf::symbol("prove").arg(term_with_vars("x > 0"));
        assert!(matches!(ask(prove, "x == 1", &[]), Recognized::No));
    }

    #[test]
    fn a_piecewise_answer_needs_every_branch_to_resolve() {
        assert!(recognized("x == 1 || x == 2", "x", &[]));
        assert!(!recognized("x == 1 || x == y", "x", &["y"]));
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
