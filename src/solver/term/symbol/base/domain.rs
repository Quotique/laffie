use crate::term::{Term, TermBuf, TermRef};

/// The head a premise rule wraps its condition in.
pub const NAME: &str = "domain";

pub fn mark(inner: TermBuf) -> TermBuf {
    TermBuf::symbol(NAME).arg(inner)
}

/// The condition inside a `domain(c)` marker.
pub fn marked(root: TermRef) -> Option<TermRef> {
    if !root.data().is_symbol_name(NAME) || root.degree() != 1 {
        return None;
    }
    root.first_arg()
}

#[cfg(test)]
mod tests {
    use super::{NAME, mark, marked};
    use crate::term::{TermBuf, term_with_vars};

    #[test]
    fn only_a_one_argument_domain_is_a_marker() {
        assert!(marked(TermBuf::symbol(NAME).term()).is_none());
        assert!(marked(mark(term_with_vars("0 <= a")).term()).is_some());
        let two_args = TermBuf::symbol(NAME)
            .arg(TermBuf::one())
            .arg(TermBuf::one());
        assert!(marked(two_args.term()).is_none());
    }

    #[test]
    fn an_ordinary_term_is_not_a_marker() {
        assert!(marked(term_with_vars("0 <= a").term()).is_none());
    }

    #[test]
    fn marked_returns_the_condition() {
        let wrapped = mark(term_with_vars("0 <= a"));
        assert_eq!(marked(wrapped.term()).unwrap().to_string(), "0<=a");
    }
}
