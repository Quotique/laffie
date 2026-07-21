use std::{cmp::Ordering, collections::HashSet, iter::Iterator};

use derive_more::{Debug, Display, From};
use trees::Node;

use utils::SubsetIterator;

use super::{
    Atom, ParamSubstitution, Substitute, Symbol, Term, TermBuf, TermRef, Truth,
    VariableSubstitution,
};
use crate::NormLevel;

/// Max normalization sweeps before giving up (see `normalize`).
const NORMALIZE_FIXPOINT_CAP: usize = 8;

#[derive(Debug, Display, From)]
pub struct TermMut<'a>(&'a mut Node<Atom>);

impl<'a> TermMut<'a> {
    #[inline]
    pub fn as_ref(&self) -> TermRef<'_> {
        TermRef::from(self.0 as &Node<_>)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = TermRef<'_>> {
        self.0.iter().map(TermRef::from)
    }

    #[inline]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = TermMut<'b>> + 'b {
        self.0.iter_mut().map(|x| TermMut(x.get_mut()))
    }

    pub fn values(&self) -> impl Iterator<Item = &Atom> {
        self.0.bfs().iter.map(|x| x.data)
    }

    #[inline]
    pub fn first_arg(&self) -> Option<TermRef<'_>> {
        self.0.front().map(TermRef::from)
    }

    #[inline]
    pub fn last_arg(&self) -> Option<TermRef<'_>> {
        self.0.back().map(TermRef::from)
    }

    #[inline]
    pub fn first_arg_mut<'b>(&'b mut self) -> Option<TermMut<'b>> {
        self.0.front_mut().map(|x| TermMut(x.get_mut()))
    }

    #[inline]
    pub fn last_arg_mut<'b>(&'b mut self) -> Option<TermMut<'b>> {
        self.0.back_mut().map(|x| TermMut(x.get_mut()))
    }

    #[inline]
    pub fn insert_after(&mut self, term: TermBuf) {
        self.0.insert_next_sib(term.into());
    }

    #[inline]
    pub fn insert_before(&mut self, term: TermBuf) {
        self.0.insert_prev_sib(term.into());
    }

    #[inline]
    pub fn pop_first_arg(&mut self) -> Option<TermBuf> {
        self.0.pop_front().map(TermBuf::from)
    }

    #[inline]
    pub fn pop_last_arg(&mut self) -> Option<TermBuf> {
        self.0.pop_back().map(TermBuf::from)
    }

    #[inline]
    pub fn push_first_arg(&mut self, term: TermBuf) -> &mut Self {
        self.0.push_front(term.into());
        self
    }

    #[inline]
    pub fn push_last_arg(&mut self, term: TermBuf) -> &mut Self {
        self.0.push_back(term.into());
        self
    }
}

impl<'a> TermMut<'a> {
    pub fn symbols(&self) -> HashSet<Symbol> {
        self.0.bfs().iter.filter_map(|x| x.data.symbol()).collect()
    }

    #[inline]
    pub fn to_term(&self) -> TermBuf {
        self.0.deep_clone().into()
    }

    #[inline]
    pub fn detach(&mut self) -> TermBuf {
        self.0.detach().into()
    }

