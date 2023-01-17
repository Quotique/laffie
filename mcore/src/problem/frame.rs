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
        write!(f, "{:?}", self)
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
                .parents
                .first()
                .map(|id| self.stack[*id].clone())
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

    pub fn suppose_proof(&self, suppose: &Suppose) -> bool {
        for req in suppose.requirements.iter() {
            if !self.proof(req) {
                return false;
            }
        }
        true
    }

    pub fn proof(&self, statement: &Statement) -> bool {
        if statement.root().check_truth().is_true() {
            return true;
        }

        let mut clone = statement.root().deep_clone();
        is_replace(&mut clone.root_mut());
        normalize(&mut clone.root_mut());

        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("proof").unwrap()) / clone,
            ))))
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

        let (answer_wrap, to_transform) =
            if self[index].statement.root().data().is_symbol_name("answer") {
                (
                    true,
                    self[index].statement.root().front().unwrap().deep_clone(),
                )
            } else {
                (false, self[index].statement.root().deep_clone())
            };

        let subproblem = ProblemBuilder::default()
            .with_target(MarkedStatement::from(Arc::new(Statement::from(
                tr(Term::with_symbol_name("transform").unwrap()) / to_transform,
            ))))
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

        solution.solve().ok()?;
        let mut answer = solution.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = tr(Term::with_symbol_name("answer").unwrap());
            swap_node(&mut answer.root_mut(), &mut tmp.root_mut());
            answer.root_mut().push_back(tmp);
        }

        if solution.solve().is_err() || *self[index].statement == answer {
            return None;
        }
        let mut result = MarkedStatement::from(Arc::new(answer));
        result.blocked_rules = self[index].blocked_rules.clone();
        result.simplified = true;
        result.parents = vec![self[index].id];

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
        for shared_rule in rules {
            let rule = shared_rule.read();
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
                for mut sup in result {
                    let mut proofed = true;
                    sup.resolution.rule = Some(shared_rule.clone());
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
        for shared_rule in rules {
            let rule = shared_rule.read();
            trace!(target: "rule_selection", "Rule: {}", rule);

            if !filter(&rule) {
                trace!(target: "rule_selection", "Rule: {} rejected by filter", rule);
                continue;
            }

            if let Ok(result) = rule
                .apply(&mut self.stack[index], target)
                .map_err(|e| trace!(target: "rule_selection", "Rule not applied: {:?}", e))
            {
                let mut res = vec![];
                for mut sup in result {
                    let mut proofed = true;
                    sup.resolution.rule = Some(shared_rule.clone());
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
