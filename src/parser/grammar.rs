use trees::{tr, Tree};

peg::parser! {
    pub grammar ra() for str {
        use peg::ParseLiteral;

        rule _() = quiet!{[' '|'\t'|'\n']*}

        rule comma() = ","

        rule commasep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ",") ","? {v}

        rule semicolonsep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ";") ";"? {v}

        rule keyword(id: &'static str) = ##parse_string_literal(id) !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule ident() -> Tree<String> = _ s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) {tr(String::from(s))}

        rule attrs() -> Vec<Tree<String>> = _ keyword("attr") _ a:commasep(<arithmetic()>) { a }

        pub rule any() -> Vec<Tree<String>> = _ s:semicolonsep(<problem()/lang_rule()/symbol()>) _ { s }

        pub rule statements() -> Vec<Tree<String>> = _ c:semicolonsep(<arithmetic()>) { c }

        pub rule problem() -> Tree<String> = _ keyword("problem") _ "{"
            _ keyword("target") _ t:eval() ";"
                _ c:semicolonsep(<arithmetic()>)
            _ "}"
            {
                let mut p = tr(String::from("Problem")) /(tr(String::from("Target")) / t);
                for i in c.iter().cloned() {
                    p.push_back(i);
                }
                p
            }

        pub rule symbol() -> Tree<String> =
            _ keyword("symbol")
                _ s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '*' | '/' |
                        '^' | '=' | '<' | '>' | '!' | '|' | '&' | '_' ]+)
                _ a:("{" _ a:attrs() _ "}" { a } )?
                {
                    match a {
                        Some(a) => {
                            let mut t = tr(String::from("Attrs"));
                            for i in a.iter().cloned() {
                                t.push_back(i);
                            }
                            tr(String::from("Declare")) / (
                                tr(String::from("Symbol")) /
                                tr(String::from(s))) / t
                        }
                        None => {
                            tr(String::from("Declare")) / (
                                tr(String::from("Symbol")) / tr(String::from(s))
                            )
                        }
                    }
                }

        pub rule lang_rule() -> Tree<String> =
            _ keyword("rule") _ "{"
                _ a:(a:attrs() _ ";" {a})?
                _ r:arithmetic() _ ";"
                _ p:commasep(<arithmetic()>)
                _ ";"?
            _ "}"
            {
                let mut t = tr(String::from("Rule"));
                t.push_back(r);
                let mut pred = tr(String::from("Predicates"));
                for i in p.iter().cloned() {
                    pred.push_back(i);
                }
                t.push_back(pred);
                if let Some(a) = a {
                    let mut attr = tr(String::from("Attributes"));
                    for i in a.iter().cloned() {
                        attr.push_back(i);
                    }
                    t.push_back(attr);
                }
                t
            }

        pub rule arithmetic() -> Tree<String> = precedence!{
            s:string() { tr(s) }
            --
            x:(@) _ "=>" _ y:@ { tr(String::from("=>"))/x/y }
            x:(@) _ "<=>" _ y:@ { tr(String::from("<=>"))/x/y }
            --
            x:(@) _ "||" _ y:@ { tr(String::from("||"))/x/y }
            x:(@) _ "&&" _ y:@ { tr(String::from("&&"))/x/y }
            --
            x:(@) _ "is" _ y:@ { tr(String::from("is"))/x/y }
            x:(@) _ "in" _ y:@ { tr(String::from("in"))/x/y }
            x:(@) _ "==" _ y:@ { tr(String::from("=="))/x/y }
            x:(@) _ "!=" _ y:@ { tr(String::from("!="))/x/y }
            x:(@) _ "<=" _ y:@ { tr(String::from("<="))/x/y }
            x:(@) _ ">=" _ y:@ { tr(String::from(">="))/x/y }
            x:(@) _ "<" _ y:@ { tr(String::from("<"))/x/y }
            x:(@) _ ">" _ y:@ { tr(String::from(">"))/x/y }
            --
            "-" _ x:@ { tr(String::from("-"))/x }
            "+" _ x:@ { tr(String::from("+"))/x }
            "!" _ x:@ { tr(String::from("!"))/x }
            --
            x:(@) _ "+" _ y:@ { tr(String::from("+"))/x/y }
            x:(@) _ "-" _ y:@ { tr(String::from("-"))/x/y }
            --
            x:(@) _ "*" _ y:@ { tr(String::from("*"))/x/y }
            x:(@) _ "/" _ y:@ { tr(String::from("/"))/x/y }
            --
            x:@ "^" y:(@) { tr(String::from("^"))/x/y }
            --
            e:eval() { e }
            i:ident() { i }
            "(" _  a:arithmetic() _ ")" {a}
        }

        rule eval() -> Tree<String> = t:ident() "(" _ a:commasep(<arithmetic()>) _ ")" {
            let mut t = t;
            for i in a.iter().cloned() {
                t.push_back(i);
            }
            t
        }

        pub rule string() -> String =
            "\"" s:quoted_char()* "\"" { s.into_iter().collect() }

        rule quoted_char() -> char = !("\"" / "\\") c:$([_]) { c.chars().next().unwrap() }

    //    rule problem() -> Tree<String> = keyword("problem") _ "{" c:commasep<Tree<String>> "}"
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
            Ok(tr(String::from("sum")) /
                (tr(String::from("+")) / tr(String::from("one")) / tr(String::from("three"))) /
                tr(String::from("two")))
        );
    }

    #[test]
    fn predicate_parse_test() {
        let test = "x in Real; x is Unknown;";
        let states = ra::statements(test).unwrap();

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
                      a is true <=> a"#;

        let states = ra::statements(test).unwrap();
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
        let states = ra::lang_rule(test).unwrap();
        assert_eq!(
            states,
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
                        target find(x);
                        2*x+5 == 0;
                    }"#;
        let states = ra::problem(test).unwrap();
        assert_eq!(
            states,
            tr(String::from("Problem")) /
                (tr(String::from("Target")) / (tr(String::from("find")) / tr(String::from("x")))) /
                (tr(String::from("==")) /
                    (tr(String::from("+")) /
                        (tr(String::from("*")) /
                            tr(String::from("2")) /
                            tr(String::from("x"))) /
                        tr(String::from("5"))) /
                    tr(String::from("0")))
        )
    }

    #[test]
    fn priority_test() {
        let test = r#"-a/b"#;
        let states = ra::statements(test).unwrap();
        assert_eq!(
            states[0],
            tr(String::from("-")) /
                (tr(String::from("/")) / tr(String::from("a")) / tr(String::from("b")))
        )
    }
}
