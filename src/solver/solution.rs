use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
    time::Instant,
};
use trees::linked::fully::tr;

use colored::*;

use core::{
    rule::{Rule, RuleAttr, RulesEngine},
    statement::Statement,
    symbols::symbol_by_name,
    term::{StatementTree, Term},
    tree_utils::swap_node,
};

use super::{
    operations::{is_true, normalize},
    problem::{MarkedStatement, Problem, ProblemType, DEFAULT_WEIGHT},
};

pub const MAX_SUBPROBLEM_LEVEL: usize = 10;
pub const STACK_SIZE: usize = 20;

pub struct PerfStats {
    problem_hash:   u64,
    cycles_count:   usize,
    solution_depth: usize,
    absolute_time:  f64,
}

pub struct Solution {
    pub conditions:   Vec<MarkedStatement>,
    condition_hashes: HashMap<u64, Vec<usize>>,

    pub target: ProblemType,
    pub answer: Option<Arc<Statement>>,

    pub perf_stats: PerfStats,

    rules_engine:       Arc<RulesEngine>,
    local_rules:        Vec<Arc<RwLock<Rule>>>,
    equivalent_targets: Vec<MarkedStatement>,

    subproblem_level: usize,
}

#[derive(Debug)]
pub enum SolutionError {
    StackOverflow,
    MaxSubproblemLevelExceed,
    NoConditions,
    NoSolutionsFound,
}

impl fmt::Display for SolutionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SolutionError::StackOverflow => write!(f, "StackOverflow"),
            SolutionError::MaxSubproblemLevelExceed => write!(f, "Max subproblem level exceed"),
            SolutionError::NoConditions => write!(f, "No conditions"),
            SolutionError::NoSolutionsFound => write!(f, "No solutions found"),
        }
    }
}

impl PerfStats {
    pub fn new(problem: &Problem) -> PerfStats {
        PerfStats {
            problem_hash:   problem.id,
            cycles_count:   0,
            solution_depth: 0,
            absolute_time:  0.,
        }
    }
}

impl Solution {
    pub fn new(problem: &Problem, rules: Arc<RulesEngine>) -> Solution {
        let mut result = Solution {
            target:           problem.target.clone(),
            condition_hashes: HashMap::new(),
            conditions:       vec![],
            answer:           None,

            perf_stats: PerfStats::new(&problem),

            rules_engine:       rules,
            local_rules:        vec![],
            equivalent_targets: vec![],

            subproblem_level: problem.subproblem_level,
        };
        for i in problem.conditions.iter() {
            let _ = result.add_condition(i.clone().normalize());
        }
        result
    }

    pub fn solve(&mut self) -> Result<(), SolutionError> {
        trace!("Subproblem: {}, {:?}", self.target, self.conditions);
        if self.subproblem_level > MAX_SUBPROBLEM_LEVEL {
            return Err(SolutionError::MaxSubproblemLevelExceed);
        }
        let start = Instant::now();
        let result = self.solution_loop();
        self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
        result
    }

    pub fn is_answer(&self, statement: &Statement, weight: &usize) -> bool {
        match &self.target {
            ProblemType::Calculate(x) => {
                let eq_sym = symbol_by_name(&String::from("==")).unwrap().id;
                let in_sym = symbol_by_name(&String::from("in")).unwrap().id;
                if statement.root.degree() != 2 ||
                    (statement.root.root().data != Term::Symbol(eq_sym) &&
                        statement.root.root().data != Term::Symbol(in_sym))
                {
                    return false;
                }
                if statement.root.first().unwrap() == x.root() {
                    let is_id = symbol_by_name(&"is".into()).unwrap().id;
                    let known_id = symbol_by_name(&"known".into()).unwrap().id;

                    let is_known = tr(Term::Symbol(is_id)) /
                        statement.root.last().unwrap().to_owned() /
                        tr(Term::Symbol(known_id));

                    debug!("Attempt to proof: {}", is_known);
                    if self.proof(Arc::new(Statement::from(is_known))) {
                        debug!("Prooved!");
                        return true;
                    } else {
                        debug!("Can't proof!");
                        return false;
                    }
                }
                false
            }
            ProblemType::Proof(x) => {
                if statement.root == x.statement.root {
                    return true;
                }
                if self.is_true(&x.statement.root) {
                    return true;
                }

                for i in self.equivalent_targets.iter() {
                    if statement.root == i.statement.root {
                        return true;
                    }
                    if self.is_true(&i.statement.root) {
                        return true;
                    }
                }
                false
            }
            ProblemType::Transform => {
                trace!("weight: {}", weight);
                return weight == &1;
            }
        }
    }

