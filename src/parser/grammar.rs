use trees::{tr, Tree};

fn tree(data: &str) -> Tree<String> {
    tr(String::from(data))
}

peg::parser! {
    pub grammar ra() for str {
        use peg::ParseLiteral;

        rule _() = quiet!{[' '|'\t'|'\n']*}

        rule comma() = ","

        rule commasep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ",") ","? {v}

        rule semicolonsep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ";") ";"? {v}

        rule keyword(id: &'static str) =
            ##parse_string_literal(id) !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule ident() -> Tree<String> =
            _ s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) { tree(s) }

        rule attrs() -> Vec<Tree<String>> = _ keyword("attr") _ a:commasep(<arithmetic()>) { a }

        pub rule any() -> Vec<Tree<String>> =
            _ s:semicolonsep(<problem()/lang_rule()/symbol()>) _ { s }

        pub rule statements() -> Vec<Tree<String>> = _ c:semicolonsep(<arithmetic()>) { c }

        pub rule problem() -> Tree<String> = _ keyword("problem") _ "{"
            _ keyword("target") _ t:eval() ";"
                _ c:semicolonsep(<arithmetic()>)
            _ "}"
            {
                let mut p = tree("Problem") /(tree("Target") / t);
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
                            let mut t = tree("Attrs");
                            for i in a.iter().cloned() {
                                t.push_back(i);
                            }
                            tree("Declare") / (tree("Symbol") / tree(s)) / t
                        }
                        None => {
                            tree("Declare") / (tree("Symbol") / tree(s))
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
                let mut t = tree("Rule");
                t.push_back(r);
                let mut pred = tree("Predicates");
                for i in p.iter().cloned() {
                    pred.push_back(i);
                }
                t.push_back(pred);
                if let Some(a) = a {
                    let mut attr = tree("Attributes");
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
            x:(@) _ "=>" _ y:@ { tree("=>")/x/y }
            x:(@) _ "<=>" _ y:@ { tree("<=>")/x/y }
            --
            x:(@) _ "||" _ y:@ { tree("||")/x/y }
            x:(@) _ "&&" _ y:@ { tree("&&")/x/y }
            --
            x:(@) _ "is" _ y:@ { tree("is")/x/y }
            x:(@) _ "in" _ y:@ { tree("in")/x/y }
            x:(@) _ "==" _ y:@ { tree("==")/x/y }
            x:(@) _ "!=" _ y:@ { tree("!=")/x/y }
            x:(@) _ "<=" _ y:@ { tree("<=")/x/y }
            x:(@) _ ">=" _ y:@ { tree(">=")/x/y }
            x:(@) _ "<" _ y:@ { tree("<")/x/y }
            x:(@) _ ">" _ y:@ { tree(">")/x/y }
            --
            x:(@) _ "as" _ y:ident() { tree("as")/x/y }
            --
            "-" _ x:@ { tree("-")/x }
            "+" _ x:@ { tree("+")/x }
            "!" _ x:@ { tree("!")/x }
            --
            x:(@) _ "+" _ y:@ { tree("+")/x/y }
            x:(@) _ "-" _ y:@ { tree("-")/x/y }
            --
            x:(@) _ "*" _ y:@ { tree("*")/x/y }
            x:(@) _ "/" _ y:@ { tree("/")/x/y }
            --
            x:@ "^" y:(@) { tree("^")/x/y }
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
            Ok(tree("sum") / (tree("+") / tree("one") / tree("three")) / tree("two"))
        );
    }

    #[test]
    fn bindings_test() {
        let test = "set(a, b) as S is Known";
        let states = ra::statements(test).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(
            states[0],
            tree("is") /
                (tree("as") / (tree("set") / tree("a") / tree("b")) / tree("S")) /
                tree("Known")
        )
    }

    #[test]
    fn predicate_parse_test() {
        let test = "x in Real; x is Unknown;";
        let states = ra::statements(test).unwrap();

        assert_eq!(states.len(), 2);
        assert_eq!(states[0], tree("in") / tree("x") / tree("Real"));
        assert_eq!(states[1], tree("is") / tree("x") / tree("Unknown"));
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
            tree("=>") /
                (tree("==") /
                    (tree("*") / tree("x") / (tree("+") / tree("y") / tree("z"))) /
                    tree("0")) /
                (tree("||") /
                    (tree("==") / (tree("+") / tree("y") / tree("z")) / tree("0")) /
                    (tree("==") / tree("x") / tree("0")))
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
            tree("Rule") /
                (tree("=>") /
                    (tree("==") /
                        (tree("+") / (tree("*") / tree("a") / tree("x")) / tree("b")) /
                        tree("0")) /
                    (tree("==") / tree("x") / (tree("/") / tree("b") / tree("a")))) /
                (tree("Predicates") / (tree("!=") / tree("a") / tree("0")))
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
            tree("Problem") /
                (tree("Target") / (tree("find") / tree("x"))) /
                (tree("==") /
                    (tree("+") / (tree("*") / tree("2") / tree("x")) / tree("5")) /
                    tree("0"))
        )
    }

    #[test]
    fn priority_test() {
        let test = r#"-a/b"#;
        let states = ra::statements(test).unwrap();
        assert_eq!(states[0], tree("-") / (tree("/") / tree("a") / tree("b")))
    }
}
