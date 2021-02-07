use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    iter::{FromIterator, Iterator},
    ops::{Index, IndexMut},
    sync::Arc,
};

use crate::statement::{MarkedStatement, Statement};

use super::solution::SolutionError;

pub const STACK_SIZE: usize = 20;

#[derive(Default)]
pub struct Frame {
    stack: Vec<MarkedStatement>,
    index: HashMap<Arc<Statement>, usize>,
}

impl Frame {
    #[inline]
    pub fn contains(&self, statement: &Arc<Statement>) -> bool {
        self.index.contains_key(statement)
    }

    pub fn add_condition(&mut self, statement: MarkedStatement) -> Result<(), SolutionError> {
        if self.contains(&statement.statement) {
            return Ok(());
        }

        // if let Some(x) = self.dumper.as_ref() {
        // 	   x.borrow_mut().add_statement(&statement);
        // }
        self.index
            .insert(statement.statement.clone(), self.stack.len());
        self.stack.push(statement);
        if self.stack.len() > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &MarkedStatement> {
        self.stack.iter()
    }

    pub fn pick_condition(&self) -> Result<usize, SolutionError> {
        self.stack
            .iter()
            .enumerate()
            .min_by_key(|(_, x)| x.weight)
            .map(|(num, _)| num)
            .ok_or(SolutionError::NoConditions)
    }
}

impl Index<usize> for Frame {
    type Output = MarkedStatement;

    fn index(&self, index: usize) -> &Self::Output {
        self.stack.index(index)
    }
}

impl IndexMut<usize> for Frame {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.stack.index_mut(index)
    }
}

impl FromIterator<MarkedStatement> for Frame {
    fn from_iter<I: IntoIterator<Item = MarkedStatement>>(iter: I) -> Self {
        let mut result = Self::default();
        for item in iter {
            let _ = result.add_condition(item);
        }
        result
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::core::term::Term;
    use std::sync::Arc;
    use trees::tr;

    #[test]
    fn hash_test() {
        let statement: Statement = (tr(Term::with_symbol_name("==").unwrap()) /
            (tr(Term::with_symbol_name("+").unwrap()) /
                (tr(Term::with_symbol_name("*").unwrap()) /
                    tr(Term::Param(1)) /
                    tr(Term::Param(2))) /
                tr(Term::Param(3))) /
            tr(Term::Number(0.into())))
        .into();
        let mut s = DefaultHasher::new();
        statement.hash(&mut s);
        let hash_1 = s.finish();

        let statement = Arc::new(statement);
        let mut s = DefaultHasher::new();
        statement.hash(&mut s);
        let hash_2 = s.finish();

        assert_eq!(hash_1, hash_2);
    }
}
