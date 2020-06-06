use std::{
    cell::RefCell,
    fmt,
    sync::{Arc, RwLock},
};

use colored::*;

use core::{
    rule::{Rule, RuleFlags, RulesEngine},
    statement::Statement,
    symbols::symbol_by_name,
    term::{StatementTree, Term},
};

use super::{
    operations::{is_true, normalize},
    problem::{Problem, ProblemType},
};

pub const DEFAULT_WEIGHT: usize = 10;

pub struct Solution {
    pub conditions: Vec<(Arc<Statement>, RefCell<usize>)>,
    pub target:     ProblemType,
    pub answer:     Option<Arc<Statement>>,

    rules_engine:       Arc<RulesEngine>,
    local_rules:        Vec<Arc<RwLock<Rule>>>,
    equivalent_targets: Vec<Arc<Statement>>,
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

            rules_engine:       rules,
            local_rules:        vec![],
            equivalent_targets: vec![],
        }
    }

    pub fn solve(&mut self) -> Result<(), String> {
        loop {
            match self.conditions.iter().max_by(|x, y| x.1.cmp(&y.1)) {
                Some((state, weight)) => {
                    trace!("Statement: {} ({})", state, weight.borrow());
                    trace!("Local rules: {:?}", self.local_rules);
                    if *weight.borrow() == 0 {
                        return Err("No solution found".into());
                    }
                    *weight.borrow_mut() -= 1;
                    if self.is_answer(&state, &*weight.borrow()) {
                        self.answer = Some(state.clone());
                        return Ok(());
                    }

                    if let Some(mut r) = state.rule() {
                        r.id = (self.local_rules.len() + 1) | 0x80_00_00_00_00_00_00_00;
                        state.block_rule(r.id); // disable self-apply
                        self.local_rules.push(Arc::new(RwLock::new(r)));
                    }

                    if let Some(s) = self.next_statement(state.clone(), |_| true) {
                        self.conditions
                            .push((Arc::new(s), RefCell::new(DEFAULT_WEIGHT)));
                    }
                    self.prepare_target();
                }
                None => return Err("Conditions not found".into()),
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
                statement.root.first().unwrap() == x.root()
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

    fn subproblem(&self, target: ProblemType) -> Problem {
        Problem {
            conditions: self.conditions.iter().map(|(x, _)| x.clone()).collect(),
            target,
        }
    }

    fn prepare_target(&mut self) {
        let mut alt_targets = vec![];
        match &self.target {
            ProblemType::Proof(x) => {
                for i in std::iter::once(x).chain(self.equivalent_targets.iter()) {
                    while let Some(x) = self.next_statement(i.clone(), |r| {
                        r.flags.contains(RuleFlags::EQUIVALENCE) |
                            r.flags.contains(RuleFlags::SUBTREE_REPLACEMENT)
                    }) {
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
    ) -> Option<Statement> {
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
            trace!("Rule: {:?}", rule);
            match Statement::apply(statement.clone(), rule.clone()) {
                Ok(mut s) => {
                    for r in rule
                        .read()
                        .expect("Unable to lock rule")
                        .requirements
                        .iter()
                    {
                        let sub_p = self.subproblem(ProblemType::Proof(r.clone()));
                        let mut sol = Solution::new(&sub_p, self.rules_engine.clone());
                        if sol.solve().is_err() {
                            trace!("Can't proof: {}", r);
                            continue;
                        }
                    }

                    normalize(s.root.root_mut());
                    return Some(s);
                }
                Err(e) => {
                    trace!("Cant apply rule: {}", e);
                }
            }
        }
        None
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
            write!(f, "{}\n", "SOLVED!".green())
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
