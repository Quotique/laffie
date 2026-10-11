use std::{convert::From, error::Error, fmt, io, path::Path};

use crate::Location;

type PegError = peg::error::ParseError<peg::str::LineCol>;

const LINES_AROUND: usize = 5;

/// A non-fatal load failure; `message` starts with the file it is about.
#[derive(Debug)]
pub struct LoadError {
    pub message: String,
}

/// `Corpus` lists every problem, not the first: `.sym` fixes go in one pass.
#[derive(Debug)]
pub enum LoadFailure {
    Io(io::Error),
    Corpus(Vec<LoadError>),
}

#[derive(Clone, Debug)]
pub struct ParserError {
    pub loc: Location,
    pub msg: String,
}

impl fmt::Display for LoadFailure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Corpus(errors) => {
                for e in errors {
                    writeln!(f, "{}", e.message)?;
                }
                write!(f, "{} load error(s)", errors.len())
            }
        }
    }
}

impl Error for LoadFailure {}

impl From<io::Error> for LoadFailure {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
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

impl Error for ParserError {}

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

        // Clamp: a row past the end (EOF) must not index out of bounds.
        let insert_at = self.loc.row.min(lines.len());
        lines.insert(
            insert_at,
            format!("{}^~~", " ".repeat(self.loc.col + count_len + 2)),
        );

        let first_line = insert_at.saturating_sub(LINES_AROUND);
        let last_line = std::cmp::min(lines.len(), insert_at + LINES_AROUND + 1);

        let at = match path {
            Some(path) => format!("{}:{}:{}", path.display(), self.loc.row, self.loc.col),
            None => format!("{}:{}", self.loc.row, self.loc.col),
        };
        format!(
            "{at}: {}\n{}",
            self.msg,
            lines[first_line..last_line].join("\n")
        )
    }
}
