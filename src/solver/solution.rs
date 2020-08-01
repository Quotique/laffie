use std::{
    cell::RefCell,
    fmt,
    sync::{Arc, RwLock},
    time::Instant,
};
use trees::linked::fully::tr;

use colored::*;

use core::{
    rule::{Rule, RuleFlags, RulesEngine},
    statement::Statement,
    symbols::symbol_by_name,
    term::{StatementTree, Term},
    tree_utils::swap_node,
};

use super::{
    operations::{is_true, normalize},
    problem::{Problem, ProblemType},
};

pub const DEFAULT_WEIGHT: usize = 10;

pub struct PerfStats {
    problem_hash:   u64,
    cycles_count:   usize,
    solution_depth: usize,
    absolute_time:  f64,
}

pub struct Solution {
    pub conditions: Vec<(Arc<Statement>, RefCell<usize>)>,
    pub target:     ProblemType,
    pub answer:     Option<Arc<Statement>>,

    pub perf_stats: PerfStats,

    rules_engine:       Arc<RulesEngine>,
    local_rules:        Vec<Arc<RwLock<Rule>>>,
    equivalent_targets: Vec<Arc<Statement>>,
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
        Solution {
            target:     problem.target.clone(),
            conditions: problem
                .conditions
                .iter()
                .map(|x| {
                    let mut s = (**x).clone();
                    normalize(s.root.root_mut());

                    (Arc::new(s), RefCell::new(DEFAULT_WEIGHT))
                })
                .collect(),
            answer:     None,

            perf_stats: PerfStats::new(&problem),

            rules_engine:       rules,
            local_rules:        vec![],
            equivalent_targets: vec![],
        }
    }

    pub fn solve(&mut self) -> Result<(), String> {
        let start = Instant::now();
        loop {
            self.perf_stats.cycles_count += 1;
            match self.conditions.iter().max_by(|x, y| x.1.cmp(&y.1)) {
                Some((state, weight)) => {
                    trace!("Statement: {} ({})", state, weight.borrow());
                    trace!("Local rules: {:?}", self.local_rules);
                    if *weight.borrow() == 0 {
                        self.perf_stats.absolute_time =
                            (start.elapsed().as_nanos() as f64) / 1000000.;
                        return Err("No solution found".into());
                    }
                    *weight.borrow_mut() -= 1;
                    if self.is_answer(&state, &*weight.borrow()) {
                        self.answer = Some(state.clone());
                        self.perf_stats.absolute_time =
                            (start.elapsed().as_nanos() as f64) / 1000000.;
                        return Ok(());
                    }

                    if let Some(mut r) = state.rule() {
                        r.id = (self.local_rules.len() + 1) | 0x80_00_00_00_00_00_00_00;
                        state.block_rule(r.id); // disable self-apply
                        self.local_rules.push(Arc::new(RwLock::new(r)));
                    }

                    for s in self.next_statement(state.clone(), |_| true).into_iter() {
                        self.conditions
                            .push((Arc::new(s), RefCell::new(DEFAULT_WEIGHT)));
                    }
                    if self.conditions.len() > 20 {
                        self.perf_stats.absolute_time =
                            (start.elapsed().as_nanos() as f64) / 1000000.;
                        return Err("Stack overflow".into());
                    }
                    self.prepare_target();
                }
                None => {
                    self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
                    return Err("Conditions not found".into());
                }
            }
        }
    }

    pub fn is_answer(&self, statement: &Statement, weight: &usize) -> bool {
        match &self.target {
            ProblemType::Calculate(x) => {
                let eq_sym = symbol_by_name(&String::from("==")).unwrap().id;
                if statement.root.degree() != 2 ||
                    statement.root.root().data != Term::Symbol(eq_sym)
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
                if statement.root == x.root {
                    return true;
                }
                if self.is_true(&x.root) {
                    return true;
                }

                for i in self.equivalent_targets.iter() {
                    if statement.root == i.root {
                        return true;
                    }
                    if self.is_true(&i.root) {
                        return true;
                    }
                }
                false
            }
            ProblemType::Transform => {
                return weight == &1;
            }
        }
    }

    pub fn apply(
        &self,
        statement: Arc<Statement>,
        rule: Arc<RwLock<Rule>>,
    ) -> Result<Vec<Statement>, String> {
        if !statement
            .applied_rules
            .borrow_mut()
            .insert(rule.read().expect("Cant lock rule").id)
        {
            return Err("Already applied".into());
        }
        if let Ok(new_trees) = rule.read().expect("Cant lock rule").apply(&statement.root) {
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
            let mut new_tree = statement.root.clone();

            if self.subtree_apply(new_tree.root_mut(), rule.clone()) {
                Ok(vec![Statement::from(new_tree).with_rule(rule.clone())])
            } else {
                Err("Rule applied".into())
            }
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
                    for r in reqs {
                        if !self.proof(Arc::new(r)) {
                            continue;
                        }
                    }
                    applied = true;
                    // TODO: is multiple replace possible?
                    swap_node(i, &mut variant);
                    break;
                }
            }
        }
        applied
    }

    fn subproblem(&self, target: ProblemType) -> Problem {
        Problem {
            id: 0,
            conditions: self.conditions.iter().map(|(x, _)| x.clone()).collect(),
            target,
        }
    }

    fn proof(&self, statement: Arc<Statement>) -> bool {
        let sub_p = self.subproblem(ProblemType::Proof(statement.clone()));
        let mut sol = Solution::new(&sub_p, self.rules_engine.clone());
        if sol.solve().is_err() {
            trace!("Can't proof: {}", statement);
            return false;
        }
        return true;
    }

    fn prepare_target(&mut self) {
        let mut alt_targets = vec![];
        match &self.target {
            ProblemType::Proof(x) => {
                for i in std::iter::once(x).chain(self.equivalent_targets.iter()) {
                    for x in self
                        .next_statement(i.clone(), |r| {
                            r.flags.contains(RuleFlags::EQUIVALENCE) |
                                r.flags.contains(RuleFlags::SUBTREE_REPLACEMENT)
                        })
                        .into_iter()
                    {
                        alt_targets.push(Arc::new(x));
                    }
                }
            }
            _ => {}
        }
        self.equivalent_targets.append(&mut alt_targets);
    }

    fn next_statement<F: Fn(&Rule) -> bool>(
        &self,
        statement: Arc<Statement>,
        rule_filter: F,
    ) -> Vec<Statement> {
        let mut rules = self.rules_engine.find_rules(&statement.symbols);
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
            trace!("Rule: {}", rule.read().unwrap());
            match self.apply(statement.clone(), rule.clone()) {
                Ok(results) => {
                    return results
                        .into_iter()
                        .map(|mut s| {
                            normalize(s.root.root_mut());
                            s
                        })
                        .collect()
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
