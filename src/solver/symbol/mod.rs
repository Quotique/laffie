pub mod codec;
mod func;
mod symbol_enum;

pub use func::{
    base::normalize, FuncSymbol, SymbolAttr, SymbolAttrValue, TruthChecker, TruthResult,
};
pub use symbol_enum::{Param, Placeholder, Symbol, SymbolNode, SymbolTree, Variable};

#[cfg(test)]
mod tests {
    use crate::{term::term_with_params, NormalizationLevel};

    #[test]
    fn unification_test() {
        let test =
            term_with_params("2*x*x + x + 3*x + 4 + 2 == 0").normalize(NormalizationLevel::max());
        let test_norm =
            term_with_params("2*x^2 + 4*x + 6 == 0").normalize(NormalizationLevel::max());
        assert_eq!(test, test_norm);
    }

    #[test]
    fn unification_with_minus_test() {
        let test =
            term_with_params("x^2 + (-5)*x - x + 5 == 0").normalize(NormalizationLevel::max());
        let test_norm =
            term_with_params("x^2 + (-6)*x + 5 == 0").normalize(NormalizationLevel::max());
        assert_eq!(test, test_norm);
    }
}
