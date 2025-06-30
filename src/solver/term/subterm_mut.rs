use std::{cmp::Ordering, collections::HashSet, iter::Iterator, rc::Rc};

use derive_more::{Debug, From};
use trees::Node;

use utils::SubsetIterator;

use super::{ParamsMapping, Subterm, Symbol, Term, TermNode, Truth, VariablesMap};
use crate::NormalizationLevel;

#[derive(Debug, From)]
pub struct SubtermMut<'a>(&'a mut Node<TermNode>);

impl<'a> SubtermMut<'a> {
    #[inline]
    pub fn as_ref(&self) -> Subterm {
        Subterm::from(self.0 as &Node<_>)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Subterm> {
        self.0.iter().map(Subterm::from)
    }

    #[inline]
    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = SubtermMut<'b>> + 'b {
        self.0.iter_mut().map(|x| SubtermMut(x.get_mut()))
    }

    pub fn values(&self) -> impl Iterator<Item = &TermNode> {
        self.0.bfs().iter.map(|x| x.data)
    }

    #[inline]
    pub fn first_arg(&self) -> Option<Subterm> {
        self.0.front().map(Subterm::from)
    }

    #[inline]
    pub fn last_arg(&self) -> Option<Subterm> {
        self.0.back().map(Subterm::from)
    }

    #[inline]
    pub fn first_arg_mut<'b>(&'b mut self) -> Option<SubtermMut<'b>> {
        self.0.front_mut().map(|x| SubtermMut(x.get_mut()))
    }

    #[inline]
    pub fn last_arg_mut<'b>(&'b mut self) -> Option<SubtermMut<'b>> {
        self.0.back_mut().map(|x| SubtermMut(x.get_mut()))
    }

    #[inline]
    pub fn insert_after(&mut self, term: Term) {
        self.0.insert_next_sib(term.into());
    }

    #[inline]
    pub fn insert_before(&mut self, term: Term) {
        self.0.insert_prev_sib(term.into());
    }

    #[inline]
    pub fn pop_first_arg(&mut self) -> Option<Term> {
        self.0.pop_front().map(Term::from)
    }

    #[inline]
    pub fn pop_last_arg(&mut self) -> Option<Term> {
        self.0.pop_back().map(Term::from)
    }

    #[inline]
    pub fn push_first_arg(&mut self, term: Term) -> &mut Self {
        self.0.push_front(term.into());
        self
    }

    #[inline]
    pub fn push_last_arg(&mut self, term: Term) -> &mut Self {
        self.0.push_back(term.into());
        self
    }
}

impl<'a> SubtermMut<'a> {
    pub fn symbols(&self) -> HashSet<Symbol> {
        self.0.bfs().iter.filter_map(|x| x.data.symbol()).collect()
    }

    #[inline]
    pub fn to_term(&self) -> Term {
        self.0.deep_clone().into()
    }

    #[inline]
    pub fn detach(&mut self) -> Term {
        self.0.detach().into()
    }

    #[inline]
    pub fn degree(&self) -> usize {
        self.0.degree()
    }

    #[inline]
    pub fn data(&self) -> &TermNode {
        self.0.data()
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut TermNode {
        self.0.data_mut()
    }
}

impl<'a> SubtermMut<'a> {
    pub fn apply_param_map(&mut self, params: &ParamsMapping) -> &mut Self {
        match self.data().clone() {
            TermNode::Param(p) => {
                if let Some(p) = params.params.get(&p) {
                    self.swap(&mut p.clone().as_subterm_mut());
                }
            }
            TermNode::ArgList(p) => {
                if let Some(p) = params.arglists.get(&p) {
                    let mut p = p.clone();
                    self.swap(&mut p[0].as_subterm_mut());
                    for i in p.into_iter().skip(1).rev() {
                        self.insert_after(i);
                    }
                }
            }
            _ => {
                for mut i in self.iter_mut() {
                    i.apply_param_map(params);
                }
            }
        }
        self
    }

    pub fn apply_variable_map(&mut self, variables: &VariablesMap) {
        if let Some(mut v) = self
            .data()
            .variable()
            .and_then(|x| variables.get(x))
            .cloned()
        {
            self.swap(&mut v.as_subterm_mut());
        } else {
            for mut i in self.iter_mut() {
                i.apply_variable_map(variables);
            }
        }
    }