    #[inline]
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    #[inline]
    pub fn data(&self) -> &Atom {
        self.0.data()
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut Atom {
        self.0.data_mut()
    }
}

impl<'a> Substitute for TermMut<'a> {
    fn substitute(&mut self, params: &ParamSubstitution) {
        match self.data().clone() {
            Atom::Param(p) => {
                if let Some(p) = params.params.get(&p) {
                    self.swap(&mut p.as_ref().clone().term_mut());
                }
            }
            Atom::ArgList(p) => {
                if let Some(p) = params.arglists.get(&p) {
                    let mut p = p.clone();
                    if p.is_empty() {
                        self.detach();
                    } else {
                        self.swap(&mut p[0].term_mut());
                        for i in p.into_iter().skip(1).rev() {
                            self.insert_after(i);
                        }
                    }
                }
            }
            _ => {
                for mut i in self.iter_mut() {
                    i.substitute(params);
                }
            }
        }
    }
}

impl<'a> TermMut<'a> {
    pub fn apply_variable_map(&mut self, variables: &VariableSubstitution) {
        if let Some(mut v) = self
            .data()
            .variable()
            .and_then(|x| variables.get(x))
            .cloned()
        {
            self.swap(&mut v.term_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_variable_map(variables);
            }
        }
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = TermBuf> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().symbol().unwrap();
            let mut parts = vec![TermBuf::from(Atom::Symbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]]
                    .term_mut()
                    .push_last_arg(child.to_owned());
            }

            for p in parts.iter_mut() {
                if p.term().degree() == 1 {
                    let mut child = p.term_mut().pop_first_arg().unwrap();
                    p.term_mut().swap(&mut child.term_mut());
                }
            }
            let mut result = TermBuf::from(Atom::Symbol(s.clone()));
            for p in parts.into_iter() {
                result.term_mut().push_last_arg(p);
            }
            result
        }))
    }

    #[inline]
    pub fn evaluate(&mut self, level: NormLevel) -> bool {
        self.data()
            .symbol()
            .map(|x| x.evaluate(self, level))
            .unwrap_or(false)
    }

    #[inline]
    pub fn truth(&self) -> Truth {
        TermRef::from(self.0 as &Node<_>).truth()
    }

    /// Swap data and children between two subterms.
    ///
    /// This swaps the data fields, then moves all children from self to other,
    /// and moves excess children back to self to maintain the original degree
    /// of other.
    pub fn swap(&mut self, other: &mut TermMut) {
        std::mem::swap(self.data_mut(), other.data_mut());

        // Swap children: move all from self to other, then move excess back to self
        let self_degree = self.degree();

        // Move all children from self to other
        while let Some(t) = self.pop_first_arg() {
            other.push_last_arg(t);
        }

        // Move excess children from other back to self
        while other.degree() > self_degree {
            if let Some(t) = other.pop_first_arg() {
                self.push_last_arg(t);
            }
        }
    }

    pub fn replace(&mut self, src: TermRef, dst: TermRef) {
        self.iter_mut().for_each(|mut child| {
            child.replace(src, dst);
        });
        self.iter_mut().for_each(|mut child| {
            if child == src {
                child.swap(&mut dst.to_owned().term_mut());
            }
        });
    }

    fn associative_nesting_remove(&mut self) -> bool {
        let mut result = false;
        if let Some(symbol) = self.data().symbol() {
            if !symbol.is_associative() {
                return result;
            }
            let root_degree = self.degree();
            for _ in 0..root_degree {
                let mut child = self.pop_first_arg().unwrap();
                if let Some(child_symbol) = &child.data().symbol() &&
                    *child_symbol == symbol
                {
                    while let Some(node) = child.term_mut().pop_first_arg() {
                        self.push_last_arg(node);
                    }
                    result = true;
                    continue;
                }
                self.push_last_arg(child);
            }
        }
        result
    }

    pub fn commutative_reorder(&mut self) -> bool {
        if let Some(symbol) = self.data().symbol() {
            if !symbol.is_commutative() {
                return false;
            }
            if !self
                .iter()
                .is_sorted_by(|l, r| symbol.arg_order(*l, *r) != Ordering::Greater)
            {
                let mut to_sort: Vec<TermBuf> = vec![];
                while let Some(t) = self.pop_first_arg() {
                    to_sort.push(t);
                }

                to_sort.sort_by(|x, y| symbol.arg_order(x.term(), y.term()));

                while let Some(t) = to_sort.pop() {
                    self.push_first_arg(t);
                }
                return true;
            }
        }
        false
    }

    /// Normalizes the subtree to a fixpoint by repeating bottom-up passes,
    /// so a calculator can emit a term the next pass finishes.
    pub fn normalize(&mut self, level: NormLevel) -> bool {
        let mut any = false;
        for _ in 0..NORMALIZE_FIXPOINT_CAP {
            if !self.normalize_pass(level) {
                return any;
            }
            any = true;
        }
        // Hitting the cap = non-confluent rewrites; release stops with a
        // bounded partial result.
        debug_assert!(
            false,
            "normalize did not converge in {NORMALIZE_FIXPOINT_CAP} passes"
        );
        any
    }

    /// One bottom-up normalization sweep. Returns whether anything changed.
    fn normalize_pass(&mut self, level: NormLevel) -> bool {
        let mut result = false;
        for mut i in self.iter_mut() {
            result |= i.normalize_pass(level);
        }

        result |= self.associative_nesting_remove();
        result |= self.evaluate(level);
        if level > NormLevel::Off {
            result |= self.commutative_reorder();
        }
        result
    }
}

