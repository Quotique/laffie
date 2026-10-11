use crate::{
    Location, ParserError,
    syntax::{Expr, ExprKind, Span},
};

/// Parsed text, for turning spans back into locations.
pub(crate) struct Source {
    text:     String,
    /// Byte offsets of every `\n`, matching `peg`'s byte `position!()`.
    newlines: Vec<usize>,
}

impl Source {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let newlines = text
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(n, _)| n)
            .collect();
        Self { text, newlines }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn slice(&self, span: Span) -> &str {
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

    /// 1-based `row`, char-counted `col`.
    pub fn loc(&self, span: Span) -> Location {
        let pos = span.start;
        let row = self
            .newlines
            .iter()
            .position(|&n| n > pos)
            .map_or(self.newlines.len() + 1, |n| n + 1);
        let line_start = if row > 1 {
            self.newlines[row - 2] + 1
        } else {
            0
        };
        let col = self
            .text
            .get(line_start..pos)
            .map_or(0, |line| line.chars().count());
        Location { row, col }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newlines_are_byte_offsets() {
        for (text, newlines) in [
            ("", vec![]),
            ("abacaba", vec![]),
            ("aba\ncaba", vec![3]),
            ("aba\ncaba\n\nlolkek", vec![3, 8, 9]),
            // Byte offsets, not char indices: "абв" is 6 bytes, "текст" is 10.
            ("абв\nгд", vec![6]),
            ("текст\nx", vec![10]),
        ] {
            assert_eq!(Source::new(text).newlines, newlines);
        }
    }

    #[test]
    fn a_location_is_the_row_and_col_of_its_offset() {
        let text = "Lorem ipsum dolor sit amet,\n  consectetur adipiscing elit,";
        assert_eq!(at(text, text.find("amet").unwrap()), (1, 22));
        assert_eq!(at(text, text.find("elit").unwrap()), (2, 25));
    }

    #[test]
    fn column_counts_chars_not_bytes() {
        let text = "переменная = 5";
        assert_eq!(at(text, text.find('5').unwrap()), (1, 13));
    }

    #[test]
    fn error_after_cyrillic_line_keeps_row_and_col() {
        // Row must not drift on multi-byte glyphs in an earlier line.
        let text = "текст фыва\nx + 2";
        assert_eq!(at(text, text.find('+').unwrap()), (2, 2));
    }

    #[test]
    fn last_line_position_gets_the_last_row() {
        let text = "a\nb\nc";
        assert_eq!(at(text, text.rfind('c').unwrap()), (3, 0));
    }

    fn at(text: &str, pos: usize) -> (usize, usize) {
        let loc = Source::new(text).loc(Span {
            start: pos,
            end:   pos + 1,
        });
        (loc.row, loc.col)
    }
}
