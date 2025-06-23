use crate::term::Subterm;

pub struct TruthChecker(pub Box<dyn Fn(Subterm) -> TruthResult + Sync + Send>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TruthResult {
    True,
    False,
    Unknown,
}

impl TruthResult {
    #[inline]
    pub fn is_true(&self) -> bool {
        self == &TruthResult::True
    }

    #[inline]
    pub fn reverse(&self) -> TruthResult {
        match self {
            TruthResult::True => TruthResult::False,
            TruthResult::False => TruthResult::True,
            TruthResult::Unknown => TruthResult::Unknown,
        }
    }
}
