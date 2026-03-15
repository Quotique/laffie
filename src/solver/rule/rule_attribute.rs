use std::str::FromStr;

use eyre::{Result, bail};

use crate::{CompactString, term::TermBuf};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuleAttr {
    Equivalence,
    Replace,
    Goal,
    Level,
    Zero,
    One,
    Block,
    Id,
    Normalize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleAttrValue {
    None,
    UInt(u64),
    Str(CompactString),
    Goal(TermBuf),
}

impl FromStr for RuleAttr {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "equivalence" => Ok(RuleAttr::Equivalence),
            "replace" => Ok(RuleAttr::Replace),
            "level" => Ok(RuleAttr::Level),
            "goal" => Ok(RuleAttr::Goal),
            "zero" => Ok(RuleAttr::Zero),
            "one" => Ok(RuleAttr::One),
            "id" => Ok(RuleAttr::Id),
            "block" => Ok(RuleAttr::Block),
            "normalize" => Ok(RuleAttr::Normalize),
            _ => bail!(""),
        }
    }
}

impl RuleAttrValue {
    pub fn str(&self) -> Option<&str> {
        if let RuleAttrValue::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn uint(&self) -> Option<u64> {
        if let RuleAttrValue::UInt(u) = self {
            Some(*u)
        } else {
            None
        }
    }
}
