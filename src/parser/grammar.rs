use std::convert::From;

use trees::{Tree, tr};

use crate::CompactString;

pub(crate) const TOKEN_TASK: &str = "Task";
pub(crate) const TOKEN_GOAL: &str = "Goal";
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

        rule quoted_char() -> char =
            "\\\"" { '\"' } / "\\\\" { '\\' } / !("\"" / "\\") c:$([_]) { c.chars().next().unwrap() }

        rule commasep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ",") ","? {v}

        rule semicolonsep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ";") ";"? {v}

        rule keyword(id: &'static str) =
            #{|input, pos| input.parse_string_literal(pos, id)} !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule reserved_word() =
            ("in" / "is" / "as") !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule ident() -> Tree<Token> =
            _ p:position!() !reserved_word()
            s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]+) { Token::new(s, p) }

        rule char_first_ident() -> Tree<Token> =
            _ p:position!() !reserved_word()
            s:$(['a'..='z' | 'A'..='Z']
                ['a'..='z' | 'A'..='Z' | '0'..='9' | '_' ]*) {
                    Token::new(s, p)
            }

        rule number() -> Tree<Token> =
            _ p:position!() n:$(['0'..='9']+("."['0'..='9']+)?) { Token::new(n, p) }

        rule minus_number() -> Tree<Token> = "-" _ p:position!() _ n:number() {
            Token::new(&format!("-{}", n.root().data().symbol), p)
        }

        rule arg_list() -> Tree<Token> = p:position!() ".." { Token::new("..", p) }

        rule attrs() -> Vec<Tree<Token>> = _ keyword("attr") _ a:commasep(<term()>) { a }

        pub rule any() -> Vec<Tree<Token>> =
            _ s:( (task()/lang_rule()/symbol())* ) _ { s }

        pub rule terms() -> Vec<Tree<Token>> = _ c:semicolonsep(<term()>) _ { c }

        pub rule task() -> Tree<Token> = _ pp:position!() keyword("task") _ "{"
                _ pt:position!() keyword("goal") _ p:eval() ";"
                _ tt:position!() t:(keyword("text") _ t:string() ";" {t} )?
                _ at:position!() a:(keyword("answer") _ a:commasep(<term()>) _ ";" {a} )?
                _ c:semicolonsep(<term()>)
            _ "}"
            {
                let mut p = Token::new(TOKEN_TASK, pp)
                              /(Token::new(TOKEN_GOAL, pt) / p)
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
            _ ps:position!() keyword("symbol") _ pn:position!()
                s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '*' | '/' |
                        '^' | '=' | '<' | '>' | '!' | '|' | '&' | '_' ]+)
                _ pa:position!() a:( "{"
                    _ at:attrs()? _ ";"?
                    _ pr:(keyword("py") _ pr:string() ";" {pr})?
                _ "}" { (at,pr) } )?
                ";"?
                {
                    let mut result = Token::new(TOKEN_DECLARE, ps)
                        / (Token::new(TOKEN_SYMBOL, ps) / Token::new(s, pn));

                    if let Some(a) = a {
                        if let Some(attr) = a.0 {
                            let mut t = Token::new("Attrs", pa);
                            for i in attr.into_iter() {
                                t.push_back(i);
                            }
                            result = result / t;
                        }
                        if let Some(program) = a.1 {
                            let prog = Token::new("PyProg", pa)
                                /Token::new(program.as_str(), pa);
                            result = result / prog;
                        }
                    }
                    result
                }

        pub rule lang_rule() -> Tree<Token> =
            _ rp:position!() keyword("rule") _ "{"
                _ ap:position!() a:(a:attrs() _ ";" {a})?
                _ r:term() _ ";"
                _ pp:position!() p:commasep(<term()>)
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

        rule predicate_op() -> &'input str =
            s:$("is" / "in" / "==" / "!=" / "<=" / ">=" / "<" / ">") { s }

        pub rule term() -> Tree<Token> =
            deduction()
            / or()

        rule deduction() -> Tree<Token> =
            l:or() _ p:position!() op:$("=>"/"<=>") _ r:or() { Token::new(op, p) /l /r}

        #[cache_left_rec]
        rule or() -> Tree<Token> =
            l:or() _ p:position!() "||" _ r:and() { Token::new("||", p) /l /r}
            / and()

        #[cache_left_rec]
        rule and() -> Tree<Token> =
            l:and() _ p:position!() "&&" _ r:predicate() { Token::new("&&", p) /l /r}
            / predicate()

        #[cache_left_rec]
        rule predicate() -> Tree<Token> =
            l:bind() _ p:position!() op:predicate_op() _ r:bind() { Token::new(op, p) /l /r}
            / bind()

        rule bind() -> Tree<Token> =
            l:sum() _ p:position!() "as" _ r:ident() { Token::new("as", p) /l /r}
            / sum()

        #[cache_left_rec]
        rule sum() -> Tree<Token> =
            l:sum() _ p:position!() "+" _ r:product() { Token::new("+", p) /l /r }
            / l:sum() _ p:position!() "+" _ n:number() _ r:product() { Token::new("+", p) /l /(Token::new("*", p) /n /r) }
            / l:sum() _ p:position!() _ n:minus_number() _ "*" _ r:product() { Token::new("+", p) /l /(Token::new("*", p) /n /r) }
            / l:sum() _ p:position!() _ n:minus_number() _ "/" _ r:product() { Token::new("+", p) /l /(Token::new("/", p) /n /r) }
            / l:sum() _ p:position!() _ n:minus_number() _ r:product() { Token::new("+", p) /l /(Token::new("*", p) /n /r) }
            / l:sum() _ p:position!() _ n:minus_number() { Token::new("+", p) /l /n }
            / l:sum() _ p:position!() "-" _ r:product() { Token::new("+", p) /l /(Token::new("*", p) /Token::new("-1", p) /r) }
            / unary()

        rule unary() -> Tree<Token> =
            p:position!() n:minus_number() _ "/" _ x:product() { Token::new("/", p) /n /x }
            / p:position!() n:minus_number() _ x:product() { Token::new("*", p) /n /x }
            / n:minus_number() { n }
            / p:position!() "-" _ x:product() { Token::new("*", p) /Token::new("-1", p) /x }
            / p:position!() "+" _ x:product() { Token::new("+", p) /x }
            / p:position!() "!" _ x:product() { Token::new("!", p) /x }
            / product()

        #[cache_left_rec]
        rule product() -> Tree<Token> =
            l:product() _ p:position!() "*" _ r:power() { Token::new("*", p) /l /r }
            / l:product() _ p:position!() "/" _ r:power() { Token::new("/", p) /l /r }
            / power()

        rule power() -> Tree<Token> =
            x:atom() _ p:position!() "^" y:atom() { Token::new("^", p) /x /y }
            / atom()

        rule atom() -> Tree<Token> =
            n:number() p:position!() e:eval() { Token::new("*", p) /n /e }
            / "(" _ b:or() _ ")" { b }
            / e:eval() { e }
            / p:arg_list() { p }
            / n:number() p:position!() i:char_first_ident() { Token::new("*", p) /n /i }
            / n:number() { n }
            / i:ident() { i }
            / p:position!() s:string() { Token::new(s.as_str(), p) }

        rule eval() -> Tree<Token> = t:char_first_ident() "(" _ a:commasep(<or()>) _ ")" {
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
    use crate::error::ParserError;

    fn terms_test(text: &str, expect: Vec<Tree<Token>>) {
        let states = ra::terms(text)
            .map_err(|e| println!("{}", ParserError::from(e).error_string(text, None)))
            .unwrap();
        assert_eq!(states, expect)
    }

    #[test]
    fn string_test() {
        assert_eq!(ra::string("\"sdfdsf\""), Ok(String::from("sdfdsf")));
    }

    #[test]
    fn args_test() {
        assert_eq!(
            ra::term("sum(one + three, two)"),
            Ok(Token::new("sum", 0) /
                (Token::new("+", 8) / Token::new("one", 4) / Token::new("three", 10)) /
                Token::new("two", 17))
        );
    }

    #[test]
    fn bindings_test() {
        terms_test(
            "set(a, b) as S is Known",
            vec![
                Token::new("is", 15) /
                    (Token::new("as", 10) /
                        (Token::new("set", 0) / Token::new("a", 4) / Token::new("b", 7)) /
                        Token::new("S", 13)) /
                    Token::new("Known", 18),
            ],
        );
    }

    #[test]
    fn predicate_parse_test() {
        terms_test(
            "x in Real; x is Unknown;",
            vec![
                Token::new("in", 2) / Token::new("x", 0) / Token::new("Real", 5),
                Token::new("is", 13) / Token::new("x", 11) / Token::new("Unknown", 16),
            ],
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
    fn priority_test() {
        terms_test(
            r#"-a/b"#,
            vec![
                Token::new("*", 0) /
                    Token::new("-1", 0) /
                    (Token::new("/", 2) / Token::new("a", 1) / Token::new("b", 3)),
            ],
        );

        terms_test(
            r#"-a+b"#,
            vec![
                Token::new("+", 2) /
                    (Token::new("*", 0) / Token::new("-1", 0) / Token::new("a", 1)) /
                    Token::new("b", 3),
            ],
        );
    }

    #[test]
    fn comment_test() {
        terms_test(
            r#"// test comment before
            -a/b // test comment near
            //test comment after"#,
            vec![
                Token::new("*", 35) /
                    Token::new("-1", 35) /
                    (Token::new("/", 37) / Token::new("a", 36) / Token::new("b", 38)),
            ],
        );
    }

    #[test]
    fn arglist_test() {
        terms_test(
            "set(..) as S is Known",
            vec![
                Token::new("is", 13) /
                    (Token::new("as", 8) /
                        (Token::new("set", 0) / Token::new("..", 4)) /
                        Token::new("S", 11)) /
                    Token::new("Known", 16),
            ],
        );
    }

    #[test]
    fn linear_test() {
        terms_test(
            r#"2*x-5+t == 0"#,
            vec![
                Token::new("==", 8) /
                    (Token::new("+", 5) /
                        (Token::new("+", 3) /
                            (Token::new("*", 1) / Token::new("2", 0) / Token::new("x", 2)) /
                            Token::new("-5", 4)) /
                        Token::new("t", 6)) /
                    Token::new("0", 11),
            ],
        );
    }

    #[test]
    fn polynom_test() {
        terms_test(
            r#"x^4 - 25*x^2 + 60*x - 36 != 0"#,
            vec![
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
                            (Token::new("*", 17) /
                                Token::new("60", 15) /
                                Token::new("x", 18))) /
                        Token::new("-36", 22)) /
                    Token::new("0", 28),
            ],
        );
    }

    #[test]
    fn short_mul_ident_test() {
        terms_test(
            r#"6x"#,
            vec![Token::new("*", 1) / Token::new("6", 0) / Token::new("x", 1)],
        );
    }

    #[test]
    fn short_mul_expr_test() {
        terms_test(
            r#"6sin(x)"#,
            vec![
                Token::new("*", 1) /
                    Token::new("6", 0) /
                    (Token::new("sin", 1) / Token::new("x", 5)),
            ],
        );
    }

    #[test]
    fn fraction_test() {
        terms_test(
            r#"x == -5/3"#,
            vec![
                Token::new("==", 2) /
                    Token::new("x", 0) /
                    (Token::new("/", 5) / Token::new("-5", 6) / Token::new("3", 8)),
            ],
        );
    }

    #[test]
    fn constant_in_test() {
        terms_test(
            r#"3 in set(1, 3)"#,
            vec![
                Token::new("in", 2) /
                    Token::new("3", 0) /
                    (Token::new("set", 5) / Token::new("1", 9) / Token::new("3", 12)),
            ],
        );
    }

    #[test]
    fn decimal_fraction_test() {
        terms_test(
            r#"2.1sin(x)"#,
            vec![
                Token::new("*", 3) /
                    Token::new("2.1", 0) /
                    (Token::new("sin", 3) / Token::new("x", 7)),
            ],
        );

        terms_test(
            r#"2.1/3.5"#,
            vec![Token::new("/", 3) / Token::new("2.1", 0) / Token::new("3.5", 4)],
        );
    }

    #[test]
    fn task_text_parse_test() {
        let test = r#"task {
                        goal find(x);
                        text "Решите уравнение 2x+5 = 0";
                        answer x == -2.5;
                        2*x+5 == 0;
                    }"#;
        let states = ra::task(test).unwrap();
        assert_eq!(
            states,
            Token::new("Task", 0) /
                (Token::new("Goal", 31) / (Token::new("find", 36) / Token::new("x", 41))) /
                (Token::new("Text", 69) / Token::new("Решите уравнение 2x+5 = 0", 69)) /
                (Token::new("Answer", 142) /
                    (Token::new("==", 151) / Token::new("x", 149) / Token::new("-2.5", 155))) /
                (Token::new("==", 190) /
                    (Token::new("+", 187) /
                        (Token::new("*", 185) /
                            Token::new("2", 184) /
                            Token::new("x", 186)) /
                        Token::new("5", 188)) /
                    Token::new("0", 193))
        )
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
                        goal find(x);
                        2*x+5 == 0;
                    }"#;
        let states = ra::task(test).unwrap();
        assert_eq!(
            states,
            Token::new("Task", 0) /
                (Token::new("Goal", 31) / (Token::new("find", 36) / Token::new("x", 41))) /
                (Token::new("Text", 69) / Token::new("", 69)) /
                (Token::new("==", 75) /
                    (Token::new("+", 72) /
                        (Token::new("*", 70) / Token::new("2", 69) / Token::new("x", 71)) /
                        Token::new("5", 73)) /
                    Token::new("0", 78))
        )
    }
}