    pub fn apply(
        &self,
        statement: &MarkedStatement,
        rule: Arc<RwLock<Rule>>,
    ) -> Result<Vec<Statement>, String> {
        trace!(
            "State: {} {:?} {}",
            statement.statement,
            statement.applied_rules,
            statement.weight.borrow()
        );
        trace!(
            "App rule: {}, {:?}",
            rule.read().unwrap().id,
            statement.applied_rules.borrow()
        );
        if !statement
            .applied_rules
            .borrow_mut()
            .insert(rule.read().expect("Cant lock rule").id)
        {
            return Err("Already applied".into());
        }
        if let Ok(new_trees) = rule
            .read()
            .expect("Cant lock rule")
            .apply(&statement.statement.root)
        {
            Ok(new_trees
                .into_iter()
                .filter_map(|(x, reqs)| {
                    for r in reqs {
                        if !self.proof(Arc::new(r)) {
                            return None;
                        }
                    }
                    Some(Statement::from(x).with_rule(rule.clone()))
                })
                .collect())
        } else {
            // if subtree replacement
            let mut new_tree = statement.statement.root.clone();

            if self.subtree_apply(new_tree.root_mut(), rule.clone()) {
                Ok(vec![Statement::from(new_tree).with_rule(rule.clone())])
            } else {
                Err("Rule applied".into())
            }
        }
    }

    fn add_condition(&mut self, statement: MarkedStatement) -> Result<usize, SolutionError> {
        let mut s = DefaultHasher::new();
        statement.statement.hash(&mut s);
        let hash = s.finish();
        trace!("Hashes: {:?}, new: {}", self.condition_hashes, hash);
        for i in self.condition_hashes.entry(hash).or_insert(vec![]) {
            trace!(
                "Compare: {}, {}",
                statement.statement.root,
                self.conditions[*i].statement.root
            );
            if statement.statement.root == self.conditions[*i].statement.root {
                trace!("Same!");
                return Ok(*i);
            }
        }
        trace!("New condition: {}", statement.statement);
        self.condition_hashes
            .get_mut(&hash)
            .unwrap()
            .push(self.conditions.len());
        self.conditions.push(statement);
        if self.conditions.len() > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }

