use crate::term::{SharedTerm, Term, TermBuf, TermRef, match_term};

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

    pub(crate) fn bind(&mut self, at: usize, term: SharedTerm, id: usize) -> bool {
        let part = &mut self.parts[at];
        if part.got.is_some() {
            return false;
        }
        part.got = Some((term, id));
        true
    }
}

pub(super) fn is_answer_form(
    term: TermRef,
    target: TermRef,
    known: &mut dyn FnMut(TermRef) -> bool,
) -> bool {
    if term.data().is_symbol_name("||") {
        return term.degree() > 0 && term.args_iter().all(|b| is_answer_branch(b, target, known));
    }
    is_answer_branch(term, target, known)
}

/// `target == <known>` or `target in <known>`.
pub(super) fn is_answer_leaf(
    term: TermRef,
    target: TermRef,
    known: &mut dyn FnMut(TermRef) -> bool,
) -> bool {
    let Some((lhs, rhs)) =
        match_term!(term, "=="(lhs, rhs)).or_else(|| match_term!(term, "in"(lhs, rhs)))
    else {
        return false;
    };
    lhs == target && known(rhs)
}

/// An answer leaf, or `&&(guards..., leaf)` with exactly one target-resolving
/// leaf and every other conjunct an `is known` guard.
pub(super) fn is_answer_branch(
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
    use super::Answer;
    use crate::term::{SharedTerm, term_with_vars};

    fn bound(asked: &[&'static str], terms: &[&'static str]) -> Answer {
        let mut answer = Answer::new(asked.iter().map(|a| term_with_vars(a)));
        for (at, term) in terms.iter().enumerate() {
            assert!(answer.bind(at, SharedTerm::new(term_with_vars(term)), at));
        }
        answer
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
