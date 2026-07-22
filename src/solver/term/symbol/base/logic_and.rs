use super::{SymbolProgram, dedup_operands};
use crate::{
    NormLevel,
    term::{Term, TermMut, TermRef, Truth, TruthCtx},
};

pub fn symbol() -> SymbolProgram {
    SymbolProgram {
        name: "&&".into(),
        calculator: Box::new(dedup),
        truth_checker: Box::new(and),
        ..Default::default()
    }
}

/// Idempotence: `a & a & b` → `a & b`.
pub fn dedup(root: &mut TermMut, _: NormLevel) -> bool {
    dedup_operands(root, "&&")
}

/// Kleene conjunction: `False` if any operand is `False`; `True` if every
/// operand is `True`; otherwise `Unknown`.
pub fn and(root: TermRef, ctx: TruthCtx) -> Truth {
    if !root.data().is_symbol_name("&&") {
        return Truth::Unknown;
    }
    let mut all_true = true;
    for arg in root.args_iter() {
        match arg.truth(ctx) {
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
        term::{Truth, TruthCtx, term_with_vars},
    };

    fn truth(src: &'static str) -> Truth {
        let t = term_with_vars(src).normalize(NormLevel::Full);
        and(t.term(), TruthCtx::default())
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

    #[test]
    fn dedup_equal_operands() {
        let t = term_with_vars("x == 0 && x == 0").normalize(NormLevel::Full);
        assert_eq!(t.term().to_string(), "x==0");
    }
}
