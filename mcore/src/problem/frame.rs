use std::{
    collections::HashMap,
    fmt,
    iter::Iterator,
    ops::{Index, IndexMut},
    sync::Arc,
};

use trees::tr;

use crate::{
    rule::{Rule, RulesEngine, SharedRule, Suppose},
    statement::{term::Term, tree_utils::NodeMapping, MarkedStatement, Statement},
    utils::{Dumper, DumperSink, VecDisplay},
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
    dumper:       Dumper,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", VecDisplay(&self.stack))
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Frame {
    pub fn new(rules: Arc<RulesEngine>, dump: Dumper) -> Self {
        Frame {
            stack:        Vec::new(),
            index:        HashMap::new(),
            rules_engine: rules,
            dumper:       dump,
        }
    }

    pub fn with_statements(
        rules: Arc<RulesEngine>,
        dumper: Dumper,
        statements: impl IntoIterator<Item = MarkedStatement>,
    ) -> Self {
        let mut result = Self::new(rules, dumper);
        for i in statements {
            // TODO: error processing
            let _ = result.add_condition(i);
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

    #[inline]
    pub fn dumper(&mut self) -> &mut Dumper {
        &mut self.dumper
    }

    pub fn add_condition(&mut self, mut statement: MarkedStatement) -> Result<(), SolutionError> {
        if self.contains(&statement.statement) {
            return Ok(());
        }
        self.dumper.add_statement(&statement);

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
            if !self.proof(req) {
                return false;
            }
        }
        true
    }

    pub fn proof(&self, statement: &Statement) -> bool {
        if statement.root().check_truth() {
            return true;
        }

        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("proof").unwrap()) / statement.root().deep_clone(),
            ))))
            .expect("Can't build subproblem")
            .with_conditions(self.stack.iter().cloned())
            .build()
            .expect("Can't build subproblem");
        let mut solution =
            Solution::new(subproblem, self.rules_engine.clone(), self.dumper.clone());

        if let Err(e) = solution.solve() {
            trace!("Can't proof {}: {}", statement, e);
            return false;
        }
        true
    }

    pub fn transform(&mut self, index: usize) -> Option<MarkedStatement> {
        if self[index].simplified {
            return None;
        }
        self[index].simplified = true;

        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("transform").unwrap()) /
                    self[index].statement.root().deep_clone(),
            ))))
            .expect("Can't build subproblem")
            .with_conditions(self.stack.iter().cloned())
            .build()
            .expect("Can't build subproblem");
        let mut solution =
            Solution::new(subproblem, self.rules_engine.clone(), self.dumper.clone());

        if solution.solve().is_err() || self[index].statement == solution.answer().unwrap() {
            return None;
        }
        let mut result = MarkedStatement::from(solution.answer().unwrap());
        result.blocked_rules = self[index].blocked_rules.clone();
        result.simplified = true;
        result.parents.push(self[index].id);

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
            let rule = rule.read();
            trace!(target: "rule_selection", "Rule: {}", rule);

            if !filter(&rule) {
                trace!(target: "rule_selection", "Rule: {} rejected by filter", rule);
                continue;
            }

            if let Ok(result) = rule
                .apply(statement, target)
                .map_err(|e| trace!(target: "rule_selection", "Rule not applied: {:?}", e))
            {
                let mut res = vec![];
                for sup in result {
                    let mut proofed = true;
                    if self.contains(&sup.resolution.statement) {
                        continue;
                    }

                    trace!(target: "rule_selection", "Suppose: {}", sup);
                    for req in sup.requirements {
                        if !self.proof(req.as_ref()) {
                            trace!(target: "rule_selection", "Can't proof: {} suppose rejected", req);
                            proofed = false;
                            break;
                        }
                    }

                    if proofed {
                        trace!(target: "rule_selection", "Suppose: proofed, resolution applied");
                        res.push(sup.resolution);
                    }
                }
                if !res.is_empty() {
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
            let rule = rule.read();
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
                if !res.is_empty() {
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
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        sync::Arc,
    };

    use crate::parser::statement_with_params;

    use super::*;

    #[test]
    fn hash_test() {
        let statement: Statement = statement_with_params("a*x + c == 0");
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