impl<'a, 'b> PartialEq<TermRef<'a>> for TermMut<'b> {
    fn eq(&self, other: &TermRef) -> bool {
        self.as_ref().eq(other)
    }
}

#[cfg(test)]
mod tests {
    use crate::term::term_with_params;

    #[test]
    fn replace_test() {
        let mut test_state = term_with_params("a/(a+b)");
        let test_pattern = term_with_params("(a+b)");
        let test_replace = term_with_params("c");

        test_state
            .term_mut()
            .replace(test_pattern.term(), test_replace.term());
        assert_eq!(test_state, term_with_params("a/c"));
    }
}

#[cfg(test)]
mod operations_tests {
    use crate::term::{TermBuf, var};

    use super::*;

    #[test]
    fn associative_nesting_remove_test() {
        // (1+2)+(1+2) -> 1+2+1+2
        let mut test_tree = TermBuf::symbol("+")
            .arg(
                TermBuf::symbol("+")
                    .arg(TermBuf::number(1))
                    .arg(TermBuf::number(2)),
            )
            .arg(
                TermBuf::symbol("+")
                    .arg(TermBuf::number(1))
                    .arg(TermBuf::number(2)),
            );
        assert!(test_tree.term_mut().associative_nesting_remove());
        assert_eq!(test_tree.term().degree(), 4);
    }

