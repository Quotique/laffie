mod operations;
mod symbols;

pub use self::{operations::normalize, symbols::*};

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
}
