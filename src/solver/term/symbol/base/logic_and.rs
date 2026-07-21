use super::SymbolProgram;
use crate::term::{Term, TermRef, Truth};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "&&".into(),
        truth_checker: Box::new(and),
        ..Default::default()
    }
}

/// Kleene conjunction: `False` if any operand is `False`; `True` if every
/// operand is `True`; otherwise `Unknown`.
pub fn and(root: TermRef) -> Truth {
    if !root.data().is_symbol_name("&&") {
        return Truth::Unknown;
    }
    let mut all_true = true;
    for arg in root.args_iter() {
        match arg.truth() {
            Truth::False => return Truth::False,
            Truth::Unknown => all_true = false,
            Truth::True => {}
        }
    }
    if all_true {
        Truth::True
    } else {
        Truth::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::and;
    use crate::{
        NormLevel,
        term::{Truth, term_with_vars},
    };

    fn truth(src: &'static str) -> Truth {
        let t = term_with_vars(src).normalize(NormLevel::Full);
        and(t.term())
    }

    #[test]
    fn false_if_any_false() {
        // 1 == 2 is False, so the whole conjunction is False.
        assert_eq!(truth("1 == 2 && 3 == 3"), Truth::False);
    }

    #[test]
    fn true_if_all_true() {
        assert_eq!(truth("1 == 1 && 3 == 3"), Truth::True);
    }

    #[test]
    fn unknown_otherwise() {
        assert_eq!(truth("1 == 1 && x == 3"), Truth::Unknown);
    }
}
