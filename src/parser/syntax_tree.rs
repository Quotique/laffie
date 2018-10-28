use std::fmt::{Debug, Formatter, Error};

pub struct Node {
    pub symbol_id: i32,
    pub childs: Vec<Box<node>>,
}

impl Debug for Node {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        write!(fmt, "");
    }
}
