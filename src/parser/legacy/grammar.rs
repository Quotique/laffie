use std::convert::From;

use trees::{Tree, tr};

use crate::CompactString;

pub(crate) const TOKEN_TASK: &str = "Task";
pub(crate) const TOKEN_GOAL: &str = "Goal";
pub(crate) const TOKEN_TEXT: &str = "Text";
pub(crate) const TOKEN_ANSWER: &str = "Answer";
pub(crate) const TOKEN_ID: &str = "Id";
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
                _ it:position!() i:(keyword("id") _ i:string() ";" {i} )?
                _ tt:position!() t:(keyword("text") _ t:string() ";" {t} )?
                _ at:position!() a:(keyword("answer") _ a:commasep(<term()>) _ ";" {a} )?
                _ c:semicolonsep(<term()>)
            _ "}"
            {
                let mut p = Token::new(TOKEN_TASK, pp)
                              /(Token::new(TOKEN_GOAL, pt) / p)
                              /(Token::new(TOKEN_TEXT, tt)
                                /Token::new(t.unwrap_or_default().as_str(), tt));
                if let Some(id) = i {
                    p.push_back(Token::new(TOKEN_ID, it) / Token::new(id.as_str(), it));
                }
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
            / l:sum() _ p:position!() "+" _ r:signed_product() { Token::new("+", p) /l /r }
            / l:sum() _ p:position!() r:signed_product() { Token::new("+", p) /l /r }
            / l:sum() _ p:position!() "-" _ r:product() { Token::new("+", p) /l /(Token::new("*", p) /Token::new("-1", p) /r) }
            / unary()

        rule unary() -> Tree<Token> =
            signed_product()
            / p:position!() "-" _ x:product() { Token::new("*", p) /Token::new("-1", p) /x }
            / p:position!() "+" _ x:product() { Token::new("+", p) /x }
            / p:position!() "!" _ x:product() { Token::new("!", p) /x }
            / product()

        #[cache_left_rec]
        rule product() -> Tree<Token> =
            l:product() _ p:position!() "*" _ r:power() { Token::new("*", p) /l /r }
            / l:product() _ p:position!() "/" _ r:power() { Token::new("/", p) /l /r }
            / power()

        // A product led by a negative literal: `-2*x/y` is `((-2)*x)/y`.
        #[cache_left_rec]
        rule signed_product() -> Tree<Token> =
            l:signed_product() _ p:position!() "*" _ r:power() { Token::new("*", p) /l /r }
            / l:signed_product() _ p:position!() "/" _ r:power() { Token::new("/", p) /l /r }
            / signed_factor()

        // The literal binds weaker than `^`: `-2^2` is `-(2^2)`, parsed by `unary`.
        rule signed_factor() -> Tree<Token> =
            p:position!() n:minus_number() !(_ "^") _ x:power() { Token::new("*", p) /n /x }
            / n:minus_number() !(_ "^") { n }

        rule power() -> Tree<Token> =
            x:atom() _ p:position!() "^" y:atom() { Token::new("^", p) /x /y }
            / atom()

        rule atom() -> Tree<Token> =
            n:number() p:position!() e:eval() { Token::new("*", p) /n /e }
            / "(" _ b:or() _ ")" { b }
            / f:solve_block() { f }
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

        // `find(vars...) { eqs... }` → `solve(find(vars...), eqs...)`.
        rule solve_block() -> Tree<Token> =
            _ p:position!() keyword("find")
            _ pf:position!() "(" _ vars:commasep(<or()>) _ ")"
            _ "{" _ givens:terms() _ "}"
            {
                let mut find = Token::new("find", pf);
                for v in vars.iter().cloned() {
                    find.push_back(v);
                }
                let mut t = Token::new("solve", p);
                t.push_back(find);
                for g in givens.iter().cloned() {
                    t.push_back(g);
                }
                t
            }
    }
}
