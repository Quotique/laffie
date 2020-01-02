extern crate trees;

pub mod lang;

#[cfg(test)]
mod parser_tests {
    use super::*;
    use parser::trees::linked::fully::tr;

    #[test]
    fn predicate_parse_test() {
        let test = "x in Real; x is Unknown;";
        let states = lang::StatementsParser::new().parse(test).unwrap();

        assert_eq!(states.len(), 2);
        assert_eq!(
            states[0],
            tr(String::from("in")) / tr(String::from("x")) / tr(String::from("Real"))
        );
        assert_eq!(
            states[1],
            tr(String::from("is")) / tr(String::from("x")) / tr(String::from("Unknown"))
        );
    }

    #[test]
    fn rule_parse_test() {
        let test = r#"x*(y+z) == 0 => y+z == 0 || x == 0;
                      -sin(x) == 0 => x == Pi*n && n in Z;"#;

        let states = lang::StatementsParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(
            states[0],
            tr(String::from("=>")) /
                (tr(String::from("==")) /
                    (tr(String::from("*")) /
                        tr(String::from("x")) /
                        (tr(String::from("+")) / tr(String::from("y")) / tr(String::from("z")))) /
                    tr(String::from("0"))) /
                (tr(String::from("||")) /
                    (tr(String::from("==")) /
                        (tr(String::from("+")) / tr(String::from("y")) / tr(String::from("z"))) /
                        tr(String::from("0"))) /
                    (tr(String::from("==")) / tr(String::from("x")) / tr(String::from("0"))))
        );
    }
}
