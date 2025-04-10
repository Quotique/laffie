use std::convert::From;

use trees::{tr, Tree};

use crate::CompactString;

pub(crate) const TOKEN_TASK: &str = "Task";
pub(crate) const TOKEN_PURPOSE: &str = "Purpose";
pub(crate) const TOKEN_TEXT: &str = "Text";
pub(crate) const TOKEN_ANSWER: &str = "Answer";
pub(crate) const TOKEN_DECLARE: &str = "Declare";
pub(crate) const TOKEN_SYMBOL: &str = "Symbol";
pub(crate) const TOKEN_RULE: &str = "Rule";
pub(crate) const TOKEN_PREDICATES: &str = "Predicates";
pub(crate) const TOKEN_ATTRIBUTES: &str = "Attributes";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub symbol:   CompactString,
    pub position: usize,
}

impl Token {
    pub fn new(data: &str, position: usize) -> Tree<Self> {
        tr(Self {
            symbol: CompactString::from(data),
            position,
        })
    }
}

peg::parser! {
    pub grammar ra() for str {
        use peg::ParseLiteral;

        rule line_comment() = "//" (!"\n" [_])* ("\n" / ![_])

        rule whitespace_char() = ['\t' | '\r' | ' ']

        rule _() = quiet!{ (whitespace_char() / "\n" / line_comment())* }

        rule comma() = ","

        pub rule string() -> String =
            "\"" s:quoted_char()* "\"" { s.into_iter().collect() }

        rule quoted_char() -> char = !("\"" / "\\") c:$([_]) { c.chars().next().unwrap() }

        rule commasep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ",") ","? {v}

        rule semicolonsep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ";") ";"? {v}

        rule keyword(id: &'static str) =
            ##parse_string_literal(id) !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule ident() -> Tree<Token> =
            _ p:position!() s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) { Token::new(s, p) }

        rule char_first_ident() -> Tree<Token> =
            _ p:position!() s:$(['a'..='z' | 'A'..='Z']
                ['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]*) {
                    Token::new(s, p)
            }

        rule number() -> Tree<Token> =
            _ p:position!() n:$(['0'..='9']+("."['0'..='9']+)?) { Token::new(n, p) }

        rule minus_number() -> Tree<Token> = "-" _ p:position!() _ n:number() {
            Token::new(&format!("-{}", n.root().data().symbol), p)
        }

        rule placeholder() -> Tree<Token> = p:position!() ".." { Token::new("..", p) }

        rule attrs() -> Vec<Tree<Token>> = _ keyword("attr") _ a:commasep(<arithmetic()>) { a }

        pub rule any() -> Vec<Tree<Token>> =
            _ s:( (task()/lang_rule()/symbol())* ) _ { s }

        pub rule terms() -> Vec<Tree<Token>> = _ c:semicolonsep(<arithmetic()>) _ { c }

        pub rule task() -> Tree<Token> = _ pp:position!() keyword("task") _ "{"
                _ pt:position!() keyword("purpose") _ p:eval() ";"
                _ tt:position!() t:(keyword("text") _ t:string() ";" {t} )?
                _ at:position!() a:(keyword("answer") _ a:commasep(<arithmetic()>) _ ";" {a} )?
                _ c:semicolonsep(<arithmetic()>)
            _ "}"
            {
                let mut p = Token::new(TOKEN_TASK, pp)
                              /(Token::new(TOKEN_PURPOSE, pt) / p)
                              /(Token::new(TOKEN_TEXT, tt)
                                /Token::new(t.unwrap_or_default().as_str(), tt));
                if let Some(ans) = a {
                    let mut answers = Token::new(TOKEN_ANSWER, at);
                    for i in ans.iter().cloned() {
                        answers.push_back(i);
                    }
                    p.push_back(answers);
                }
                for i in c.iter().cloned() {
                    p.push_back(i);
                }
                p
            }

        pub rule symbol() -> Tree<Token> =
            _ ps:position!() keyword("symbol")
                _ pn:position!() s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '*' | '/' |
                        '^' | '=' | '<' | '>' | '!' | '|' | '&' | '_' ]+)
                _ pa:position!() a:( "{" _ a:attrs() _ "}" { a } / ";" { vec![] } )
                {
                    if !a.is_empty() {
                        let mut t = Token::new("Attrs", pa);
                        for i in a.into_iter() {
                            t.push_back(i);
                        }
                        Token::new(TOKEN_DECLARE, ps)
                          / (Token::new(TOKEN_SYMBOL, ps) / Token::new(s, pn)) / t
                    } else {
                        Token::new(TOKEN_DECLARE, ps)
                          / (Token::new(TOKEN_SYMBOL, ps) / Token::new(s, pn))
                    }
                }

        pub rule lang_rule() -> Tree<Token> =
            _ rp:position!() keyword("rule") _ "{"
                _ ap:position!() a:(a:attrs() _ ";" {a})?
                _ r:arithmetic() _ ";"
                _ pp:position!() p:commasep(<arithmetic()>)
                _ ";"?
            _ "}"
            {
                let mut t = Token::new(TOKEN_RULE, rp);
                t.push_back(r);
                let mut pred = Token::new(TOKEN_PREDICATES, pp);
                for i in p.iter().cloned() {
                    pred.push_back(i);
                }
                t.push_back(pred);
                if let Some(a) = a {
                    let mut attr = Token::new(TOKEN_ATTRIBUTES, ap);
                    for i in a.iter().cloned() {
                        attr.push_back(i);
                    }
                    t.push_back(attr);
                }
                t
            }

        pub rule arithmetic() -> Tree<Token> = precedence!{
            p:position!() s:string() { Token::new(s.as_str(), p) }
            --
            x:(@) _ p:position!() "=>" _ y:@ { Token::new("=>", p) /x /y }
            x:(@) _ p:position!() "<=>" _ y:@ { Token::new("<=>", p) /x /y }
            --
            x:(@) _ p:position!() "||" _ y:@ { Token::new("||", p) /x /y }
            x:(@) _ p:position!() "&&" _ y:@ { Token::new("&&", p) /x /y }
            --
            x:(@) _ p:position!() "is" _ y:@ { Token::new("is", p) /x /y }
            x:(@) _ p:position!() "in" _ y:@ { Token::new("in", p) /x /y }
            x:(@) _ p:position!() "==" _ y:@ { Token::new("==", p) /x /y }
            x:(@) _ p:position!() "!=" _ y:@ { Token::new("!=", p) /x /y }
            x:(@) _ p:position!() "<=" _ y:@ { Token::new("<=", p) /x /y }
            x:(@) _ p:position!() ">=" _ y:@ { Token::new(">=", p) /x /y }
            x:(@) _ p:position!() "<" _ y:@ { Token::new("<", p) /x /y }
            x:(@) _ p:position!() ">" _ y:@ { Token::new(">", p) /x /y }
            --
            x:(@) _ p:position!() "as" _ y:ident() { Token::new("as", p) /x /y }
            --
            x:(@) _ p:position!() "+" _ y:@ { Token::new("+", p) /x /y }
            x:(@) _ p:position!() _ m:minus_number() _ y:@ { Token::new("+", p) /x /(Token::new("*", p) /m /y) }
            x:(@) _ p:position!() _ m:minus_number() _ "*" _ y:@ { Token::new("+", p) /x /(Token::new("*", p) /m /y) }
            x:(@) _ p:position!() _ m:minus_number() { Token::new("+", p) /x /m }
            x:(@) _ p:position!() "-" _ y:@ {
                Token::new("+", p) /x /(Token::new("*", p) /Token::new("-1", p) /y)
            }
            --
            n:minus_number() { n }
            p:position!() "-" _ x:@ { Token::new("*", p) /Token::new("-1", p) /x }
            p:position!() "+" _ x:@ { Token::new("+", p) /x }
            p:position!() "!" _ x:@ { Token::new("!", p) /x }
            --
            x:(@) _ p:position!() "*" _ y:@ { Token::new("*", p) /x /y }
            x:(@) _ p:position!() "/" _ y:@ { Token::new("/", p) /x /y }
            --
            x:@ p:position!() "^" y:(@) { Token::new("^", p) /x /y }
            --
            n:number() p:position!() e:eval() { Token::new("*", p) /n /e }
            e:eval() { e }
            p:placeholder() { p }
            n:number() p:position!() i:char_first_ident() { Token::new("*", p) /n /i }
            n:number() { n }
            i:ident() { i }
            "(" _  a:arithmetic() _ ")" {a}
        }

        rule eval() -> Tree<Token> = t:char_first_ident() "(" _ a:commasep(<arithmetic()>) _ ")" {
            let mut t = t;
            for i in a.iter().cloned() {
                t.push_back(i);
            }
            t
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_test() {
        assert_eq!(ra::string("\"sdfdsf\""), Ok(String::from("sdfdsf")));
    }

    #[test]
    fn args_test() {
        assert_eq!(
            ra::arithmetic("sum(one + three, two)"),
            Ok(Token::new("sum", 0) /
                (Token::new("+", 8) / Token::new("one", 4) / Token::new("three", 10)) /
                Token::new("two", 17))
        );
    }

    #[test]
    fn bindings_test() {
        let test = "set(a, b) as S is Known";
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Token::new("is", 15) /
                (Token::new("as", 10) /
                    (Token::new("set", 0) / Token::new("a", 4) / Token::new("b", 7)) /
                    Token::new("S", 13)) /
                Token::new("Known", 18)
        )
    }

    #[test]
    fn predicate_parse_test() {
        let test = "x in Real; x is Unknown;";
        let states = ra::terms(test).unwrap();

        assert_eq!(states.len(), 2);
        assert_eq!(
            states[0],
            Token::new("in", 2) / Token::new("x", 0) / Token::new("Real", 5)
        );
        assert_eq!(
            states[1],
            Token::new("is", 13) / Token::new("x", 11) / Token::new("Unknown", 16)
        );
    }

    #[test]
    fn rule_parse_test() {
        let test = r#"x*(y+z) == 0 => y+z == 0 || x == 0;
                      -sin(x) == 0 => x == Pi*n && n in Z;
                      a is true <=> a"#;

        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 3);
        assert_eq!(
            states[0],
            Token::new("=>", 13) /
                (Token::new("==", 8) /
                    (Token::new("*", 1) /
                        Token::new("x", 0) /
                        (Token::new("+", 4) / Token::new("y", 3) / Token::new("z", 5))) /
                    Token::new("0", 11)) /
                (Token::new("||", 25) /
                    (Token::new("==", 20) /
                        (Token::new("+", 17) / Token::new("y", 16) / Token::new("z", 18)) /
                        Token::new("0", 23)) /
                    (Token::new("==", 30) / Token::new("x", 28) / Token::new("0", 33)))
        );

        assert_eq!(
            states[1],
            Token::new("=>", 71) /
                (Token::new("==", 66) /
                    (Token::new("*", 58) /
                        Token::new("-1", 58) /
                        (Token::new("sin", 59) / Token::new("x", 63))) /
                    Token::new("0", 69)) /
                (Token::new("&&", 84) /
                    (Token::new("==", 76) /
                        Token::new("x", 74) /
                        (Token::new("*", 81) / Token::new("Pi", 79) / Token::new("n", 82))) /
                    (Token::new("in", 89) / Token::new("n", 87) / Token::new("Z", 92)))
        );
    }

    #[test]
    fn extended_rule_parse_test() {
        let test = r#"rule {
                a*x+b==0 => x==b/a;
                a!=0;
            }"#;
        let states = ra::lang_rule(test).unwrap();
        assert_eq!(
            states,
            Token::new("Rule", 0) /
                (Token::new("=>", 32) /
                    (Token::new("==", 28) /
                        (Token::new("+", 26) /
                            (Token::new("*", 24) /
                                Token::new("a", 23) /
                                Token::new("x", 25)) /
                            Token::new("b", 27)) /
                        Token::new("0", 30)) /
                    (Token::new("==", 36) /
                        Token::new("x", 35) /
                        (Token::new("/", 39) / Token::new("b", 38) / Token::new("a", 40)))) /
                (Token::new("Predicates", 59) /
                    (Token::new("!=", 60) / Token::new("a", 59) / Token::new("0", 62)))
        );
    }

    #[test]
    fn task_parse_test() {
        let test = r#"task {
                        purpose find(x);
                        2*x+5 == 0;
                    }"#;
        let states = ra::task(test).unwrap();
        assert_eq!(
            states,
            Token::new("Task", 0) /
                (Token::new("Purpose", 31) / (Token::new("find", 39) / Token::new("x", 44))) /
                (Token::new("Text", 72) / Token::new("", 72)) /
                (Token::new("==", 78) /
                    (Token::new("+", 75) /
                        (Token::new("*", 73) / Token::new("2", 72) / Token::new("x", 74)) /
                        Token::new("5", 76)) /
                    Token::new("0", 81))
        )
    }

    #[test]
    fn priority_test() {
        let test = r#"-a/b"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(
            states[0],
            Token::new("*", 0) /
                Token::new("-1", 0) /
                (Token::new("/", 2) / Token::new("a", 1) / Token::new("b", 3))
        );

        let test = r#"-a+b"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(
            states[0],
            Token::new("+", 2) /
                (Token::new("*", 0) / Token::new("-1", 0) / Token::new("a", 1)) /
                Token::new("b", 3)
        );
    }

    #[test]
    fn comment_test() {
        let test = r#"// test comment before
            -a/b // test comment near
            //test comment after
        "#;
        let states = ra::terms(test).unwrap();
        assert_eq!(
            states[0],
            Token::new("*", 35) /
                Token::new("-1", 35) /
                (Token::new("/", 37) / Token::new("a", 36) / Token::new("b", 38))
        );
    }

    #[test]
    fn placeholder_test() {
        let test = "set(..) as S is Known";
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Token::new("is", 13) /
                (Token::new("as", 8) /
                    (Token::new("set", 0) / Token::new("..", 4)) /
                    Token::new("S", 11)) /
                Token::new("Known", 16)
        )
    }

    #[test]
    fn polynom_test() {
        let test = r#"x^4 - 25*x^2 + 60*x - 36 != 0"#;
        let states = ra::terms(test).unwrap();

        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Token::new("!=", 25) /
                (Token::new("+", 20) /
                    (Token::new("+", 13) /
                        (Token::new("+", 4) /
                            (Token::new("^", 1) /
                                Token::new("x", 0) /
                                Token::new("4", 2)) /
                            (Token::new("*", 4) /
                                Token::new("-25", 6) /
                                (Token::new("^", 10) /
                                    Token::new("x", 9) /
                                    Token::new("2", 11)))) /
                        (Token::new("*", 17) / Token::new("60", 15) / Token::new("x", 18))) /
                    Token::new("-36", 22)) /
                Token::new("0", 28)
        )
    }

    #[test]
    fn short_mul_ident_test() {
        let test = r#"6x"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        assert_eq!(
            states[0],
            Token::new("*", 1) / Token::new("6", 0) / Token::new("x", 1)
        );
    }

    #[test]
    fn short_mul_expr_test() {
        let test = r#"6sin(x)"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        assert_eq!(
            states[0],
            Token::new("*", 1) / Token::new("6", 0) / (Token::new("sin", 1) / Token::new("x", 5))
        );
    }

    #[test]
    fn decimal_fraction_test() {
        let test = r#"2.1sin(x)"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Token::new("*", 3) / Token::new("2.1", 0) / (Token::new("sin", 3) / Token::new("x", 7))
        );

        let test = r#"2.1/3.5"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Token::new("/", 3) / Token::new("2.1", 0) / Token::new("3.5", 4)
        );
    }

    #[test]
    fn task_text_parse_test() {
        let test = r#"task {
                        purpose find(x);
                        text "Решите уравнение 2x+5 = 0";
                        answer x == -2.5;
                        2*x+5 == 0;
                    }"#;
        let states = ra::task(test).unwrap();
        assert_eq!(
            states,
            Token::new("Task", 0) /
                (Token::new("Purpose", 31) / (Token::new("find", 39) / Token::new("x", 44))) /
                (Token::new("Text", 72) / Token::new("Решите уравнение 2x+5 = 0", 72)) /
                (Token::new("Answer", 145) /
                    (Token::new("==", 154) / Token::new("x", 152) / Token::new("-2.5", 158))) /
                (Token::new("==", 193) /
                    (Token::new("+", 190) /
                        (Token::new("*", 188) /
                            Token::new("2", 187) /
                            Token::new("x", 189)) /
                        Token::new("5", 191)) /
                    Token::new("0", 196))
        )
    }
}