    pub fn subsets(&self, count: usize) -> Box<dyn Iterator<Item = Term> + '_> {
        Box::new(SubsetIterator::new(self.degree(), count).map(move |i| {
            let s = self.data().symbol().unwrap();
            let mut parts = vec![Term::from(TermNode::Symbol(s.clone())); count];
            for (id, child) in self.iter().enumerate() {
                parts[i.as_vec()[id]]
                    .as_subterm_mut()
                    .push_last_arg(child.to_term());
            }

            for p in parts.iter_mut() {
                if p.as_subterm().degree() == 1 {
                    let mut child = p.as_subterm_mut().pop_first_arg().unwrap();
                    p.as_subterm_mut().swap(&mut child.as_subterm_mut());
                }
            }
            let mut result = Term::from(TermNode::Symbol(s.clone()));
            for p in parts.into_iter() {
                result.as_subterm_mut().push_last_arg(p);
            }
            result
        }))
    }

    #[inline]
    pub fn evaluate(&mut self, level: NormalizationLevel) -> bool {
        self.data()
            .symbol()
            .map(|x| x.evaluate(self, level))
            .unwrap_or(false)
    }

    #[inline]
    pub fn truth(&self) -> Truth {
        Subterm::from(self.0 as &Node<_>).truth()
    }

    // TODO: fast swap with first arg
    pub fn swap(&mut self, other: &mut SubtermMut) {
        std::mem::swap(self.data_mut(), other.data_mut());
        let self_degree = self.degree();

        while let Some(t) = self.pop_first_arg() {
            other.push_last_arg(t);
        }
        while other.degree() > self_degree {
            if let Some(t) = other.pop_first_arg() {
                self.push_last_arg(t);
            }
        }
    }

    pub fn replace(&mut self, src: Subterm, dst: Subterm) {
        self.iter_mut().for_each(|mut child| {
            child.replace(src, dst);
        });
        self.iter_mut().for_each(|mut child| {
            if child == src {
                child.swap(&mut dst.to_term().as_subterm_mut());
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
                if let Some(child_symbol) = &child.data().symbol() {
                    if *child_symbol == symbol {
                        while let Some(node) = child.as_subterm_mut().pop_first_arg() {
                            self.push_last_arg(node);
                        }
                        result = true;
                        continue;
                    }
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
                let mut to_sort = vec![];
                while let Some(t) = self.pop_first_arg() {
                    to_sort.push(Rc::new(t));
                }

                to_sort.sort_by(|x, y| symbol.arg_order(x.as_subterm(), y.as_subterm()));

                while let Some(t) = to_sort.pop() {
                    self.push_first_arg(Rc::try_unwrap(t).unwrap());
                }
                return true;
            }
        }
        false
    }

    pub fn normalize(&mut self, level: NormalizationLevel) -> bool {
        let mut result = false;
        for mut i in self.iter_mut() {
            result |= i.normalize(level);
        }

        result |= self.associative_nesting_remove();
        result |= self.evaluate(level);
        if level > NormalizationLevel(0) {
            result |= self.commutative_reorder();
        }
        result
    }
}

impl<'a, 'b> PartialEq<Subterm<'a>> for SubtermMut<'b> {
    fn eq(&self, other: &Subterm) -> bool {
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
            .as_subterm_mut()
            .replace(test_pattern.as_subterm(), test_replace.as_subterm());
        assert_eq!(test_state, term_with_params("a/c"));
    }
}

#[cfg(test)]
mod operations_tests {
    use crate::term::Term;

    use super::*;

    #[test]
    fn associative_nesting_remove_test() {
        // (1+2)+(1+2) -> 1+2+1+2
        let mut test_tree = Term::symbol("+")
            .with_child(
                Term::symbol("+")
                    .with_child(Term::number(1))
                    .with_child(Term::number(2)),
            )
            .with_child(
                Term::symbol("+")
                    .with_child(Term::number(1))
                    .with_child(Term::number(2)),
            );
        assert!(test_tree.as_subterm_mut().associative_nesting_remove());
        assert_eq!(test_tree.as_subterm().degree(), 4);
    }

    #[test]
    fn evaluate_plus_test() {
        // 1+2+5 -> 8
        let mut test_tree1 = Term::symbol("+")
            .with_child(Term::number(1))
            .with_child(Term::number(2))
            .with_child(Term::number(5));

        assert!(test_tree1
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree1, Term::number(8));

        // x+1+2+5 -> x+8
        let mut test_tree1 = Term::symbol("+")
            .with_child(Term::variable("x"))
            .with_child(Term::number(1))
            .with_child(Term::number(2))
            .with_child(Term::number(5));

        assert!(test_tree1
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        test_tree1.as_subterm_mut().commutative_reorder();
        assert_eq!(
            test_tree1,
            Term::symbol("+")
                .with_child(Term::variable("x"))
                .with_child(Term::number(8))
        );
    }

    #[test]
    fn evaluate_multiply_test() {
        // 1*2*5 -> 10
        let mut test_tree = Term::symbol("*")
            .with_child(Term::number(1))
            .with_child(Term::number(2))
            .with_child(Term::number(5));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number(10));

        // x*1*2*5 -> 10*x
        let mut test_tree = Term::symbol("*")
            .with_child(Term::variable("x"))
            .with_child(Term::number(1))
            .with_child(Term::number(2))
            .with_child(Term::number(5));

        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        test_tree.as_subterm_mut().commutative_reorder();
        assert_eq!(
            test_tree,
            Term::symbol("*")
                .with_child(Term::number(10))
                .with_child(Term::variable("x"))
        );

        // x*1 -> x
        let mut test_tree = Term::symbol("*")
            .with_child(Term::variable("x"))
            .with_child(Term::number(1));

        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::variable("x"));
    }

    #[test]
    fn evaluate_divide_test() {
        // 10 / 2 -> 5
        let mut test_tree = Term::symbol("/")
            .with_child(Term::number(10))
            .with_child(Term::number(2));

        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number(5));

        // x / 2 -> x / 2
        let mut test_tree = Term::symbol("/")
            .with_child(Term::variable("x"))
            .with_child(Term::number(2));

        assert!(!test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            Term::symbol("/")
                .with_child(Term::variable("x"))
                .with_child(Term::number(2))
        );

        // 2 / 5 -> 0.4
        let mut test_tree = Term::symbol("/")
            .with_child(Term::number(2))
            .with_child(Term::number(5));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number((4, 1)));

