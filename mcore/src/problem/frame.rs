use std::{
    collections::HashMap,
    fmt,
    iter::Iterator,
    ops::{Index, IndexMut},
    sync::Arc,
};

use trees::tr;

use crate::{
    predefine::{normalize, symbol_by_name},
    rule::{Rule, RulesEngine, SharedRule, Suppose},
    statement::{
        term::{StatementNode, Term},
        tree_utils::{swap_node, NodeMapping},
        MarkedStatement, Statement,
    },
    utils::{Dumper, DumperSink, VecDisplay},
};

use super::{
    cache::{ProblemStatus, ProblemsCache},
    problem::ProblemBuilder,
    solution::{Solution, SolutionError},
};

pub const STACK_SIZE: usize = 2048;

pub struct Frame {
    stack: Vec<MarkedStatement>,
    index: HashMap<Arc<Statement>, usize>,

    rules_engine: Arc<RulesEngine>,
    dumper:       Dumper,

    subproblem_level: usize,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", VecDisplay(&self.stack))
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

fn is_replace(root: &mut StatementNode) {
    if !root.data().is_symbol_name("is") || root.degree() != 2 {
        return;
    }

    match root.back().unwrap().data().symbol().map(|x| x.name) {
        Some(name) if name == "true" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
        }
        Some(name) if name == "false" => {
            let child = root.pop_front().unwrap();
            let mut neg = tr(Term::Symbol(symbol_by_name("!").unwrap().id)) / child;
            swap_node(root, &mut neg.root_mut());
        }
        _ => {}
    }
}

impl Frame {
    pub fn new(rules: Arc<RulesEngine>, dump: Dumper, level: usize) -> Self {
        Frame {
            stack:        Vec::new(),
            index:        HashMap::new(),
            rules_engine: rules,
            dumper:       dump,

            subproblem_level: level,
        }
    }

    pub fn with_statements(
        rules: Arc<RulesEngine>,
        dumper: Dumper,
        statements: impl IntoIterator<Item = MarkedStatement>,
        level: usize,
    ) -> Self {
        let mut result = Self::new(rules, dumper, level);
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
        self.dumper.add_statement(
            &statement,
            &statement
                .parent
                .map(|id| self.stack[id].clone())
                .unwrap_or_else(|| MarkedStatement::from(Arc::new(Statement::zero()))),
        );

        statement.id = self.stack.len();
        if self.stack.len() + 1 > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }
        self.index
            .insert(statement.statement.clone(), self.stack.len());
        self.stack.push(statement);
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

    pub fn suppose_proof(
        &self,
        suppose: &Suppose,
        cache: Arc<ProblemsCache>,
    ) -> Option<Vec<Arc<Statement>>> {
        let mut result = vec![];
        for req in suppose.requirements.iter() {
            if let Some(proofed) = self.proof(req, cache.clone()) {
                result.push(proofed);
            } else {
                trace!(target: "rule_selection", "Can't proof: {}", req);
                return None;
            }
        }
        Some(result)
    }

    // Returns proof target (is a key for problems cache)
    pub fn proof(
        &self,
        statement: &Statement,
        cache: Arc<ProblemsCache>,
    ) -> Option<Arc<Statement>> {
        let mut clone = statement.root().deep_clone();
        is_replace(&mut clone.root_mut());
        normalize(&mut clone.root_mut());

        let proof_target = Arc::new(Statement::from(
            tr(Term::with_symbol_name("proof").unwrap()) / clone,
        ));

        if statement.root().check_truth().is_true() {
            return Some(proof_target);
        }

        if let Some(status) = cache.status(&proof_target) {
            match status {
                ProblemStatus::Solved(_) => return Some(proof_target),
                _ => return None,
            }
        }

        cache.add(Statement::from(proof_target.root().deep_clone()));

        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(proof_target.clone()))
            .expect("Can't build subproblem")
            .with_conditions(
                self.stack
                    .iter()
                    .filter(|x| !x.statement.root().data().is_symbol_name("answer"))
                    .cloned(),
            )
            .with_level(self.subproblem_level + 1)
            .build()
            .expect("Can't build subproblem");
        let mut solution =
            Solution::new(subproblem, self.rules_engine.clone(), self.dumper.clone());

