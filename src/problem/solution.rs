use super::{
    frame::Frame,
    problem::{Problem, ProblemBuilder},
};
use crate::{
    core::{
        term::{StatementTree, Term},
        tree_utils::swap_node,
    },
    rule::{Rule, RuleAttr, RulesEngine, SharedRule},
    solver::operations::{is_true, normalize},
    statement::{MarkedStatement, Statement},
    utils::Dumper,
};
use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    convert::From,
    fmt,
    iter::FromIterator,
    rc::Rc,
    sync::{Arc, RwLock},
    time::Instant,
};
use trees::{tr, Node};

use colored::*;

pub const MAX_SUBPROBLEM_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;

pub struct PerfStats {
    problem_hash:   u64,
    cycles_count:   usize,
    solution_depth: usize,
    absolute_time:  f64,
}

pub struct Solution {
    pub problem: Problem,

    stack: Frame,

    rules_engine:       Arc<RulesEngine>,
    local_rules:        Vec<SharedRule>,
    equivalent_targets: Frame,
    pub answer:         Option<usize>,

    pub perf_stats: PerfStats,

    dumper: Option<Rc<RefCell<Box<dyn Dumper + 'static>>>>,
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
            SolutionError::MaxSubproblemLevelExceed => write!(f, "MaxSubproblemLevelExceed"),
            SolutionError::NoConditions => write!(f, "NoConditions"),
            SolutionError::NoSolutionsFound => write!(f, "NoSolutionsFound"),
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
    pub fn new(problem: Problem, rules: Arc<RulesEngine>) -> Solution {
        let mut targets = Frame::default();
        targets.add_condition(problem.target.clone());
        Solution {
            stack: Frame::from_iter(problem.conditions.iter().cloned()),

            rules_engine:       rules,
            local_rules:        vec![],
            equivalent_targets: targets,

            answer: None,

            perf_stats: PerfStats::new(&problem),

            dumper: None,

            problem: problem,
        }
    }

    pub fn answer(&self) -> Option<Arc<Statement>> {
        self.answer.map(|i| self.stack[i].statement.clone())
    }

    pub fn set_dumper(&mut self, dumper: Rc<RefCell<Box<dyn Dumper>>>) {
        self.dumper = Some(dumper);
    }

    pub fn solve(&mut self) -> Result<(), SolutionError> {
        if let Some(x) = self.dumper.as_ref() {
            x.borrow_mut().subproblem_start(&self);
        }
        // trace!("Subproblem: {}, {:?}", self.target, self.conditions);
        if self.problem.subproblem_level > MAX_SUBPROBLEM_LEVEL {
            return Err(SolutionError::MaxSubproblemLevelExceed);
        }

        let start = Instant::now();
        let result = self.solution_loop();

        self.perf_stats.absolute_time = (start.elapsed().as_nanos() as f64) / 1000000.;
        if let Some(x) = self.dumper.as_ref() {
            x.borrow_mut().subproblem_end()
        }
        result
    }

    fn pick_condition(&self) -> Result<usize, SolutionError> {
        self.stack
            .iter()
            .enumerate()
            .min_by_key(|(_, x)| x.weight)
            .map(|(num, _)| num)
            .ok_or(SolutionError::NoConditions)
    }

    fn solution_loop(&mut self) -> Result<(), SolutionError> {
        loop {
            self.perf_stats.cycles_count += 1;

            let index = self.stack.pick_condition()?;
            let level = self.stack[index].weight;
            trace!("Level: {}", level);
            if level > MAX_LEVEL {
                return Err(SolutionError::NoSolutionsFound);
            }
            self.prepare_target(level);

            if self.is_answer(&self.stack[index]) {
                trace!("Solved. Answer: {}", self.stack[index].statement);
                self.answer = Some(index);
                return Ok(());
            }
            if let Some(r) = self.stack[index].rule(
                (self.local_rules.len() + 1) | 0x80_00_00_00_00_00_00_00,
                (level + 1) as u64,
            ) {
                self.local_rules.push(r);
            }

            let statements = self.next_statement(index);
            if statements.is_empty() {
                self.stack[index].weight += 1;
            }
            for s in statements {
                self.stack.add_condition(s)?;
            }
        }
    }

    fn prepare_target(&mut self, level: usize) {
        let target_root = self.problem.target.statement.root();

        if target_root.data.is_symbol_name(&"find".into()) {
        } else if target_root.data.is_symbol_name(&"proof".into()) {
            loop {
                if let Ok(index) = self.equivalent_targets.pick_condition() {
                    if self.equivalent_targets[index].weight > level {
                        return;
                    }
                    let mut rules = self
                        .rules_engine
                        .suggest_rules(&self.equivalent_targets[index], &self.problem.target);
                    rules.append(&mut self.local_rules.clone());
                    let mut applied = false;
                    for rule in rules.iter().filter(|r| {
                        r.read()
                            .expect("Can't read rule")
                            .attribute(&RuleAttr::Equivalence)
                            .is_some()
                    }) {
                        if let Ok(result) = rule
                            .read()
                            .expect("Can't read rule")
                            .apply(&mut self.equivalent_targets[index], &self.problem.target)
                        {
                            for sup in result {
                                let mut proofed = true;
                                if self.equivalent_targets.contains(&sup.resolution.statement) {
                                    continue;
                                }

                                for req in sup.requirements {
                                    if !self.proof(req) {
                                        proofed = false;
                                        break;
                                    }
                                }

                                if proofed {
                                    applied = true;
                                    trace!("New alternative target: {}", sup.resolution);
                                    self.equivalent_targets.add_condition(sup.resolution);
                                }
                            }
                        }
                    }
                    if !applied {
                        self.equivalent_targets[index].weight += 1;
                    }
                }
            }
        } else if target_root.data.is_symbol_name(&"transform".into()) {
        }
    }

