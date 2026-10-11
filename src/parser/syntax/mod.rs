//! What the grammar reads, before names are resolved: the symbol registry is
//! filled only after every file is parsed, so the grammar cannot build terms.

pub(crate) mod grammar;

use solver::{CompactString, Rational};

/// Byte offsets into the source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub start: usize,
    pub end:   usize,
}

/// An operation's span is its operator's.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ExprKind {
    Num(Rational),
    Name(CompactString),
    // TODO: a name until strings get a term of their own; the corpus relies on it.
    Str(String),
    /// `..`
    ArgList,
    Call {
        name: CompactString,
        args: Vec<Expr>,
    },
    /// `find(vars..) { givens.. }`
    Solve {
        vars:   Vec<Expr>,
        givens: Vec<Expr>,
    },
    /// Parentheses: transparent, but a `-` outside does not reach a literal inside.
    Group(Box<Expr>),
    Neg(Box<Expr>),
    Pos(Box<Expr>),
    Not(Box<Expr>),
    Bin {
        op:  BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `expr as Name`
    Bind {
        expr: Box<Expr>,
        name: CompactString,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    /// `2x`, `2(x+1)`, `(a)(b)`
    Juxt,
    Pow,
    And,
    Or,
    Cmp(CmpOp),
    Deduction(DeductionOp),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CmpOp {
    Is,
    In,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeductionOp {
    Implies,
    Equiv,
}

/// A top-level item of a `.sym` or `.pbl` file. Attributes stay expressions until
/// `build` checks them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Item {
    Task(Task),
    Rule(Rule),
    Symbol(Symbol),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Task {
    pub span:    Span,
    pub goal:    Expr,
    pub name:    Option<String>,
    pub text:    String,
    pub answers: Vec<Expr>,
    pub givens:  Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rule {
    pub span:         Span,
    pub attrs:        Vec<Expr>,
    pub pattern:      Expr,
    pub op:           DeductionOp,
    pub op_span:      Span,
    pub resolution:   Expr,
    pub requirements: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Symbol {
    pub span:  Span,
    pub name:  CompactString,
    pub attrs: Vec<Expr>,
    pub py:    Option<PySource>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PySource {
    pub code: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JuxtLeft {
    Number,
    Paren,
    No,
}

impl Expr {
    pub fn new(kind: ExprKind, start: usize, end: usize) -> Self {
        Self {
            kind,
            span: Span { start, end },
        }
    }

    pub fn bin(op: BinOp, lhs: Expr, rhs: Expr, span: Span) -> Self {
        Self {
            kind: ExprKind::Bin {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        }
    }

    /// `f` or `f(a, …)` as a name and its arguments.
    pub fn as_call(&self) -> Option<(&str, &[Expr])> {
        match &self.kind {
            ExprKind::Name(name) => Some((name, &[])),
            ExprKind::Call { name, args } => Some((name, args)),
            _ => None,
        }
    }

    /// What this operand admits juxtaposed on its right: `f(x)` stays a call, `x y` fails.
    pub fn juxt_left(&self) -> JuxtLeft {
        match &self.kind {
            ExprKind::Num(_) => JuxtLeft::Number,
            ExprKind::Group(_) => JuxtLeft::Paren,
            ExprKind::Bin {
                op: BinOp::Pow | BinOp::Juxt,
                rhs,
                ..
            } => rhs.juxt_left(),
            _ => JuxtLeft::No,
        }
    }
}

impl BinOp {
    /// The symbol this operation becomes in a term.
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add | BinOp::Sub => "+",
            BinOp::Mul | BinOp::Juxt => "*",
            BinOp::Div => "/",
            BinOp::Pow => "^",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::Cmp(op) => op.symbol(),
            BinOp::Deduction(DeductionOp::Implies) => "=>",
            BinOp::Deduction(DeductionOp::Equiv) => "<=>",
        }
    }
}

impl CmpOp {
    pub fn symbol(self) -> &'static str {
        match self {
            CmpOp::Is => "is",
            CmpOp::In => "in",
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Le => "<=",
            CmpOp::Ge => ">=",
            CmpOp::Lt => "<",
            CmpOp::Gt => ">",
        }
    }
}
