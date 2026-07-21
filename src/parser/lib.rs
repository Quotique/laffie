#[macro_use]
extern crate log;

use std::fmt;

mod dir_loader;
mod error;
mod grammar;
pub mod lang;
mod py_term;
mod rule;
mod symbol;
mod task;
mod term;

pub type CompactString = smartstring::alias::String;

pub use self::{
    dir_loader::{DirectoryParser, LoadError, LoadReport},
    error::ParserError,
    rule::RuleParser,
    symbol::SymbolParser,
    task::TaskParser,
    term::TermParser,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Location {
    pub col: usize,
    pub row: usize,
    pub len: usize,
}

#[derive(Hash, PartialEq, Eq)]
pub struct NodeData {
    pub symbol:   CompactString,
    pub location: Location,
}

pub type Tree = trees::Tree<NodeData>;
pub type Node = trees::Node<NodeData>;

impl fmt::Display for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl Location {
    /// Byte offsets of every `\n` — matching `peg`'s byte `position!()`.
    pub fn new_line_poses(text: &str) -> Vec<usize> {
        text.bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(n, _)| n)
            .collect()
    }

    /// Location of byte offset `pos`: 1-based `row`, char-counted `col`.
    pub fn from_position(text: &str, pos: usize, len: usize, new_line_poses: &[usize]) -> Self {
        let row = new_line_poses
            .iter()
            .enumerate()
            .find(|(_, x)| x > &&pos)
            .map(|(n, _)| n + 1)
            .unwrap_or(new_line_poses.len() + 1);
        let line_start = if row > 1 {
            new_line_poses[row - 2] + 1
        } else {
            0
        };
        let col = text
            .get(line_start..pos)
            .map(|slice| slice.chars().count())
            .unwrap_or(0);
        Self { col, row, len }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_line_poses_test() {
        for (test_str, poses) in &[
            ("", vec![]),
            ("abacaba", vec![]),
            ("aba\ncaba", vec![3]),
            ("aba\ncaba\n\nlolkek", vec![3, 8, 9]),
            // Byte offsets, not char indices: "абв" is 6 bytes, "текст" is 10.
            ("абв\nгд", vec![6]),
            ("текст\nx", vec![10]),
        ] {
            assert_eq!(&Location::new_line_poses(test_str), poses);
        }
    }

    #[test]
    fn location_from_pos_test() {
        let test_str = r#"Lorem ipsum dolor sit amet,
                          consectetur adipiscing elit,
                          sed do eiusmod tempor incididunt
                          ut labore et dolore magna aliqua."#;

        let poses = Location::new_line_poses(test_str);

        let start = test_str.find("amet").unwrap();
        let location = Location::from_position(test_str, start, 4, &poses);

        let error = ParserError {
            loc: location.clone(),
            msg: "Error text".to_owned(),
        };

        println!("{}", error.error_string(test_str, None));

        assert_eq!(location.len, 4);
        assert_eq!(location.row, 1);
        assert_eq!(location.col, start);

        let start = test_str.find("elit").unwrap();
        let location = Location::from_position(test_str, start, 4, &poses);

        assert_eq!(location.len, 4);
        assert_eq!(location.row, 2);
        assert_eq!(location.col, start - 28);
    }

    #[test]
    fn column_counts_chars_not_bytes() {
        // col must be char count (13), not byte offset (23).
        let text = "переменная = 5";
        let poses = Location::new_line_poses(text);
        let pos = text.find('5').unwrap();
        let location = Location::from_position(text, pos, 1, &poses);
        assert_eq!(location.row, 1);
        assert_eq!(location.col, 13);
    }

    #[test]
    fn error_after_cyrillic_line_keeps_row_and_col() {
        // Row must not drift on multi-byte glyphs in an earlier line.
        let text = "текст фыва\nx + 2";
        let poses = Location::new_line_poses(text);
        let pos = text.find('+').unwrap();
        let location = Location::from_position(text, pos, 1, &poses);
        assert_eq!(location.row, 2);
        assert_eq!(location.col, 2); // "x " then '+'
    }

    #[test]
    fn last_line_position_gets_the_last_row() {
        // Error on the last line: row = new_lines + 1.
        let text = "a\nb\nc";
        let poses = Location::new_line_poses(text);
        let pos = text.rfind('c').unwrap();
        let location = Location::from_position(text, pos, 1, &poses);
        assert_eq!(location.row, 3);
        assert_eq!(location.col, 0);
    }

    #[test]
    fn error_string_row_past_end_does_not_panic() {
        let text = "a\nb\nc";
        let error = ParserError {
            loc: Location {
                row: 999,
                col: 0,
                len: 0,
            },
            msg: "boom".to_owned(),
        };
        let _ = error.error_string(text, None); // must not panic
    }
}
