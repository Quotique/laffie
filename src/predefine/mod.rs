mod math;
mod operations;
mod replace;
mod symbols;

pub use self::{
    operations::{is_true, normalize},
    symbols::setup,
};

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::{ra, StatementParser};

    #[test]
    fn unification_test() {
        setup();

        let test = "2*x*x + x + 3*x + 4 + 2 == 0";
        let states = ra::statements(test).unwrap();
        assert_eq!(states.len(), 1);

        let mut result = StatementParser::new(&states[0]).parse().unwrap();
        result.inpl_normalize();

        let test_norm = "2*x^2 + 4*x + 6 == 0";
        let states = ra::statements(test_norm).unwrap();
        assert_eq!(states.len(), 1);

        let mut result_norm = StatementParser::new(&states[0]).parse().unwrap();
        result_norm.inpl_normalize();
        assert_eq!(result, result_norm);
    }
}
