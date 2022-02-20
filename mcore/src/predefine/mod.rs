mod operations;
mod symbols;

pub use self::{operations::normalize, symbols::*};

#[cfg(test)]
mod tests {
    use crate::statement::statement_with_params;

    #[test]
    fn unification_test() {
        let mut test = statement_with_params("2*x*x + x + 3*x + 4 + 2 == 0");
        test.inpl_normalize();

        let mut test_norm = statement_with_params("2*x^2 + 4*x + 6 == 0");
        test_norm.inpl_normalize();
        assert_eq!(test, test_norm);
    }
}
