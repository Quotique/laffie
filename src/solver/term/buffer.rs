use std::{collections::HashSet, convert::From, fmt, sync::Arc};

use derive_more::From;
use serde::{Serialize, Serializer};
use serde_derive::Deserialize;
use trees::Tree;

use super::{Atom, Symbol, TermMut, TermRef, sym};
use crate::{
    CompactString, Decimal, NormLevel,
    term::{Param, Variable},
};

type SymbolTree = Tree<Atom>;

pub type SharedTerm = Arc<TermBuf>;

#[derive(Debug, Default, Clone, From, PartialEq, Eq, Hash)]
pub struct TermPath(pub(super) Vec<usize>);

impl std::ops::Deref for TermPath {
    type Target = [usize];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
#[derive(Deserialize)]
pub struct TermBuf(SymbolTree);

impl Serialize for TermBuf {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.term().serialize(serializer)
    }
}

impl TermBuf {
    #[inline]
    pub fn symbol(symbol: impl AsRef<str>) -> Self {
        Self::from(Atom::from(sym(symbol)))
    }

    #[inline]
    pub fn number(num: impl Into<Decimal>) -> Self {
        Self::from(Atom::from(num))
    }

    #[inline]
    pub fn variable(var: impl Into<CompactString>) -> Self {
        Self::from(Atom::Variable(var.into().into()))
    }

    #[inline]
    pub fn param(param: impl Into<CompactString>) -> Self {
        Self::from(Atom::Param(param.into().into()))
    }

    #[inline]
    pub fn arg(mut self, arg: impl Into<Self>) -> Self {
        self.term_mut().push_last_arg(arg.into());
        self
    }

    #[inline]
    pub fn one() -> Self {
        Self::number(1)
    }

    #[inline]
    pub fn zero() -> Self {
        Self::number(0)
    }
}

impl TermBuf {
    #[inline]
    pub fn normalize(mut self, level: NormLevel) -> Self {
        self.term_mut().normalize(level);
        self
    }

    pub fn symbols(&self) -> HashSet<Symbol> {
        self.0
            .root()
            .bfs()
            .iter
            .filter_map(|x| x.data.symbol())
            .collect()
    }

    #[inline]
    pub fn replace(&mut self, src: &Self, dst: &Self) {
        self.term_mut().replace(src.term(), dst.term())
    }

    #[inline]
    pub fn data(&self) -> &Atom {
        self.0.root().data()
    }

    #[inline]
    pub fn get(&self, id: &TermPath) -> Option<TermRef<'_>> {
        let mut root = self.0.root();
        for i in id.0.iter() {
            root = root.iter().nth(*i)?;
        }
        Some(root.into())
    }

    #[inline]
    pub fn get_mut(&mut self, id: &TermPath) -> Option<TermMut<'_>> {
        let mut root = self.0.root_mut().get_mut();
        for i in id.0.iter() {
            let next_root = root.iter_mut().nth(*i)?.get_mut();
            root = next_root;
        }

        Some(root.into())
    }

    #[inline]
    pub fn term(&self) -> TermRef<'_> {
        self.0.root().into()
    }

    #[inline]
    pub fn term_mut(&mut self) -> TermMut<'_> {
        self.0.root_mut().get_mut().into()
    }

    #[inline]
    pub fn swap(&mut self, node: &mut TermMut) {
        self.term_mut().swap(node);
    }
}

impl From<Param> for TermBuf {
    fn from(value: Param) -> Self {
        Self::from(Atom::Param(value))
    }
}

impl From<Variable> for TermBuf {
    fn from(value: Variable) -> Self {
        Self::from(Atom::Variable(value))
    }
}

impl From<TermBuf> for SymbolTree {
    fn from(value: TermBuf) -> Self {
        value.0
    }
}

impl From<Atom> for TermBuf {
    fn from(value: Atom) -> Self {
        Self::from(Tree::new(value))
    }
}

impl From<SymbolTree> for TermBuf {
    fn from(source: SymbolTree) -> Self {
        Self(source)
    }
}

impl fmt::Debug for TermBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for TermBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.term())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        rc::Rc,
    };

    use crate::term::{ArgList, Term, term_with_params, term_with_vars};

    use super::*;

    #[test]
    fn unification_test() {
        let test = term_with_params("2*x*x + x + 3*x + 4 + 2 == 0").normalize(NormLevel::Full);
        let test_norm = term_with_params("2*x^2 + 4*x + 6 == 0").normalize(NormLevel::Full);
        assert_eq!(test, test_norm);
    }

    #[test]
    fn unification_with_minus_test() {
        let test = term_with_params("x^2 + (-5)*x - x + 5 == 0").normalize(NormLevel::Full);
        let test_norm = term_with_params("x^2 + (-6)*x + 5 == 0").normalize(NormLevel::Full);
        assert_eq!(test, test_norm);
    }

    #[test]
    fn placeholder_test() {
        let test = "set(a, ..) is known <=> true";

        let term = term_with_params(test);

        assert_eq!(
            term.term()
                .first_arg()
                .unwrap()
                .first_arg()
                .unwrap()
                .last_arg()
                .unwrap()
                .data()
                .placeholder(),
            Some(ArgList::from(1))
        );
    }

    #[test]
    fn hash_test() {
        let term = term_with_params("a*x + c == 0");
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_1 = s.finish();

        let term = Rc::new(term);
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_2 = s.finish();

        assert_eq!(hash_1, hash_2);
    }

    #[test]
    fn serde_test() {
        let test = term_with_vars("x in set(2, 5)");

        let json_text = serde_json::to_string(&test.term()).expect("");
        insta::assert_snapshot!(
            json_text,
            @r#"{"data":{"Symbol":"in"},"children":[{"data":{"Variable":"x"}},{"data":{"Symbol":"set"},"children":[{"data":{"Number":"2"}},{"data":{"Number":"5"}}]}]}"#
        );
    }

    #[test]
    fn serde_roundtrip_test() {
        for src in [
            "x",
            "2",
            "a*x + b == 0",
            "x in set(2, 5)",
            "set(a, ..) is known",
            "a*x + b == 0 => x == -b/a",
            "x^2 + 2*x*y + y^2 == (x + y)^2",
        ] {
            let term = term_with_params(src);
            let json = serde_json::to_string(&term.term())
                .unwrap_or_else(|e| panic!("serialize {src:?}: {e}"));
            let back: TermBuf = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize {src:?} from {json}: {e}"));
            assert_eq!(term, back, "roundtrip mismatch for {src:?}");
        }
    }
}
