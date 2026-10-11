use crate::{
    Location, ParserError,
    syntax::{Expr, ExprKind, Span},
};

/// Parsed text, for turning spans back into locations.
pub(crate) struct Source<'a> {
    text:           &'a str,
    new_line_poses: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            new_line_poses: Location::new_line_poses(text),
        }
    }

    pub fn slice(&self, span: Span) -> &'a str {
        &self.text[span.start..span.end]
    }

    /// Read as written: `1.0` is not a u64.
    pub fn uint(&self, span: Span) -> Result<u64, ParserError> {
        self.slice(span)
            .parse()
            .map_err(|_| self.error(span, "must be u64"))
    }

    /// A name or a string as itself, anything else as written.
    pub fn ident<'e>(&'e self, e: &'e Expr) -> &'e str {
        match &e.kind {
            ExprKind::Name(s) => s,
            ExprKind::Str(s) => s,
            _ => self.slice(e.span),
        }
    }

    pub fn error(&self, span: Span, msg: impl ToString) -> ParserError {
        ParserError {
            loc: self.loc(span),
            msg: msg.to_string(),
        }
    }

    pub fn loc(&self, span: Span) -> Location {
        Location::from_position(
            self.text,
            span.start,
            span.end - span.start,
            &self.new_line_poses,
        )
    }
}