    fn is_answer(&self, statement: &MarkedStatement) -> bool {
        let target_root = self.problem.target.statement.root();
        let statement_root = statement.statement.root();

        if target_root.data.is_symbol_name(&"find".into()) {
            if statement_root.degree() != 2 ||
                (!statement_root.data.is_symbol_name(&"==".into()) &&
                    !statement_root.data.is_symbol_name(&"in".into()))
            {
                return false;
            }

            if statement_root.first().unwrap() == target_root.first().unwrap() {
                let is_known = tr(Term::with_symbol_name("is").unwrap()) /
                    statement_root.last().unwrap().to_owned() /
                    tr(Term::with_symbol_name("known").unwrap());

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
        } else if target_root.data.is_symbol_name(&"proof".into()) {
            for i in self.equivalent_targets.iter() {
                if statement_root == i.statement.root().first().unwrap() {
                    return true;
                }
                if self.is_true(&i.statement.root().first().unwrap()) {
                    return true;
                }
            }
            false
        } else if target_root.data.is_symbol_name(&"transform".into()) {
            // 		   trace!("weight: {}", weight);
            // 		   return weight == &1;
            false
        } else {
            false
        }
    }

    fn proof(&self, statement: Arc<Statement>) -> bool {
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
        if let Some(x) = self.dumper.as_ref() {
            solution.set_dumper(x.clone());
        }

        if solution.solve().is_err() {
            trace!("Can't proof: {}", statement);
            return false;
        }
        return true;
    }

    fn is_true(&self, statement: &Node<Term>) -> bool {
        if is_true(statement) {
            return true;
        }
        match statement.data {
            Term::Symbol(_) => false,
            _ => false,
        }
    }

    fn next_statement(&mut self, index: usize) -> Vec<MarkedStatement> {
        let mut rules = self
            .rules_engine
            .suggest_rules(&self.stack[index], &self.problem.target);
        rules.append(&mut self.local_rules.clone());
        for rule in rules {
            if let Ok(result) = rule
                .read()
                .expect("Can't read rule")
                .apply(&mut self.stack[index], &self.problem.target)
            {
                let mut res = vec![];
                for sup in result {
                    let mut proofed = true;
                    if self.stack.contains(&sup.resolution.statement) {
                        continue;
                    }

                    for req in sup.requirements {
                        if !self.proof(req) {
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

    fn transform(&self, statement: &MarkedStatement) -> Option<MarkedStatement> {
        None
        // 	  match self.target {
        // 		  ProblemType::Transform => {
        // 			  return None;
        // 		  }
        // 		  _ => {}
        // 	  }
        // 	  if *statement.simplified.borrow() {
        // 		  return None;
        // 	  }
        // 	  *statement.simplified.borrow_mut() = true;
        //
        // 	  let mut subproblem = self.subproblem(ProblemType::Transform);
        // 	  subproblem.conditions.push(MarkedStatement {
        // 		  statement:	 statement.statement.clone(),
        // 		  applied_rules: RefCell::new(HashSet::new()),
        // 		  blocked_rules: RefCell::new(HashSet::new()),
        // 		  weight:		 RefCell::new(DEFAULT_WEIGHT),
        // 		  replaced:		 RefCell::new(false),
        // 		  simplified:	 RefCell::new(true),
        // 	  });
        // 	  let mut solution = Solution::new(&subproblem,
        // self.rules_engine.clone());	  if let Some(x) =
        // self.dumper.as_ref() {		 solution.add_dumper(x.clone());
        // 	  }
        //
        // 	  let result = solution.solve();
        // 	  if result.is_err() {
        // 		  trace!("NOT Simplified {:?}", result);
        // 		  return None;
        // 	  }
        // 	  let result = MarkedStatement::from(solution.answer.expect("No
        // answer statement"));    *result.simplified.borrow_mut() =
        // true;	if result.statement.root == statement.statement.root
        // {		return None;
        // 	  }
        //
        // 	  *statement.replaced.borrow_mut() = true;
        // 	  *statement.weight.borrow_mut() = 0;
        // 	  trace!("Simplified: {}", result.statement);
        //
        // 	  return Some(result);
    }
}

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.answer.as_ref() {
            let mut trace: Vec<(usize, Vec<usize>)> = vec![];

            fn visitor(stack: &Frame, id: usize, trace: &mut Vec<(usize, Vec<usize>)>) {
                trace.push((id, stack[id].parents.clone()));
                for i in stack[id].parents.iter() {
                    visitor(stack, *i, trace);
                }
            };

            visitor(&self.stack, *a, &mut trace);

            while let Some(t) = trace.pop() {
                write!(f, "\n")?;
                for p in t.1.iter() {
                    write!(f, "{},\n", self.stack[*p].statement.to_string().underline())?;
                }
                write!(
                    f,
                    "{}\n",
                    self.stack[t.0].statement.to_string().bold().yellow()
                )?;
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
    use crate::{
        parser::{parse_problem, statement_with_vars},
        predefine::setup,
    };

    #[test]
    fn check_answer_find_test() {
        setup();

        let problem = parse_problem("problem {x == 1; target find x;};");
        let rules = Arc::new(RulesEngine::new());
        let mut solution = Solution::new(problem, rules);
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), statement_with_vars("x == 1"));
    }

    #[test]
    fn check_answer_proof_test() {
        setup();

        let problem = parse_problem("problem { x == 2; target proof (x > 0); }; ");
        let rules = Arc::new(RulesEngine::new());
        let mut solution = Solution::new(problem, rules);
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), statement_with_vars("x == 2"));
    }
}
