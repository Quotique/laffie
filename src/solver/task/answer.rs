use crate::{
    NormLevel,
    term::{SharedTerm, TermBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    parts: Vec<AnswerPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerPart {
    pub asked: TermBuf,
    pub term:  SharedTerm,
}

impl Answer {
    /// The answer as one term, shaped like a declared answer
    pub fn term(&self) -> TermBuf {
        let (first, rest) = self
            .parts
            .split_first()
            .expect("an answer has a part per asked unknown");
        if rest.is_empty() {
            return first.term.as_ref().clone();
        }
        let mut result = TermBuf::symbol("&&").arg(first.term.as_ref().clone());
        for part in rest {
            result = result.arg(part.term.as_ref().clone());
        }
        result.normalize(NormLevel::Full)
    }

    pub fn parts(&self) -> &[AnswerPart] {
        &self.parts
    }

    /// Whether the answer is one the task declares; declaring none accepts any.
    pub fn matches(&self, expected: &[TermBuf]) -> bool {
        expected.is_empty() || expected.contains(&self.term())
    }

    pub(crate) fn new(parts: Vec<AnswerPart>) -> Self {
        Self { parts }
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, AnswerPart};
    use crate::term::{SharedTerm, TermBuf, term_with_vars};

    fn answer(parts: &[(&'static str, &'static str)]) -> Answer {
        Answer::new(
            parts
                .iter()
                .map(|(asked, term)| AnswerPart {
                    asked: term_with_vars(asked),
                    term:  SharedTerm::new(term_with_vars(term)),
                })
                .collect(),
        )
    }

    #[test]
    fn one_part_answers_alone() {
        assert_eq!(answer(&[("x", "x == 1")]).term(), term_with_vars("x == 1"));
    }

    #[test]
    fn several_parts_join_in_the_order_asked() {
        assert_eq!(
            answer(&[("x", "x == 1"), ("y", "y == 2")]).term(),
            term_with_vars("x == 1 && y == 2")
        );
    }

    // A declared `p && q && r` parses into one node, so this must not nest.
    #[test]
    fn several_parts_join_flat() {
        assert_eq!(
            answer(&[("x", "x == 1"), ("y", "y == 2"), ("z", "z == 3")]).term(),
            TermBuf::symbol("&&")
                .arg(term_with_vars("x == 1"))
                .arg(term_with_vars("y == 2"))
                .arg(term_with_vars("z == 3"))
        );
    }

    #[test]
    fn nothing_declared_matches_anything() {
        assert!(answer(&[("x", "x == 1")]).matches(&[]));
    }

    #[test]
    fn a_declared_answer_is_matched_as_a_whole() {
        let a = answer(&[("x", "x == 1"), ("y", "y == 2")]);
        assert!(a.matches(&[term_with_vars("x == 1 && y == 2")]));
        assert!(!a.matches(&[term_with_vars("x == 1 && y == 3")]));
    }

    #[test]
    fn parts_keep_what_was_asked() {
        let a = answer(&[("x", "x == 1"), ("y", "y == 2")]);
        let asked: Vec<_> = a.parts().iter().map(|p| p.asked.to_string()).collect();
        assert_eq!(asked, vec!["x".to_owned(), "y".to_owned()]);
    }
}