        if let Err(e) = solution.solve_subproblem(cache.clone()) {
            trace!("Can't proof {}: {}", statement, e);
            cache.update_status(&proof_target, ProblemStatus::NotSolved);
            return None;
        }
        cache.update_status(&proof_target, ProblemStatus::Solved(Arc::new(solution)));
        Some(proof_target)
    }

    pub fn transform(
        &mut self,
        index: usize,
        cache: Arc<ProblemsCache>,
    ) -> Option<MarkedStatement> {
        if self[index].simplified {
            return None;
        }
        self[index].simplified = true;

        let (answer_wrap, to_transform) =
            if self[index].statement.root().data().is_symbol_name("answer") {
                (
                    true,
                    self[index].statement.root().front().unwrap().deep_clone(),
                )
            } else {
                (false, self[index].statement.root().deep_clone())
            };

        let problem = Arc::new(Statement::from(
            tr(Term::with_symbol_name("transform").unwrap()) / to_transform,
        ));
        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(problem.clone()))
            .expect("Can't build subproblem")
            .with_conditions(
                self.stack
                    .iter()
                    .filter(|x| !x.statement.root().data().is_symbol_name("answer"))
                    .cloned(),
            )
            .with_level(self.subproblem_level + 1)
            .build()
            .expect("Can't build subproblem");
        let mut solution =
            Solution::new(subproblem, self.rules_engine.clone(), self.dumper.clone());

        solution.solve_subproblem(cache).ok()?;
        let mut answer = solution.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = tr(Term::with_symbol_name("answer").unwrap());
            swap_node(&mut answer.root_mut(), &mut tmp.root_mut());
            answer.root_mut().push_back(tmp);
        }

        if *self[index].statement == answer {
            return None;
        }
        let mut result = MarkedStatement::from(Arc::new(answer));
        result.blocked_rules = self[index].blocked_rules.clone();
        result.simplified = true;
        result.parent = Some(self[index].id);
        result.requirements.push(problem);

        Some(result)
    }

    pub fn next_statement(
        &mut self,
        local_rules: &[SharedRule],
        index: usize,
        target: &MarkedStatement,
        cache: Arc<ProblemsCache>,
    ) -> Vec<MarkedStatement> {
        self.next_statement_with_filter(local_rules, index, target, |_| true, cache)
    }

    pub fn next_statement_with_filter(
        &mut self,
        local_rules: &[SharedRule],
        index: usize,
        target: &MarkedStatement,
        filter: impl Fn(&Rule) -> bool,
        cache: Arc<ProblemsCache>,
    ) -> Vec<MarkedStatement> {
        // Need to split &mut self into (&self, &mut MarkedStatement).
        // Only list of applied rules will be chahged in MarkedStatement, so it's safe.
        let statement: *mut MarkedStatement = &mut self.stack[index];
        let statement: &mut MarkedStatement = unsafe { &mut *statement };
        self.next_statement_with_statement(local_rules, statement, target, filter, cache)
    }

    pub fn next_statement_with_statement(
        &self,
        local_rules: &[SharedRule],
        statement: &mut MarkedStatement,
        target: &MarkedStatement,
        filter: impl Fn(&Rule) -> bool,
        cache: Arc<ProblemsCache>,
    ) -> Vec<MarkedStatement> {
        for (rule, supposes) in self
            .rules_engine
            .suggest_rules(statement, target)
            .iter()
            .chain(local_rules.iter())
            .inspect(|rule| trace!(target: "rule_selection", "Rule: {}", rule))
            .filter(|rule| filter(rule))
            .filter_map(|rule| {
                rule.apply(statement, target)
                    .map_err(|e| trace!(target: "rule_selection", "Rule not applied: {:?}", e))
                    .ok()
                    .map(|supposes| (rule, supposes))
            })
        {
            let res: Vec<_> = supposes
                .into_iter()
                .filter(|suppose| !self.contains(&suppose.resolution.statement))
                .inspect(|suppose| trace!(target: "rule_selection", "Suppose: {}", suppose))
                .filter_map(|mut suppose| {
                    if let Some(proofed) = self.suppose_proof(&suppose, cache.clone()) {
                        suppose.resolution.requirements = proofed;
                        Some(suppose)
                    } else {
                        None
                    }
                })
                .inspect(
                    |_| trace!(target: "rule_selection", "Suppose: proofed, resolution applied"),
                )
                .map(|mut suppose| {
                    suppose.resolution.rule = Some(rule.clone());
                    suppose.resolution
                })
                .collect();
            if !res.is_empty() {
                return res;
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

    use crate::statement::statement_with_params;

    #[test]
    fn hash_test() {
        let statement = statement_with_params("a*x + c == 0");
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
