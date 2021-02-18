use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    iter::{FromIterator, Iterator},
    ops::{Index, IndexMut},
    sync::Arc,
};

use trees::tr;

use crate::{
    core::term::Term,
    rule::{Rule, RulesEngine, SharedRule, Suppose},
    solver::operations::is_true,
    statement::{MarkedStatement, Statement},
};

use super::{
    problem::ProblemBuilder,
    solution::{Solution, SolutionError},
};

pub const STACK_SIZE: usize = 20;

pub struct Frame {
    stack: Vec<MarkedStatement>,
    index: HashMap<Arc<Statement>, usize>,

    rules_engine: Arc<RulesEngine>,
}

impl Frame {
    pub fn new(rules: Arc<RulesEngine>) -> Self {
        Frame {
            stack:        Vec::new(),
            index:        HashMap::new(),
            rules_engine: rules,
        }
    }

    pub fn with_statements(
        rules: Arc<RulesEngine>,
        statements: impl IntoIterator<Item = MarkedStatement>,
    ) -> Self {
        let mut result = Self::new(rules);
        for i in statements {
            result.add_condition(i);
        }
        result
    }

    #[inline]
    pub fn contains(&self, statement: &Arc<Statement>) -> bool {
        self.index.contains_key(statement)
    }

    #[inline]
    pub fn find(&self, statement: &Arc<Statement>) -> Option<usize> {
        self.index.get(statement).copied()
    }

    pub fn add_condition(&mut self, mut statement: MarkedStatement) -> Result<(), SolutionError> {
        if self.contains(&statement.statement) {
            return Ok(());
        }

        // if let Some(x) = self.dumper.as_ref() {
        // 	   x.borrow_mut().add_statement(&statement);
        // }
        statement.id = self.stack.len();
        self.index
            .insert(statement.statement.clone(), self.stack.len());
        self.stack.push(statement);
        if self.stack.len() > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }
        Ok(())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &MarkedStatement> {
        self.stack.iter()
    }

    #[inline]
    pub fn last(&self) -> Option<&MarkedStatement> {
        self.stack.last()
    }

    pub fn pick_condition(&self) -> Result<usize, SolutionError> {
        trace!("{:?}", self.stack);
        self.stack
            .iter()
            .enumerate()
            .filter(|(_, x)| !x.replaced)
            .min_by_key(|(_, x)| x.weight)
            .map(|(num, _)| num)
            .ok_or(SolutionError::NoConditions)
    }

    pub fn suppose_proof(&self, suppose: &Suppose) -> bool {
        for req in suppose.requirements.iter() {
            if !self.proof(&req) {
                return false;
            }
        }
        return true;
    }

    pub fn proof(&self, statement: &Statement) -> bool {
        if is_true(statement.root()) {
            return true;
        }

        let subproblem = ProblemBuilder::new()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("proof").unwrap()) / statement.root().to_owned(),
            ))))
            .expect("Can't build subproblem")
            .with_conditions(self.stack.iter().cloned())
            .build()
            .expect("Can't build subproblem");
        let mut solution = Solution::new(subproblem, self.rules_engine.clone());
        // if let Some(x) = self.dumper.as_ref() {
        //     solution.set_dumper(x.clone());
        // }

        if solution.solve().is_err() {
            trace!("Can't proof: {}", statement);
            return false;
        }
        return true;
    }

    pub fn transform(&self, statement: &MarkedStatement) -> Option<MarkedStatement> {
        if statement.simplified {
            return None;
        }

        let subproblem = ProblemBuilder::new()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("transform").unwrap()) /
                    statement.statement.root().to_owned(),
            ))))
            .expect("Can't build subproblem")
            .with_conditions(self.stack.iter().cloned())
            .build()
            .expect("Can't build subproblem");
        let mut solution = Solution::new(subproblem, self.rules_engine.clone());

        if solution.solve().is_err() || statement.statement == solution.answer().unwrap() {
            return None;
        }
        let mut result = MarkedStatement::from(solution.answer().unwrap().clone());
        result.simplified = true;
        result.parents.push(statement.id);

        Some(result)
    }

    pub fn next_statement(
        &mut self,
        local_rules: Vec<SharedRule>,
        index: usize,
        target: &MarkedStatement,
    ) -> Vec<MarkedStatement> {
        self.next_statement_with_filter(local_rules, index, target, |_| true)
    }

    pub fn next_statement_with_statement(
        &self,
        mut local_rules: Vec<SharedRule>,
        statement: &mut MarkedStatement,
        target: &MarkedStatement,
        filter: impl Fn(&Rule) -> bool,
    ) -> Vec<MarkedStatement> {
        let mut rules = self.rules_engine.suggest_rules(statement, target);
        rules.append(&mut local_rules);
        for rule in rules {
            let rule = rule.read().expect("Can't read rule");
            if !filter(&rule) {
                continue;
            }

            if let Ok(result) = rule.apply(statement, target) {
                let mut res = vec![];
                for sup in result {
                    let mut proofed = true;
                    if self.contains(&sup.resolution.statement) {
                        continue;
                    }

                    for req in sup.requirements {
                        if !self.proof(req.as_ref()) {
                            proofed = false;
                            break;
                        }
                    }

                    if proofed {
                        res.push(sup.resolution);
                    }
                }
                if res.len() > 0 {
                    return res;
                }
            }
        }
        vec![]
    }

    pub fn next_statement_with_filter(
        &mut self,
        mut local_rules: Vec<SharedRule>,
        index: usize,
        target: &MarkedStatement,
        filter: impl Fn(&Rule) -> bool,
    ) -> Vec<MarkedStatement> {
        let mut rules = self.rules_engine.suggest_rules(&self.stack[index], target);
        rules.append(&mut local_rules);
        for rule in rules {
            let rule = rule.read().expect("Can't read rule");
            if !filter(&rule) {
                continue;
            }

            if let Ok(result) = rule.apply(&mut self.stack[index], target) {
                let mut res = vec![];
                for sup in result {
                    let mut proofed = true;
                    if self.contains(&sup.resolution.statement) {
                        continue;
                    }

                    for req in sup.requirements {
                        if !self.proof(req.as_ref()) {
                            proofed = false;
                            break;
                        }
                    }

                    if proofed {
                        res.push(sup.resolution);
                    }
                }
                if res.len() > 0 {
                    return res;
                }
            }
        }
        vec![]
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
