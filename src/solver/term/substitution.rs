use std::{collections::HashMap, fmt};

use indexmap::IndexMap;
use itertools::{Itertools, chain};

use super::{ArgList, Param, SharedTerm, TermBuf, Variable};

pub type VariableSubstitution = HashMap<Variable, TermBuf>;

#[derive(Debug, Clone, Default)]
pub struct ParamSubstitution {
    /// Shared (`Arc`) values: cloning a substitution bumps refcounts; the deep
    /// copy happens once, at the write site in `TermMut::substitute`.
    pub params:   IndexMap<Param, SharedTerm>,
    pub arglists: IndexMap<ArgList, Vec<TermBuf>>,
}

/// Applies a [`ParamSubstitution`] to a value containing parameters.
///
/// Implementors provide [`Substitute::substitute`] (in-place); chainable and
/// iterator-based variants come for free via default methods.
pub trait Substitute: Sized {
    fn substitute(&mut self, subst: &ParamSubstitution);

    fn substituted(mut self, subst: &ParamSubstitution) -> Self {
        self.substitute(subst);
        self
    }

    fn substitute_iter<I>(&mut self, pairs: I)
    where
        I: IntoIterator<Item = (Param, TermBuf)>,
    {
        self.substitute(&pairs.into_iter().collect());
    }

    fn substituted_iter<I>(self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (Param, TermBuf)>,
    {
        self.substituted(&pairs.into_iter().collect())
    }
}

impl Substitute for TermBuf {
    fn substitute(&mut self, subst: &ParamSubstitution) {
        self.term_mut().substitute(subst);
    }
}

impl FromIterator<(Param, TermBuf)> for ParamSubstitution {
    fn from_iter<I: IntoIterator<Item = (Param, TermBuf)>>(iter: I) -> Self {
        ParamSubstitution {
            params:   iter
                .into_iter()
                .map(|(p, t)| (p, SharedTerm::new(t)))
                .collect(),
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
