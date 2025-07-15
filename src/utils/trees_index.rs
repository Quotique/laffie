use std::ops::{Index, IndexMut};

use derive_more::From;
use itertools::Itertools;
use trees::{Node, Tree};

#[derive(Debug, Default, Clone, From, PartialEq, Eq, Hash)]
pub struct TreeIndex(pub(super) Vec<usize>);

pub trait IndexedTree {
    type Value;

    fn get(&self, idx: &TreeIndex) -> Option<&Node<Self::Value>>;
    fn get_mut(&mut self, idx: &TreeIndex) -> Option<&mut Node<Self::Value>>;
    fn id(&self) -> TreeIndex;
}

impl<T: Unpin> IndexedTree for Node<T> {
    type Value = T;

    fn get(&self, idx: &TreeIndex) -> Option<&Node<Self::Value>> {
        let mut root = self;
        for i in idx.0.iter() {
            root = root.iter().nth(*i)?;
        }
        Some(root)
    }

    fn get_mut(&mut self, idx: &TreeIndex) -> Option<&mut Node<Self::Value>> {
        let mut root = self;
        for i in idx.0.iter() {
            root = root.iter_mut().nth(*i).map(|x| x.get_mut())?;
        }
        Some(root)
    }

    fn id(&self) -> TreeIndex {
        let mut current = self;
        let mut path = vec![];

        while let Some(parent) = current.parent() {
            let id = parent
                .iter()
                .find_position(|x| std::ptr::eq(*x, current))
                .map(|(num, _)| num)
                .expect("the parent must contains the child");
            path.push(id);
            current = parent;
        }
        path.reverse();
        path.into()
    }
}

impl<T: Unpin> IndexedTree for Tree<T> {
    type Value = T;

    fn get(&self, idx: &TreeIndex) -> Option<&Node<Self::Value>> {
        self.root().get(idx)
    }

    fn get_mut(&mut self, idx: &TreeIndex) -> Option<&mut Node<Self::Value>> {
        self.root_mut().get_mut().get_mut(idx)
    }

    fn id(&self) -> TreeIndex {
        self.root().id()
    }
}

impl<T> Index<&TreeIndex> for Tree<T> {
    type Output = Node<T>;

    fn index(&self, pos: &TreeIndex) -> &Self::Output {
        let mut root = self.root();
        for i in pos.0.iter() {
            root = root.iter().nth(*i).expect("Bad position");
        }
        root
    }
}

impl<T: Unpin> IndexMut<&TreeIndex> for Tree<T> {
    fn index_mut(&mut self, pos: &TreeIndex) -> &mut Self::Output {
        let mut root = self.root_mut().get_mut();
        for i in pos.0.iter() {
            let next_root = root.iter_mut().nth(*i).expect("Bad position").get_mut();
            root = next_root;
        }
        root
    }
}
