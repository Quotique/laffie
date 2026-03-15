use std::{collections::HashMap, fmt};

use indexmap::IndexMap;
use itertools::{Itertools, chain};

use super::{ArgList, Param, TermBuf, Variable};

pub type VariableSubstitution = HashMap<Variable, TermBuf>;

#[derive(Debug, Clone, Default)]
pub struct ParamSubstitution {
    pub params:   IndexMap<Param, TermBuf>,
    pub arglists: IndexMap<ArgList, Vec<TermBuf>>,
}

impl FromIterator<(Param, TermBuf)> for ParamSubstitution {
    fn from_iter<I: IntoIterator<Item = (Param, TermBuf)>>(iter: I) -> Self {
        ParamSubstitution {
            params:   FromIterator::from_iter(iter),
            arglists: Default::default(),
        }
    }
}

impl fmt::Display for ParamSubstitution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{ {} }}",
            chain(
                self.params.iter().map(|(x, y)| format!("{x}: {y}")),
                self.arglists
                    .iter()
                    .map(|(x, y)| format!("..{x}: [{}]", y.iter().format(", ")))
            )
            .format(", ")
        )
    }
}