    #[test]
    fn evaluate_plus_test() {
        // 1+2+5 -> 8
        let mut test_tree1 = TermBuf::symbol("+")
            .arg(TermBuf::number(1))
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5));

        assert!(test_tree1.term_mut().evaluate(NormLevel::Full));
        assert_eq!(test_tree1, TermBuf::number(8));

        // x+1+2+5 -> x+8
        let mut test_tree1 = TermBuf::symbol("+")
            .arg(var("x"))
            .arg(TermBuf::number(1))
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5));

        assert!(test_tree1.term_mut().evaluate(NormLevel::Full));
        test_tree1.term_mut().commutative_reorder();
        assert_eq!(
            test_tree1,
            TermBuf::symbol("+").arg(var("x")).arg(TermBuf::number(8))
        );
    }

    #[test]
    fn evaluate_multiply_test() {
        // 1*2*5 -> 10
        let mut test_tree = TermBuf::symbol("*")
            .arg(TermBuf::number(1))
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5));
        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number(10));

        // x*1*2*5 -> 10*x
        let mut test_tree = TermBuf::symbol("*")
            .arg(var("x"))
            .arg(TermBuf::number(1))
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5));

        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        test_tree.term_mut().commutative_reorder();
        assert_eq!(
            test_tree,
            TermBuf::symbol("*").arg(TermBuf::number(10)).arg(var("x"))
        );

        // x*1 -> x
        let mut test_tree = TermBuf::symbol("*").arg(var("x")).arg(TermBuf::number(1));

        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::variable("x"));
    }

    #[test]
    fn evaluate_divide_test() {
        // 10 / 2 -> 5
        let mut test_tree = TermBuf::symbol("/")
            .arg(TermBuf::number(10))
            .arg(TermBuf::number(2));

        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number(5));

        // x / 2 -> x / 2
        let mut test_tree = TermBuf::symbol("/").arg(var("x")).arg(TermBuf::number(2));

        assert!(!test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(
            test_tree,
            TermBuf::symbol("/").arg(var("x")).arg(TermBuf::number(2))
        );

        // 2 / 5 -> 0.4
        let mut test_tree = TermBuf::symbol("/")
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5));
        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number((4, 1)));

        // 30 / 45 -> 2/3
        let mut test_tree = TermBuf::symbol("/")
            .arg(TermBuf::number(30))
            .arg(TermBuf::number(45));
        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(
            test_tree,
            TermBuf::symbol("/")
                .arg(TermBuf::number(2))
                .arg(TermBuf::number(3))
        );

        // 30 / 4.5 -> 20/3
        let mut test_tree = TermBuf::symbol("/")
            .arg(TermBuf::number(30))
            .arg(TermBuf::number((45, 1)));
        assert!(test_tree.term_mut().evaluate(NormLevel::Full));
        assert_eq!(
            test_tree,
            TermBuf::symbol("/")
                .arg(TermBuf::number(20))
                .arg(TermBuf::number(3))
        );
    }

    #[test]
    fn normalize_power_test() {
        // Negative-exponent reciprocals are folded by the fixpoint, not `power`.

        // 2 ^ 2 -> 4
        let mut test_tree = TermBuf::symbol("^")
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(2));
        assert!(test_tree.term_mut().normalize(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number(4));

        // 2 ^ (-2) -> 0.25
        let mut test_tree = TermBuf::symbol("^")
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(-2));
        assert!(test_tree.term_mut().normalize(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number((25, 2)));

        // 0.5 ^ (-2) -> 4
        let mut test_tree = TermBuf::symbol("^")
            .arg(TermBuf::number((5, 1)))
            .arg(TermBuf::number(-2));
        assert!(test_tree.term_mut().normalize(NormLevel::Full));
        assert_eq!(test_tree, TermBuf::number(4));

        // 3 ^ (-2) -> 1/9
        let mut test_tree = TermBuf::symbol("^")
            .arg(TermBuf::number(3))
            .arg(TermBuf::number(-2));
        assert!(test_tree.term_mut().normalize(NormLevel::Full));
        assert_eq!(
            test_tree,
            TermBuf::symbol("/")
                .arg(TermBuf::number(1))
                .arg(TermBuf::number(9))
        );
    }

    #[test]
    fn commutative_reorder_test() {
        // 1+2+5+(2*x)+x+(2+3) -> (2+3)+(2*x)+x+1+2+5
        let mut test_tree = TermBuf::symbol("+")
            .arg(TermBuf::number(1))
            .arg(TermBuf::number(2))
            .arg(TermBuf::number(5))
            .arg(TermBuf::symbol("*").arg(TermBuf::number(2)).arg(var("x")))
            .arg(TermBuf::variable("x"))
            .arg(
                TermBuf::symbol("+")
                    .arg(TermBuf::number(2))
                    .arg(TermBuf::number(3)),
            );

        assert!(test_tree.term_mut().commutative_reorder());
        assert_eq!(
            test_tree,
            TermBuf::symbol("+")
                .arg(
                    TermBuf::symbol("+")
                        .arg(TermBuf::number(2))
                        .arg(TermBuf::number(3))
                )
                .arg(TermBuf::symbol("*").arg(TermBuf::number(2)).arg(var("x")))
                .arg(var("x"))
                .arg(TermBuf::number(1))
                .arg(TermBuf::number(2))
                .arg(TermBuf::number(5))
        );
    }
}
