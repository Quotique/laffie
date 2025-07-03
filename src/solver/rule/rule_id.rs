use bincode::{Decode, Encode};
use derive_more::{Display, From};

#[derive(Clone, Copy, Debug, Default, Display)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(From)]
#[derive(Decode, Encode)]
pub struct RuleId(u64);

impl RuleId {
    pub fn new(mask: u64, id: u64) -> Self {
        Self(mask | id)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }
}
