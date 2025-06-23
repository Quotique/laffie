#[macro_use]
extern crate log;

use std::fmt;

mod dir_loader;
mod error;
mod grammar;
pub mod lang;
mod rule;
mod symbol;
mod task;
mod term;

pub type CompactString = smartstring::alias::String;

pub use self::{
    dir_loader::DirectoryParser, error::ParserError, rule::RuleParser, symbol::SymbolParser,
    task::TaskParser, term::TermParser,
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
    pub fn new_line_poses(text: &str) -> Vec<usize> {
        text.chars()
            .enumerate()
            .filter(|(_, c)| *c == '\n')
            .map(|(n, _)| n)
            .collect()
    }

    pub fn from_position(pos: usize, len: usize, new_line_poses: &[usize]) -> Self {
        let row = new_line_poses
            .iter()
            .enumerate()
            .find(|(_, x)| x > &&pos)
            .map(|(n, _)| n + 1)
            .unwrap_or_else(|| new_line_poses.len());
        let prev_row_end = if row > 1 {
            new_line_poses[row - 2] + 1
        } else {
            0
        };
        Self {
            col: pos - prev_row_end,
            row,
            len,
        }
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
        let location = Location::from_position(start, 4, &poses);

        let error = ParserError {
            loc: location.clone(),
            msg: "Error text".to_owned(),
        };

        println!("{}", error.error_string(test_str, None));

        assert_eq!(location.len, 4);
        assert_eq!(location.row, 1);
        assert_eq!(location.col, start);

        let start = test_str.find("elit").unwrap();
        let location = Location::from_position(start, 4, &poses);

        assert_eq!(location.len, 4);
        assert_eq!(location.row, 2);
        assert_eq!(location.col, start - 28);
    }
}
