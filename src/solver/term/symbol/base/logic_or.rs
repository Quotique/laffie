use super::SymbolProgram;
use crate::term::{Term, TermRef, Truth, TruthCtx};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "||".into(),
        truth_checker: Box::new(or),
        ..Default::default()
    }
}

/// Kleene disjunction: `True` if any operand is `True`; `False` if every
/// operand is `False`; otherwise `Unknown`.
pub fn or(root: TermRef, ctx: TruthCtx) -> Truth {
    if !root.data().is_symbol_name("||") {
        return Truth::Unknown;
    }
    let mut all_false = true;
    for arg in root.args_iter() {
        match arg.truth(ctx) {
            Truth::True => return Truth::True,
            Truth::Unknown => all_false = false,
            Truth::False => {}
        }
    }
    if all_false {
        Truth::False
    } else {
        Truth::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::or;
    use crate::{
        NormLevel,
        term::{Truth, TruthCtx, term_with_vars},
    };

    fn truth(src: &'static str) -> Truth {
        let t = term_with_vars(src).normalize(NormLevel::Full);
        or(t.term(), TruthCtx::default())
    }

    #[test]
    fn true_if_any_true() {
        assert_eq!(truth("1 == 1 || x == 3"), Truth::True);
    }

    #[test]
    fn false_if_all_false() {
        assert_eq!(truth("1 == 2 || 3 == 4"), Truth::False);
    }

    #[test]
    fn unknown_otherwise() {
        assert_eq!(truth("1 == 2 || x == 3"), Truth::Unknown);
    }
}
