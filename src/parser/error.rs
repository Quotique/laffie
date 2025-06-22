use std::{convert::From, error::Error, fmt, path::Path};

use crate::Location;

type PegError = peg::error::ParseError<peg::str::LineCol>;

const LINES_AROUND: usize = 5;

#[derive(Clone, Debug)]
pub struct ParserError {
    pub loc: Location,
    pub msg: String,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "error at {}:{}: {}",
            self.loc.row, self.loc.col, self.msg
        )
    }
}

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<PegError> for ParserError {
    fn from(value: PegError) -> Self {
        Self {
            loc: Location {
                col: if value.location.column > 0 {
                    value.location.column - 1
                } else {
                    0
                },
                row: value.location.line,
                len: value.location.offset,
            },
            msg: format!("expected: {}", value.expected),
        }
    }
}

impl ParserError {
    pub fn error_string(&self, src_text: &str, path: Option<&Path>) -> String {
        let count_len = src_text.lines().count().to_string().len();
        let mut lines: Vec<_> = src_text
            .lines()
            .map(|x| x.to_owned())
            .enumerate()
            .map(|(n, s)| format!("{:>count_len$}: {s}", n + 1))
            .collect();

        lines.insert(
            self.loc.row,
            format!(
                "{}^~~: {}",
                " ".repeat(self.loc.col + count_len + 2),
                self.msg
            ),
        );

        let first_line = self.loc.row.saturating_sub(LINES_AROUND);
        let last_line = std::cmp::min(lines.len(), self.loc.row + LINES_AROUND);

        format!(
            "\n{}:{}:{}: error: parsing error\n{}",
            path.map(|x| x.to_string_lossy().to_string())
                .unwrap_or_default(),
            self.loc.row,
            self.loc.col,
            lines[first_line..last_line].join("\n")
        )
    }
}
