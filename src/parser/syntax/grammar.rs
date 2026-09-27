use solver::from_decimal_str;

use super::{BinOp, CmpOp, DedOp, Expr, ExprKind, JuxtLeft, Span};

peg::parser! {
    pub(crate) grammar syn() for str {
        use peg::ParseLiteral;

        rule line_comment() = "//" (!"\n" [_])* ("\n" / ![_])

        rule _() = quiet!{ (['\t' | '\r' | ' ' | '\n'] / line_comment())* }

        rule commasep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ",") ","? { v }

        rule semicolonsep<T>(x: rule<T>) -> Vec<T> = v:( (_ a:x() { a }) ** ";") ";"? { v }

        rule word_end() = !['0'..='9' | 'a'..='z' | 'A'..='Z' | '_']

        rule keyword(id: &'static str) =
            #{|input, pos| input.parse_string_literal(pos, id)} word_end()

        rule reserved_word() = ("in" / "is" / "as") word_end()

        pub rule one_term() -> Expr = _ t:term() _ (";" _)? { t }

        /// `a`, or one `a => b` / `a <=> b`: a deduction does not chain.
        pub rule term() -> Expr =
            l:logic() d:(_ p:position!() o:ded_op() _ r:logic() { (p, o, r) })? {
                match d {
                    Some((p, o, r)) => Expr::bin(BinOp::Ded(o), l, r, op_span(p, BinOp::Ded(o))),
                    None => l,
                }
            }

        rule ded_op() -> DedOp = "=>" { DedOp::Implies } / "<=>" { DedOp::Equiv }

        rule logic() -> Expr = precedence!{
            x:(@) _ p:position!() "||" _ y:@ { Expr::bin(BinOp::Or, x, y, op_span(p, BinOp::Or)) }
            --
            x:(@) _ p:position!() "&&" _ y:@ { Expr::bin(BinOp::And, x, y, op_span(p, BinOp::And)) }
            --
            // Below `&&`, above the comparisons: `!a == b` is `!(a == b)`.
            p:position!() "!" _ y:@ { Expr::new(ExprKind::Not(Box::new(y)), p, p + 1) }
            --
            c:compare() { c }
        }

        /// A comparison does not chain: `a == b == c` fails.
        rule compare() -> Expr =
            l:bind() t:(_ p:position!() o:cmp_op() _ r:bind() { (p, o, r) })? {
                match t {
                    Some((p, o, r)) => Expr::bin(BinOp::Cmp(o), l, r, op_span(p, BinOp::Cmp(o))),
                    None => l,
                }
            }

        rule cmp_op() -> CmpOp =
            keyword("is") { CmpOp::Is } / keyword("in") { CmpOp::In }
            / "==" { CmpOp::Eq } / "!=" { CmpOp::Ne } / "<=" { CmpOp::Le } / ">=" { CmpOp::Ge }
            / "<" { CmpOp::Lt } / ">" { CmpOp::Gt }

        rule bind() -> Expr =
            e:arith() n:(_ keyword("as") _ s:position!() i:name() { (s, i) })? {
                match n {
                    Some((s, name)) => {
                        let span = Span { start: s, end: s + name.len() };
                        Expr { kind: ExprKind::Bind { expr: Box::new(e), name }, span }
                    }
                    None => e,
                }
            }

        rule arith() -> Expr = precedence!{
            x:(@) _ p:position!() "+" _ y:@ { Expr::bin(BinOp::Add, x, y, op_span(p, BinOp::Add)) }
            x:(@) _ p:position!() "-" _ y:@ { Expr::bin(BinOp::Sub, x, y, op_span(p, BinOp::Sub)) }
            --
            s:signed_product() { s }
        }

        // Outside the table: a prefix there fires at every operand, and a
        // leading sign takes the whole product, `-a/b` is `-(a/b)`.
        rule signed_product() -> Expr =
            p:position!() "-" _ s:signed_product() { Expr::new(ExprKind::Neg(Box::new(s)), p, p + 1) }
            / p:position!() "+" _ x:product() { Expr::new(ExprKind::Pos(Box::new(x)), p, p + 1) }
            / product()

        rule product() -> Expr =
            f:factor() r:(_ p:position!() o:mul_op() _ x:factor() { (p, o, x) })* {
                r.into_iter().fold(f, |l, (p, o, x)| Expr::bin(o, l, x, op_span(p, o)))
            }

        rule mul_op() -> BinOp = "*" { BinOp::Mul } / "/" { BinOp::Div }

        /// `a * -b`
        rule factor() -> Expr =
            p:position!() "-" _ f:factor() { Expr::new(ExprKind::Neg(Box::new(f)), p, p + 1) }
            / juxt()

        /// Tighter than `*`: `a/2x` is `a/(2x)`.
        rule juxt() -> Expr =
            f:power() t:juxt_tail(f.juxt_left()) {
                t.into_iter().fold(f, |l, (p, r)| Expr::bin(BinOp::Juxt, l, r, Span { start: p, end: p }))
            }

        rule juxt_tail(left: JuxtLeft) -> Vec<(usize, Expr)> =
            _ p:position!() juxt_rhs_start(left) !reserved_word() r:power() t:juxt_tail(r.juxt_left()) {
                let mut v = vec![(p, r)];
                v.extend(t);
                v
            }
            / { Vec::new() }

        /// After a number: a parenthesis, a name or a call; after a parenthesis:
        /// only a parenthesis. Never a number: `2 3` fails.
        rule juxt_rhs_start(left: JuxtLeft) = #{|input, pos| {
            let next = input[pos..].chars().next();
            let admits = match left {
                JuxtLeft::Number => next.is_some_and(|c| c == '(' || c == '_' || c.is_ascii_alphabetic()),
                JuxtLeft::Paren => next == Some('('),
                JuxtLeft::No => false,
            };
            if admits { peg::RuleResult::Matched(pos, ()) } else { peg::RuleResult::Failed }
        }}

        /// Tighter than juxtaposition: `2x^2` is `2*(x^2)`.
        rule power() -> Expr =
            b:primary() e:(_ p:position!() "^" _ e:exponent() { (p, e) })? {
                match e {
                    Some((p, e)) => Expr::bin(BinOp::Pow, b, e, op_span(p, BinOp::Pow)),
                    None => b,
                }
            }

        /// Right-associative and signed: `2^3^2`, `x^-1`.
        rule exponent() -> Expr =
            p:position!() "-" _ e:exponent() { Expr::new(ExprKind::Neg(Box::new(e)), p, p + 1) }
            / power()

        rule primary() -> Expr =
            s:position!() "(" _ e:logic() _ ")" end:position!() {
                Expr::new(ExprKind::Group(Box::new(e)), s, end)
            }
            / solve_block()
            / call()
            / s:position!() ".." end:position!() { Expr::new(ExprKind::ArgList, s, end) }
            / num()
            / s:position!() n:name() end:position!() { Expr::new(ExprKind::Name(n), s, end) }
            / s:position!() t:string() end:position!() { Expr::new(ExprKind::Str(t), s, end) }

        rule num() -> Expr =
            s:position!() n:$(['0'..='9']+ ("." ['0'..='9']+)?) end:position!() {
                let value = from_decimal_str(n).expect("the grammar admits only decimal digits");
                Expr::new(ExprKind::Num(value), s, end)
            }

        rule name() -> solver::CompactString =
            !reserved_word() n:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '_']+) { n.into() }

        rule call() -> Expr =
            s:position!() n:$(['a'..='z' | 'A'..='Z'] ['a'..='z' | 'A'..='Z' | '0'..='9' | '_']*)
            "(" _ args:commasep(<logic()>) _ ")" end:position!() {
                Expr::new(ExprKind::Call { name: n.into(), args }, s, end)
            }

        rule solve_block() -> Expr =
            s:position!() keyword("find") _ "(" _ vars:commasep(<logic()>) _ ")"
            _ "{" _ givens:semicolonsep(<term()>) _ "}" end:position!() {
                Expr::new(ExprKind::Solve { vars, givens }, s, end)
            }

        rule string() -> String = "\"" s:quoted_char()* "\"" { s.into_iter().collect() }

        rule quoted_char() -> char =
            "\\\"" { '\"' } / "\\\\" { '\\' }
            / !("\"" / "\\") c:$([_]) { c.chars().next().expect("one char") }
    }
}

fn op_span(start: usize, op: BinOp) -> Span {
    Span {
        start,
        end: start + op.symbol().len(),
    }
}