        Ok(self.conditions.len() - 1)
    }

    fn pick_condition(&self) -> Result<usize, SolutionError> {
        trace!("Solution: {:?}", self.conditions);
        let element = self
            .conditions
            .iter()
            .enumerate()
            .max_by(|(_, x), (_, y)| x.weight.cmp(&y.weight))
            .ok_or(SolutionError::NoConditions)?;
        if *element.1.weight.borrow() == 0 {
            trace!("State: {:?} no solution!", element);
            return Err(SolutionError::NoSolutionsFound);
        }
        *element.1.weight.borrow_mut() -= 1;
        Ok(element.0)
    }

    fn solution_loop(&mut self) -> Result<(), SolutionError> {
        self.prepare_target();
        loop {
            self.perf_stats.cycles_count += 1;
            let index = self.pick_condition()?;
            let state = self.conditions.get(index).unwrap();

            trace!("Local rules: {:?}", self.local_rules);
            trace!(
                "Statement: {} ({:?}) ({})",
                state.statement,
                state.applied_rules.borrow(),
                state.weight.borrow()
            );

            if let Some(s) = self.transform(&self.conditions.get(index).unwrap()) {
                self.add_condition(s)?;
                continue;
            }

            if self.is_answer(&state.statement, &*state.weight.borrow()) {
                trace!("Solved. Answer: {}", state.statement);
                self.answer = Some(state.statement.clone());
                return Ok(());
            }

            if let Some(mut r) = state.statement.rule() {
                r.id = (self.local_rules.len() + 1) | 0x80_00_00_00_00_00_00_00;
                state.blocked_rules.borrow_mut().insert(r.id);
                self.local_rules.push(Arc::new(RwLock::new(r)));
            }

            for s in self
                .next_statement(&state, |x| !state.blocked_rules.borrow().contains(&x.id))
                .into_iter()
            {
                unsafe {
                    let sp: *mut Self = self;
                    let state = (*sp).conditions.get(index).unwrap();
                    (*sp).add_condition({
                        let s = MarkedStatement::from(Arc::new(s));
                        s.blocked_rules
                            .borrow_mut()
                            .extend(state.blocked_rules.borrow().iter());
                        s
                    })?;
                }
            }
            self.prepare_target();
        }
    }

    fn subtree_apply(
        &self,
        node: &mut trees::linked::fully::Node<Term>,
        rule: Arc<RwLock<Rule>>,
    ) -> bool {
        let mut applied = false;
        for i in node.iter_mut() {
            applied = applied || self.subtree_apply(i, rule.clone());
            if let Ok(new_sub) = rule.read().expect("Cant lock rule").apply(&i) {
                for (mut variant, reqs) in new_sub {
                    let mut all_prooved = true;
                    for r in reqs {
                        if !self.proof(Arc::new(r)) {
                            all_prooved = false;
                            break;
                        }
                    }
                    if all_prooved {
                        applied = true;
                        // TODO: is multiple replace possible?
                        swap_node(i, &mut variant);
                        break;
                    }
                }
            }
        }
        applied
    }

    fn subproblem(&self, target: ProblemType) -> Problem {
        let conditions = match target {
            ProblemType::Transform => vec![],
            _ => self
                .conditions
                .iter()
                .map(|x| {
                    let replaced = *x.replaced.borrow();
                    let weight = if replaced { 0 } else { DEFAULT_WEIGHT };
                    MarkedStatement {
                        statement:     x.statement.clone(),
                        applied_rules: x.applied_rules.clone(),
                        blocked_rules: x.blocked_rules.clone(),
                        weight:        RefCell::new(weight),
                        replaced:      RefCell::new(replaced),
                        simplified:    x.simplified.clone(),
                    }
                })
                .collect(),
        };
        let problem = Problem {
            id: 0,
            conditions,
            target,
            subproblem_level: self.subproblem_level + 1,
        };
        trace!("New subproblem: {} {}", problem, self.subproblem_level);
        problem
    }

    fn proof(&self, statement: Arc<Statement>) -> bool {
        if is_true(&statement.root) {
            return true;
        }
        let subproblem =
            self.subproblem(ProblemType::Proof(MarkedStatement::from(statement.clone())));
        let mut solution = Solution::new(&subproblem, self.rules_engine.clone());
        if solution.solve().is_err() {
            trace!("Can't proof: {}", statement);
            return false;
        }
        return true;
    }

    fn transform(&self, statement: &MarkedStatement) -> Option<MarkedStatement> {
        match self.target {
            ProblemType::Transform => {
                return None;
            }
            _ => {}
        }
        if *statement.simplified.borrow() {
            return None;
        }
        *statement.simplified.borrow_mut() = true;

        let mut subproblem = self.subproblem(ProblemType::Transform);
        subproblem.conditions.push(MarkedStatement {
            statement:     statement.statement.clone(),
            applied_rules: RefCell::new(HashSet::new()),
            blocked_rules: RefCell::new(HashSet::new()),
            weight:        RefCell::new(DEFAULT_WEIGHT),
            replaced:      RefCell::new(false),
            simplified:    RefCell::new(true),
        });
        let mut solution = Solution::new(&subproblem, self.rules_engine.clone());
        let result = solution.solve();
        if result.is_err() {
            trace!("NOT Simplified {:?}", result);
            return None;
        }
        let result = MarkedStatement::from(solution.answer.expect("No answer statement"));
        *result.simplified.borrow_mut() = true;
        if result.statement.root == statement.statement.root {
            return None;
        }

        *statement.replaced.borrow_mut() = true;
        *statement.weight.borrow_mut() = 0;
        trace!("Simplified: {}", result.statement);

        return Some(result);
    }

    fn prepare_target(&mut self) {
        trace!("Target update");
        let mut alt_targets = vec![];
        match &self.target {
            ProblemType::Proof(x) => {
                for i in std::iter::once(x).chain(self.equivalent_targets.iter()) {
                    for x in self
                        .next_statement(i, |r| {
                            r.attribute(&RuleAttr::Equivalence).is_some() ||
                                r.attribute(&RuleAttr::Subtree).is_some()
                        })
                        .into_iter()
                    {
                        trace!("New alt target: {}", x);
                        alt_targets.push(MarkedStatement::from(Arc::new(x)));
                    }
                }
            }
            _ => {}
        }
        self.equivalent_targets.append(&mut alt_targets);
    }

    fn next_statement<F: Fn(&Rule) -> bool>(
        &self,
        statement: &MarkedStatement,
        rule_filter: F,
    ) -> Vec<Statement> {
        trace!(
            "State: {} {:?} {}",
            statement.statement,
            statement.applied_rules,
            statement.weight.borrow()
        );
        let mut rules = self.rules_engine.find_rules(
            &statement.statement.symbols,
            &statement.applied_rules.borrow(),
            &statement.blocked_rules.borrow(),
            &self.target,
        );
        rules.append(&mut self.local_rules.clone());
        rules.sort_by(|x, y| {
            x.read()
                .expect("Cant lock rule")
                .level
                .cmp(&y.read().expect("Cant lock rule").level)
        });

        for rule in rules
            .iter()
            .filter(|x| rule_filter(&x.read().expect("Cant lock rule")))
        {
            trace!(
                "Rule: ({}) {}",
                rule.read().unwrap().id,
                rule.read().unwrap()
            );
            trace!(
                "State: {} {:?} {}",
                statement.statement,
                statement.applied_rules,
                statement.weight.borrow()
            );
            match self.apply(&statement, rule.clone()) {
                Ok(results) => {
                    if results.len() > 0 &&
                        rule.read()
                            .expect("Cant lock rule")
                            .attribute(&RuleAttr::Replace)
                            .is_some()
                    {
                        *statement.replaced.borrow_mut() = true;
                        *statement.weight.borrow_mut() = 0;
                    }
                    return results
                        .into_iter()
                        .map(|mut s| {
                            normalize(s.root.root_mut());
                            s
                        })
                        .collect();
                }
                Err(e) => {
                    trace!("Cant apply rule: {}", e);
                }
            }
        }
        vec![]
    }

    fn is_true(&self, statement: &StatementTree) -> bool {
        if is_true(statement) {
            return true;
        }
        match statement.data {
            Term::Symbol(_) => false,
            _ => false,
        }
    }
}

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.answer.as_ref() {
            let mut trace: Vec<Arc<Statement>> = vec![];

            fn visitor(trace: &mut Vec<Arc<Statement>>, state: &Arc<Statement>) {
                trace.push(state.clone());
                for i in state.parents.iter() {
                    visitor(trace, i);
                }
            };

            visitor(&mut trace, a);

            while let Some(t) = trace.pop() {
                write!(f, "\n")?;
                for p in t.parents.iter() {
                    write!(f, "{},\n", p.to_string().underline())?;
                }
                write!(f, "{}\n", t.to_string().bold().yellow())?;
            }
            write!(
                f,
                "{} {}\n",
                "SOLVED!".green(),
                format!(
                    "[{} cycles, {}ms]",
                    self.perf_stats.cycles_count, self.perf_stats.absolute_time
                )
                .yellow()
            )
        } else {
            write!(f, "\n")?;
            write!(f, "{}\n", "NOT SOLVED!".bold().blink().red())
        }
    }
}

#[cfg(test)]
mod solution_tests {
    use super::*;
    use bigdecimal::BigDecimal as Decimal;
    use core::symbols::symbols_tests::setup;
    use solver::trees::linked::fully::tr;
    use std::str::FromStr;

    use solver::problem::problem_tests::test_problem;

    #[test]
    fn check_answer_test() {
        setup();

        let rules = Arc::new(RulesEngine::new());
        let problem = test_problem();
        let solution = Solution::new(&problem, rules);
        let statement_answer = Statement::from(
            tr(Term::Symbol(1)) /
                tr(Term::Variable(1)) /
                tr(Term::Number(Decimal::from_str("2").unwrap())),
        );
        let statement_not_answer = Statement::from(
            tr(Term::Symbol(1)) /
                tr(Term::Variable(2)) /
                tr(Term::Number(Decimal::from_str("2").unwrap())),
        );
        assert_eq!(solution.is_answer(&statement_answer, &10), true);
        assert_eq!(solution.is_answer(&statement_not_answer, &10), false);
    }
}
