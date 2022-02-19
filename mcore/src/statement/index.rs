use std::ops::{Index, IndexMut};

use bincode::{Decode, Encode};

use super::{statement::Statement, term::StatementNode};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[derive(Decode, Encode)]
pub struct NodePosition {
    coordinates: Vec<usize>,
}

impl NodePosition {
    pub fn root() -> Self {
        Self {
            coordinates: vec![],
        }
    }

    pub fn child(mut self, num: usize) -> Self {
        self.coordinates.push(num);
        self
    }
}

impl IndexMut<&NodePosition> for Statement {
    fn index_mut(&mut self, pos: &NodePosition) -> &mut Self::Output {
        let mut root = self.tree.root_mut().get_mut();
        for i in pos.coordinates.iter() {
            let next_root = root.iter_mut().nth(*i).expect("Bad position").get_mut();
            root = next_root;
        }
        root
    }
}

impl Index<&NodePosition> for Statement {
    type Output = StatementNode;

    fn index(&self, pos: &NodePosition) -> &Self::Output {
        let mut root = self.tree.root();
        for i in pos.coordinates.iter() {
            root = root.iter().nth(*i).expect("Bad position");
        }
        root
    }
}
