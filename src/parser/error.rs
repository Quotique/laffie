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
                row: value.location.line,
                col: value.location.column.saturating_sub(1),
            },
            msg: expected(value.expected.tokens()),
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

/// A missing closing delimiter tells most, then a missing expression.
fn expected(tokens: impl Iterator<Item = &'static str>) -> String {
    let mut tokens: Vec<_> = tokens.collect();
    // peg quotes a literal Rust-style; `expected!` text is a phrase or a keyword.
    let shown = |t: &str| match t {
        "EOF" => "end of input".to_owned(),
        t if t.starts_with('"') => {
            let literal = t[1..t.len() - 1]
                .replace("\\\"", "\"")
                .replace("\\\\", "\\");
            format!("`{literal}`")
        }
        t if t.contains(' ') => t.to_owned(),
        t => format!("`{t}`"),
    };
    if let Some(first) = [r#"")""#, r#""}""#, "an expression"]
        .into_iter()
        .find(|t| tokens.contains(t))
    {
        return format!("expected {}", shown(first));
    }
    tokens.sort_unstable_by_key(|&t| (t == "EOF", t));
    let shown: Vec<_> = tokens.into_iter().map(shown).collect();
    format!("expected {}", shown.join(" or "))
}

#[cfg(test)]
mod tests {
    use super::ParserError;
    use crate::{Names, syntax::grammar::parse, term};

    #[test]
    fn a_missing_delimiter_is_named_at_the_end_of_input() {
        let err = term("(x + 1", Names::Params).unwrap_err();
        assert_eq!(err.msg, "expected `)`");
        assert_eq!((err.loc.row, err.loc.col), (1, 6));
    }

    #[test]
    fn a_missing_operand_is_an_expression() {
        for text in ["x + ", "x^", "2^^3", "a == ", ""] {
            assert_eq!(message(text), "expected an expression", "{text}");
        }
    }

    #[test]
    fn a_word_after_a_term_fails_at_the_word() {
        let err = term("x inside", Names::Params).unwrap_err();
        assert_eq!((err.loc.row, err.loc.col), (1, 2));
    }

    #[test]
    fn no_message_lists_character_classes() {
        for text in [
            "(x + 1",
            "x + ",
            "x^",
            "x inside",
            "f(x",
            "set(..) as",
            "\"abc",
            "x +* y",
            "find(x) { x == 1",
        ] {
            let msg = message(text);
            assert!(!msg.contains("['") && !msg.contains("[_]"), "{text}: {msg}");
        }
        for text in ["rule a b {}", "rule a => {}", "task find(x) { x == ; }"] {
            let msg = ParserError::from(parse::file(text).unwrap_err()).msg;
            assert!(
                !msg.contains("['") && !msg.contains("unreported"),
                "{text}: {msg}"
            );
        }
    }

    #[test]
    fn a_block_in_the_old_form_names_the_missing_head() {
        for (text, msg) in [
            ("rule { attr level(0); a => b; }", "expected an expression"),
            (
                "task { goal find(x); x == 1; }",
                "expected a goal, like `find(x)`",
            ),
        ] {
            let err = ParserError::from(parse::file(text).unwrap_err());
            assert_eq!((err.msg.as_str(), err.loc.col), (msg, 5), "{text}");
        }
    }

    fn message(text: &str) -> String {
        term(text, Names::Params).unwrap_err().msg
    }
}
