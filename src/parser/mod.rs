use std::fmt;

pub mod lang;
mod problem;
mod rule;
mod statement;

pub type Tree = trees::Tree<String>;
pub type Node = trees::Node<String>;

pub use self::{
    lang::StatementsParser as LangParser, problem::ProblemParser, statement::StatementParser,
};

#[derive(Clone, Debug)]
pub enum SemanticError {
    UnexpectedWord(String),
    WorngArgCount(String),

    Other(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedWord(w) => write!(f, "Unexpected word {}", w),
            Self::WorngArgCount(e) => write!(f, "Arg count missmath: {}", e),
            Self::Other(e) => write!(f, "Semantic error: {}", e),
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    use trees::linked::fully::tr;

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
                      -sin(x) == 0 => x == Pi*n && n in Z;
                      a is true <=> [a]"#;

        let states = lang::StatementsParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 3);
        assert_eq!(
            states[0],
            tr(String::from("=>")) /
                (tr(String::from("==")) /
                    (tr(String::from("*")) /
                        tr(String::from("x")) /
                        (tr(String::from("+")) /
                            tr(String::from("y")) /
                            tr(String::from("z")))) /
                    tr(String::from("0"))) /
                (tr(String::from("||")) /
                    (tr(String::from("==")) /
                        (tr(String::from("+")) /
                            tr(String::from("y")) /
                            tr(String::from("z"))) /
                        tr(String::from("0"))) /
                    (tr(String::from("==")) / tr(String::from("x")) / tr(String::from("0"))))
        );
    }

    #[test]
    fn extended_rule_parse_test() {
        let test = r#"rule {
                a*x+b==0 => x==b/a;
                a!=0;
            }"#;
        let states = lang::StatementsParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            tr(String::from("Rule")) /
                (tr(String::from("=>")) /
                    (tr(String::from("==")) /
                        (tr(String::from("+")) /
                            (tr(String::from("*")) /
                                tr(String::from("a")) /
                                tr(String::from("x"))) /
                            tr(String::from("b"))) /
                        tr(String::from("0"))) /
                    (tr(String::from("==")) /
                        tr(String::from("x")) /
                        (tr(String::from("/")) /
                            tr(String::from("b")) /
                            tr(String::from("a"))))) /
                (tr(String::from("Predicates")) /
                    (tr(String::from("!=")) / tr(String::from("a")) / tr(String::from("0"))))
        );
    }

    #[test]
    fn problem_parse_test() {
        let test = r#"problem {
                        2*x+5 == 0;
                        target find x;
                    };"#;
        let states = lang::StatementsParser::new().parse(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            tr(String::from("Problem")) /
                (tr(String::from("==")) /
                    (tr(String::from("+")) /
                        (tr(String::from("*")) /
                            tr(String::from("2")) /
                            tr(String::from("x"))) /
                        tr(String::from("5"))) /
                    tr(String::from("0"))) /
                (tr(String::from("Target")) / (tr(String::from("find")) / tr(String::from("x"))))
        )
    }
}
