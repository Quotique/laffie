use crate::term::{SharedTerm, TermBuf};

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
    pub fn term(&self) -> TermBuf {
        let mut parts = self.parts.iter();
        let first = parts
            .next()
            .expect("an answer has a part per asked unknown");
        let mut result = first.term.as_ref().clone();
        for part in parts {
            result = TermBuf::symbol("&&")
                .arg(result)
                .arg(part.term.as_ref().clone());
        }
        result
    }

    pub fn parts(&self) -> &[AnswerPart] {
        &self.parts
    }

    pub fn matches(&self, expected: &[TermBuf]) -> bool {
        if expected.is_empty() {
            return true;
        }
        let term = self.term();
        // TODO: есть проблема с неправильным преобразованием дерева, что приводит к
        // некорректному прямому сравнению дерева.
        expected.contains(&term) || expected.iter().any(|x| x.to_string() == term.to_string())
    }

    pub(crate) fn new(parts: Vec<AnswerPart>) -> Self {
        Self { parts }
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, AnswerPart};
    use crate::term::{SharedTerm, term_with_vars};

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