        // 30 / 45 -> 2/3
        let mut test_tree = Term::symbol("/")
            .with_child(Term::number(30))
            .with_child(Term::number(45));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            Term::symbol("/")
                .with_child(Term::number(2))
                .with_child(Term::number(3))
        );

        // 30 / 4.5 -> 20/3
        let mut test_tree = Term::symbol("/")
            .with_child(Term::number(30))
            .with_child(Term::number((45, 1)));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            Term::symbol("/")
                .with_child(Term::number(20))
                .with_child(Term::number(3))
        );
    }

    #[test]
    fn evaluate_power_test() {
        // 2 ^ 2 -> 4
        let mut test_tree = Term::symbol("^")
            .with_child(Term::number(2))
            .with_child(Term::number(2));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number(4));

        // 2 ^ (-2) -> 0.25
        let mut test_tree = Term::symbol("^")
            .with_child(Term::number(2))
            .with_child(Term::number(-2));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number((25, 2)));

        // 0.5 ^ (-2) -> 4
        let mut test_tree = Term::symbol("^")
            .with_child(Term::number((5, 1)))
            .with_child(Term::number(-2));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(test_tree, Term::number(4));

        // 3 ^ (-2) -> 1/9
        let mut test_tree = Term::symbol("^")
            .with_child(Term::number(3))
            .with_child(Term::number(-2));
        assert!(test_tree
            .as_subterm_mut()
            .evaluate(NormalizationLevel::max()));
        assert_eq!(
            test_tree,
            Term::symbol("/")
                .with_child(Term::number(1))
                .with_child(Term::number(9))
        );
    }

    #[test]
    fn commutative_reorder_test() {
        // 1+2+5+(2*x)+x+(2+3) -> (2+3)+(2*x)+x+1+2+5
        let mut test_tree = Term::symbol("+")
            .with_child(Term::number(1))
            .with_child(Term::number(2))
            .with_child(Term::number(5))
            .with_child(
                Term::symbol("*")
                    .with_child(Term::number(2))
                    .with_child(Term::variable("x")),
            )
            .with_child(Term::variable("x"))
            .with_child(
                Term::symbol("+")
                    .with_child(Term::number(2))
                    .with_child(Term::number(3)),
            );

        assert!(test_tree.as_subterm_mut().commutative_reorder());
        assert_eq!(
            test_tree,
            Term::symbol("+")
                .with_child(
                    Term::symbol("+")
                        .with_child(Term::number(2))
                        .with_child(Term::number(3))
                )
                .with_child(
                    Term::symbol("*")
                        .with_child(Term::number(2))
                        .with_child(Term::variable("x"))
                )
                .with_child(Term::variable("x"))
                .with_child(Term::number(1))
                .with_child(Term::number(2))
                .with_child(Term::number(5))
        );
    }
}
