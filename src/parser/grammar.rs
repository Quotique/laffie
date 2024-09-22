use std::convert::From;

use trees::{tr, Tree};

use crate::CompactString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Data {
    pub symbol:   CompactString,
    pub position: usize,
}

impl Data {
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

        rule ident() -> Tree<Data> =
            _ p:position!() s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) { Data::new(s, p) }

        rule char_first_ident() -> Tree<Data> =
            _ p:position!() s:$(['a'..='z' | 'A'..='Z']
                ['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]*) {
                    Data::new(s, p)
            }

        rule number() -> Tree<Data> =
            _ p:position!() n:$(['0'..='9']+("."['0'..='9']+)?) { Data::new(n, p) }

        rule placeholder() -> Tree<Data> = p:position!() ".." { Data::new("..", p) }

        rule attrs() -> Vec<Tree<Data>> = _ keyword("attr") _ a:commasep(<arithmetic()>) { a }

        pub rule any() -> Vec<Tree<Data>> =
            _ s:( (task()/lang_rule()/symbol())* ) _ { s }

        pub rule terms() -> Vec<Tree<Data>> = _ c:semicolonsep(<arithmetic()>) _ { c }

        pub rule task() -> Tree<Data> = _ pp:position!() keyword("task") _ "{"
                _ pt:position!() keyword("purpose") _ p:eval() ";"
                _ tt:position!() t:(keyword("text") _ t:string() ";" {t} )?
                _ c:semicolonsep(<arithmetic()>)
            _ "}"
            {
                let mut p = Data::new("Task", pp)
                              /(Data::new("Purpose", pt) / p)
                              /(Data::new("Text", tt) / Data::new(t.unwrap_or_default().as_str(), tt));
                for i in c.iter().cloned() {
                    p.push_back(i);
                }
                p
            }

        pub rule symbol() -> Tree<Data> =
            _ ps:position!() keyword("symbol")
                _ pn:position!() s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '*' | '/' |
                        '^' | '=' | '<' | '>' | '!' | '|' | '&' | '_' ]+)
                _ pa:position!() a:( "{" _ a:attrs() _ "}" { a } / ";" { vec![] } )
                {
                    if !a.is_empty() {
                        let mut t = Data::new("Attrs", pa);
                        for i in a.into_iter() {
                            t.push_back(i);
                        }
                        Data::new("Declare", ps) / (Data::new("Symbol", ps) / Data::new(s, pn)) / t
                    } else {
                        Data::new("Declare", ps) / (Data::new("Symbol", ps) / Data::new(s, pn))
                    }
                }

        pub rule lang_rule() -> Tree<Data> =
            _ rp:position!() keyword("rule") _ "{"
                _ ap:position!() a:(a:attrs() _ ";" {a})?
                _ r:arithmetic() _ ";"
                _ pp:position!() p:commasep(<arithmetic()>)
                _ ";"?
            _ "}"
            {
                let mut t = Data::new("Rule", rp);
                t.push_back(r);
                let mut pred = Data::new("Predicates", pp);
                for i in p.iter().cloned() {
                    pred.push_back(i);
                }
                t.push_back(pred);
                if let Some(a) = a {
                    let mut attr = Data::new("Attributes", ap);
                    for i in a.iter().cloned() {
                        attr.push_back(i);
                    }
                    t.push_back(attr);
                }
                t
            }

        pub rule arithmetic() -> Tree<Data> = precedence!{
            p:position!() s:string() { Data::new(s.as_str(), p) }
            --
            x:(@) _ p:position!() "=>" _ y:@ { Data::new("=>", p)/x/y }
            x:(@) _ p:position!() "<=>" _ y:@ { Data::new("<=>", p)/x/y }
            --
            x:(@) _ p:position!() "||" _ y:@ { Data::new("||", p)/x/y }
            x:(@) _ p:position!() "&&" _ y:@ { Data::new("&&", p)/x/y }
            --
            x:(@) _ p:position!() "is" _ y:@ { Data::new("is", p)/x/y }
            x:(@) _ p:position!() "in" _ y:@ { Data::new("in", p)/x/y }
            x:(@) _ p:position!() "==" _ y:@ { Data::new("==", p)/x/y }
            x:(@) _ p:position!() "!=" _ y:@ { Data::new("!=", p)/x/y }
            x:(@) _ p:position!() "<=" _ y:@ { Data::new("<=", p)/x/y }
            x:(@) _ p:position!() ">=" _ y:@ { Data::new(">=", p)/x/y }
            x:(@) _ p:position!() "<" _ y:@ { Data::new("<", p)/x/y }
            x:(@) _ p:position!() ">" _ y:@ { Data::new(">", p)/x/y }
            --
            x:(@) _ p:position!() "as" _ y:ident() { Data::new("as", p)/x/y }
            --
            x:(@) _ p:position!() "+" _ y:@ { Data::new("+", p)/x/y }
            x:(@) _ p:position!() "-" _ y:@ { Data::new("+", p)/x/(Data::new("-", p)/y) }
            --
            p:position!() "-" _ x:@ { Data::new("-", p)/x }
            p:position!() "+" _ x:@ { Data::new("+", p)/x }
            p:position!() "!" _ x:@ { Data::new("!", p)/x }
            --
            x:(@) _ p:position!() "*" _ y:@ { Data::new("*", p)/x/y }
            x:(@) _ p:position!() "/" _ y:@ { Data::new("/", p)/x/y }
            --
            x:@ p:position!() "^" y:(@) { Data::new("^", p)/x/y }
            --
            n:number() p:position!() e:eval() { Data::new("*", p)/n/e }
            e:eval() { e }
            p:placeholder() { p }
            n:number() p:position!() i:char_first_ident() { Data::new("*", p)/n/i }
            n:number() { n }
            i:ident() { i }
            "(" _  a:arithmetic() _ ")" {a}
        }

        rule eval() -> Tree<Data> = t:char_first_ident() "(" _ a:commasep(<arithmetic()>) _ ")" {
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
            Ok(Data::new("sum", 0) /
                (Data::new("+", 8) / Data::new("one", 4) / Data::new("three", 10)) /
                Data::new("two", 17))
        );
    }

    #[test]
    fn bindings_test() {
        let test = "set(a, b) as S is Known";
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Data::new("is", 15) /
                (Data::new("as", 10) /
                    (Data::new("set", 0) / Data::new("a", 4) / Data::new("b", 7)) /
                    Data::new("S", 13)) /
                Data::new("Known", 18)
        )
    }

    #[test]
    fn predicate_parse_test() {
        let test = "x in Real; x is Unknown;";
        let states = ra::terms(test).unwrap();

        assert_eq!(states.len(), 2);
        assert_eq!(
            states[0],
            Data::new("in", 2) / Data::new("x", 0) / Data::new("Real", 5)
        );
        assert_eq!(
            states[1],
            Data::new("is", 13) / Data::new("x", 11) / Data::new("Unknown", 16)
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
            Data::new("=>", 13) /
                (Data::new("==", 8) /
                    (Data::new("*", 1) /
                        Data::new("x", 0) /
                        (Data::new("+", 4) / Data::new("y", 3) / Data::new("z", 5))) /
                    Data::new("0", 11)) /
                (Data::new("||", 25) /
                    (Data::new("==", 20) /
                        (Data::new("+", 17) / Data::new("y", 16) / Data::new("z", 18)) /
                        Data::new("0", 23)) /
                    (Data::new("==", 30) / Data::new("x", 28) / Data::new("0", 33)))
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
            Data::new("Rule", 0) /
                (Data::new("=>", 32) /
                    (Data::new("==", 28) /
                        (Data::new("+", 26) /
                            (Data::new("*", 24) /
                                Data::new("a", 23) /
                                Data::new("x", 25)) /
                            Data::new("b", 27)) /
                        Data::new("0", 30)) /
                    (Data::new("==", 36) /
                        Data::new("x", 35) /
                        (Data::new("/", 39) / Data::new("b", 38) / Data::new("a", 40)))) /
                (Data::new("Predicates", 59) /
                    (Data::new("!=", 60) / Data::new("a", 59) / Data::new("0", 62)))
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
            Data::new("Task", 0) /
                (Data::new("Purpose", 31) / (Data::new("find", 39) / Data::new("x", 44))) /
                (Data::new("Text", 72) / Data::new("", 72)) /
                (Data::new("==", 78) /
                    (Data::new("+", 75) /
                        (Data::new("*", 73) / Data::new("2", 72) / Data::new("x", 74)) /
                        Data::new("5", 76)) /
                    Data::new("0", 81))
        )
    }

    #[test]
    fn priority_test() {
        let test = r#"-a/b"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(
            states[0],
            Data::new("-", 0) / (Data::new("/", 2) / Data::new("a", 1) / Data::new("b", 3))
        );

        let test = r#"-a+b"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(
            states[0],
            Data::new("+", 2) / (Data::new("-", 0) / Data::new("a", 1)) / Data::new("b", 3)
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
            Data::new("-", 35) / (Data::new("/", 37) / Data::new("a", 36) / Data::new("b", 38))
        );
    }

    #[test]
    fn placeholder_test() {
        let test = "set(..) as S is Known";
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Data::new("is", 13) /
                (Data::new("as", 8) /
                    (Data::new("set", 0) / Data::new("..", 4)) /
                    Data::new("S", 11)) /
                Data::new("Known", 16)
        )
    }

    #[test]
    fn short_mul_ident_test() {
        let test = r#"6x"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        assert_eq!(
            states[0],
            Data::new("*", 1) / Data::new("6", 0) / Data::new("x", 1)
        );
    }

    #[test]
    fn short_mul_expr_test() {
        let test = r#"6sin(x)"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);

        assert_eq!(
            states[0],
            Data::new("*", 1) / Data::new("6", 0) / (Data::new("sin", 1) / Data::new("x", 5))
        );
    }

    #[test]
    fn decimal_fraction_test() {
        let test = r#"2.1sin(x)"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Data::new("*", 3) / Data::new("2.1", 0) / (Data::new("sin", 3) / Data::new("x", 7))
        );

        let test = r#"2.1/3.5"#;
        let states = ra::terms(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            Data::new("/", 3) / Data::new("2.1", 0) / Data::new("3.5", 4)
        );
    }

    #[test]
    fn task_text_parse_test() {
        let test = r#"task {
                        purpose find(x);
                        text "Решите уравнение 2x+5 = 0";
                        2*x+5 == 0;
                    }"#;
        let states = ra::task(test).unwrap();
        assert_eq!(
            states,
            Data::new("Task", 0) /
                (Data::new("Purpose", 31) / (Data::new("find", 39) / Data::new("x", 44))) /
                (Data::new("Text", 72) / Data::new("Решите уравнение 2x+5 = 0", 72)) /
                (Data::new("==", 151) /
                    (Data::new("+", 148) /
                        (Data::new("*", 146) / Data::new("2", 145) / Data::new("x", 147)) /
                        Data::new("5", 149)) /
                    Data::new("0", 154))
        )
    }
}
