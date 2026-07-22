use super::{SymbolProgram, dedup_operands};
use crate::{
    NormLevel,
    term::{Term, TermMut, TermRef, Truth, TruthCtx},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "||".into(),
        calculator: Box::new(dedup),
        truth_checker: Box::new(or),
        ..Default::default()
    }
}

/// Idempotence: `a | a | b` → `a | b` (a solution found via two branches
/// must not be rejected by the answer check as `x=0|x=0`).
pub fn dedup(root: &mut TermMut, _: NormLevel) -> bool {
    dedup_operands(root, "||")
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
        term::{Term, Truth, TruthCtx, term_with_vars},
    };

    fn truth(src: &'static str) -> Truth {
        let t = term_with_vars(src).normalize(NormLevel::Full);
        or(t.term(), TruthCtx::default())
    }

    #[test]
    fn dedup_equal_operands() {
        let t = term_with_vars("x == 0 || x == 0").normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "x==0");
    }

    #[test]
    fn dedup_keeps_distinct() {
        let t = term_with_vars("x == 0 || x == 1 || x == 0").normalize(NormLevel::Full);
        assert_eq!(t.term().degree(), 2);
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
