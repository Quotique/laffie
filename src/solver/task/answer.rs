use crate::term::{SharedTerm, Term, TermBuf, TermRef, match_term};

pub(crate) enum Recognized {
    No,
    Whole,
    Binding(usize),
}

#[derive(Debug)]
pub struct Answer {
    parts: Vec<Part>,
}

#[derive(Debug)]
struct Part {
    asked: TermBuf,
    got:   Option<(SharedTerm, usize)>,
}

impl Answer {
    pub fn ready(&self) -> bool {
        self.parts.iter().all(|p| p.got.is_some())
    }

    pub fn term(&self) -> Option<TermBuf> {
        let mut parts = self.parts.iter();
        let mut result = parts.next()?.got.as_ref()?.0.as_ref().clone();
        for part in parts {
            result = TermBuf::symbol("&&")
                .arg(result)
                .arg(part.got.as_ref()?.0.as_ref().clone());
        }
        Some(result)
    }

    pub fn bindings(&self) -> impl Iterator<Item = (&TermBuf, usize)> {
        self.parts
            .iter()
            .filter_map(|p| p.got.as_ref().map(|(_, id)| (&p.asked, *id)))
    }

    pub(super) fn new(asked: impl Iterator<Item = TermBuf>) -> Self {
        Self {
            parts: asked.map(|asked| Part { asked, got: None }).collect(),
        }
    }

    pub(crate) fn recognize(
        &self,
        term: TermRef,
        known: &mut dyn FnMut(TermRef) -> bool,
    ) -> Recognized {
        if let [only] = &self.parts[..] {
            return if is_answer_form(term, only.asked.term(), known) {
                Recognized::Whole
            } else {
                Recognized::No
            };
        }
        match self
            .parts
            .iter()
            .position(|p| is_answer_leaf(term, p.asked.term(), known))
        {
            Some(at) => Recognized::Binding(at),
            None => Recognized::No,
        }
    }

    pub(crate) fn bind(&mut self, at: usize, term: SharedTerm, id: usize) -> bool {
        let part = &mut self.parts[at];
        if part.got.is_some() {
            return false;
        }
        part.got = Some((term, id));
        true
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
    use super::{Answer, Recognized};
    use crate::term::{SharedTerm, TermRef, term_with_vars};

    /// A value counts as known unless it mentions one of `unknown`.
    fn ask(asked: &[&'static str], term: &'static str, unknown: &[&str]) -> Recognized {
        let answer = Answer::new(asked.iter().map(|a| term_with_vars(a)));
        let term = term_with_vars(term);
        let mut is_known = |x: TermRef| {
            let rendered = x.to_string();
            !unknown.iter().any(|name| rendered.contains(name))
        };
        answer.recognize(term.term(), &mut is_known)
    }

    fn recognized(term: &'static str, asked: &'static str, unknown: &[&str]) -> bool {
        matches!(ask(&[asked], term, unknown), Recognized::Whole)
    }

    fn bound(asked: &[&'static str], terms: &[&'static str]) -> Answer {
        let mut answer = Answer::new(asked.iter().map(|a| term_with_vars(a)));
        for (at, term) in terms.iter().enumerate() {
            assert!(answer.bind(at, SharedTerm::new(term_with_vars(term)), at));
        }
        answer
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
    fn a_binding_of_another_unknown_is_not() {
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
    fn a_piecewise_answer_needs_every_branch_to_resolve() {
        assert!(recognized("x == 1 || x == 2", "x", &[]));
        assert!(!recognized("x == 1 || x == y", "x", &["y"]));
    }

    #[test]
    fn several_unknowns_are_answered_one_binding_at_a_time() {
        // A flat binding names which part it resolves.
        assert!(matches!(
            ask(&["x", "y"], "y == 2", &[]),
            Recognized::Binding(1)
        ));
        // A piecewise form is not a binding: with several unknowns only flat ones count.
        assert!(matches!(
            ask(&["x", "y"], "x == 1 || x == 2", &[]),
            Recognized::No
        ));
        // Nor is a binding of something nobody asked for.
        assert!(matches!(ask(&["x", "y"], "z == 3", &[]), Recognized::No));
        // Nor one to an unknown value.
        assert!(matches!(ask(&["x", "y"], "x == z", &["z"]), Recognized::No));
    }

    #[test]
    fn one_part_answers_alone() {
        let answer = bound(&["x"], &["x == 1"]);
        assert!(answer.ready());
        assert_eq!(answer.term().expect("ready"), term_with_vars("x == 1"));
    }

    #[test]
    fn several_parts_join_in_the_order_asked() {
        let answer = bound(&["x", "y"], &["x == 1", "y == 2"]);
        assert!(answer.ready());
        assert_eq!(
            answer.term().expect("ready"),
            term_with_vars("x == 1 && y == 2")
        );
    }

    #[test]
    fn an_unbound_part_leaves_no_answer() {
        let answer = bound(&["x", "y"], &["x == 1"]);
        assert!(!answer.ready());
        assert!(answer.term().is_none());
    }

    #[test]
    fn a_second_binding_of_one_part_is_refused() {
        let mut answer = bound(&["x"], &["x == 1"]);
        assert!(!answer.bind(0, SharedTerm::new(term_with_vars("x == 2")), 7));
        assert_eq!(answer.term().expect("ready"), term_with_vars("x == 1"));
    }

    #[test]
    fn bindings_report_the_term_that_answered_each_part() {
        let answer = bound(&["x", "y"], &["x == 1", "y == 2"]);
        let reported: Vec<_> = answer
            .bindings()
            .map(|(u, id)| (u.to_string(), id))
            .collect();
        assert_eq!(reported, vec![("x".to_owned(), 0), ("y".to_owned(), 1)]);
    }
}
