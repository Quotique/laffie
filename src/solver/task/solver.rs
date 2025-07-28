use std::sync::Arc;

use itertools::Itertools;

use super::{
    props::TermInference, Purpose, SharedSolution, Solution, SolutionTracer, SolveError, Task,
    TaskBuilder, TasksCache, TermIdx, TermProps,
};
use crate::{
    rule::{Hypothesis, HypothesisIterator, RuleAttr, RuleId, RulesEngine, SharedRule},
    task::solution::SolutionStatus,
    term::{SharedTerm, SubtermMut, Term},
    NormalizationLevel,
};

pub const MAX_SUBTASK_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

pub struct Solver {
    local_rules: Vec<SharedRule>,

    rules_engine:       Arc<RulesEngine>,
    execution_deadline: usize,

    cycle_counter: usize,
    cache:         Arc<TasksCache>,
    tracer:        SolutionTracer,
}

impl Solver {
    pub fn new(
        rules: Arc<RulesEngine>,
        tracer: SolutionTracer,
        execution_deadline: usize,
    ) -> Solver {
        Solver {
            local_rules: vec![],

            rules_engine: rules.clone(),
            execution_deadline,
            cycle_counter: Default::default(),
            cache: Default::default(),
            tracer,
        }
    }

    pub fn solve(&mut self, task: Task) -> SharedSolution {
        let mut solution = Solution::new(task);

        if solution.task.subtask_level > MAX_SUBTASK_LEVEL {
            solution.status = SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed);
            return solution.into();
        }
        self.tracer
            .on_subtask_start(&solution.task, self.current_cycle());

        solution.start_cycle = self.cycle_counter;

        // TODO: can be replaced with try { .. } in future
        // track: https://github.com/rust-lang/rust/issues/31436
        let mut main_loop = || loop {
            self.increment_cycle_counter()?;
            let index = self.try_focus_term(&solution)?;
            if self.try_simplify(index, &mut solution)? {
                continue;
            }
            if self.check_if_answer(&mut solution, index) {
                let level = solution.task.subtask_level;
                let answer = solution.answer().unwrap();
                trace!("Solved {level}. Answer: {answer}",);
                break Ok(());
            }
            self.add_local_rule(&mut solution[index]);
            self.try_infer_new_terms(index, &mut solution)?;
        };
        if let Err(e) = main_loop() {
            solution.status = SolutionStatus::Err(e);
        }
        solution.end_cycle = self.cycle_counter;
        self.tracer.clone().on_subtask_end(&solution);

        solution.into()
    }

    fn try_focus_term(&self, solution: &Solution) -> Result<TermIdx, SolveError> {
        let index = solution.pick_next().ok_or(SolveError::NoConditions)?;
        self.tracer.on_term_focus(&solution[index]);
        let level = solution[index].filters.weight;

        trace!(target: "subtask",
            "[{}]({}) Level: {level} -> {}",
            solution.task.subtask_level,
            self.current_cycle(),
            solution[index]
        );

        if level > MAX_LEVEL {
            return Err(SolveError::NoSolutionsFound);
        }
        Ok(index)
    }

    fn try_simplify(&mut self, index: usize, solution: &mut Solution) -> Result<bool, SolveError> {
        if solution.purpose.is_transform() {
            Ok(false)
        } else if let Some(simplified) = self.transform(solution, index) {
            solution[index].filters.mark_replaced();
            self.add_term(simplified, solution)?;
            Ok(true)
        } else {
            solution[index].filters.mark_simplified();
            Ok(false)
        }
    }

    fn add_term(&mut self, term: TermProps, s: &mut Solution) -> Result<TermIdx, SolveError> {
        self.tracer.on_new_term(
            &term,
            &term
                .inference
                .parent_id()
                .map(|parent| s[parent].clone())
                .unwrap_or_else(|| TermProps::from(Term::zero())),
        );

        let is_purpose = term.filters.is_purpose();
        let index = s.add_term(term)?;
        if !is_purpose {
            self.add_local_rule(&mut s[index]);
        }
        Ok(index)
    }

    fn add_local_rule(&mut self, term: &mut TermProps) {
        if term.filters.is_purpose() {
            return;
        }
        let level = term.filters.weight;
        if let Some(r) = term.rule(
            RuleId::new(0x80_00_00_00_00_00_00_00, self.local_rules.len() as u64 + 1),
            (level + 1) as u64,
        ) {
            // TODO: check dups
            self.local_rules.push(r);
        }
    }

    // TODO: единый обьект для purpose
    fn suggest_rules(
        &self,
        term: &TermProps,
        purpose_term: &TermProps,
        purpose: &Purpose,
    ) -> Vec<SharedRule> {
        if purpose.is_transform() && !term.filters.is_purpose() {
            return vec![];
        }

        let local_rules = self.local_rules.iter().unique();
        let local_rules = local_rules
            .filter(|rule| rule.try_filter(&term.filters, &purpose_term.term).is_ok())
            .cloned();
        let rules = self
            .rules_engine
            .suggest_rules(&term.filters, &purpose_term.term);
        let rules = rules.into_iter().chain(local_rules);

        let rules: Vec<_> = if purpose.is_proof() && term.filters.is_purpose() {
            rules
                .filter(|rule| rule.contains_attribute(&RuleAttr::Equivalence))
                .collect()
        } else {
            rules.collect()
        };

        trace!(target: "rule_selection",
            "purpose: {purpose_term}, term: {term}, suggested_rules: {}",
            rules.iter().format(", ")
        );
        rules
    }

    fn try_infer_new_terms(
        &mut self,
        index: TermIdx,
        solution: &mut Solution,
    ) -> Result<(), SolveError> {
        let is_purpose = solution[index].filters.is_purpose();
        let proof_purpose = TermProps::from(
            Term::symbol("proof").with_child(solution[index].term.as_ref().clone()),
        );

        let mut added = false;
        for rule in self.suggest_rules(
            &solution[index],
            if is_purpose && solution.purpose.is_proof() {
                &proof_purpose
            } else {
                &solution.task.purpose
            },
            &solution.purpose,
        ) {
            match self.produce(&rule, solution, index) {
                Some(s) => {
                    trace!("{} => {s}", solution[index]);
                    self.add_term(s, solution)?;
                    if is_purpose && solution.purpose.is_transform() {
                        // TODO: унифицировать weight = MAX_LEVEL и REPLACED
                        solution[index].filters.weight = MAX_LEVEL + 1;
                        break;
                    }
                    added = true;
                }
                None => {
                    solution[index].filters.applied_rules.insert(rule.id);
                }
            }
        }
        if !added {
            solution[index].filters.weight += 1;
        }
        Ok(())
    }

    fn produce(&self, rule: &SharedRule, s: &mut Solution, index: TermIdx) -> Option<TermProps> {
        let is_purpose = s[index].filters.is_purpose();
        let proof_purpose = Term::symbol("proof").with_child(s[index].term.as_ref().clone());
        HypothesisIterator::new(
            rule.clone(),
            &s[index].term,
            &s[index].filters,
            if is_purpose && s.purpose.is_proof() {
                &proof_purpose
            } else {
                &s.task.purpose.term
            },
        )
        .filter_map(|hypothesis| {
            let is_dub = if is_purpose {
                s.purpose_index.contains_key(&hypothesis.resolution)
            } else {
                s.main_index.contains_key(&hypothesis.resolution)
            };
            if is_dub {
                return None;
            }
            let props = self.try_prove_hypothesis(index, s, hypothesis);
            if props.inference.is_proven() {
                Some(props)
            } else {
                // TODO: option to disable
                s.add_term(props).expect("can't add unproven");
                None
            }
        })
        .next()
    }

    fn try_prove_hypothesis(
        &self,
        parent_idx: usize,
        solution: &Solution,
        hypothesis: Hypothesis,
    ) -> TermProps {
        trace!(
            target: "rule_selection",
            "new hypothesis {hypothesis}, rule {}, term: {}",
            hypothesis.rule, solution[parent_idx]
        );
        self.tracer.on_new_hypothesis(
            solution[parent_idx].term.clone(),
            hypothesis.rule.clone(),
            &hypothesis,
            self.current_cycle(),
        );

        let mut props = TermProps::from(hypothesis.resolution.clone());
        props.filters.blocked_rules = hypothesis.blocked_rules.iter().cloned().collect();
        if solution[parent_idx].filters.is_purpose() {
            props.filters.mark_purpose();
        }
        let mut proof_res = 0;
        let mut requirements = vec![];
        let mut iter = hypothesis.requirements.clone().into_iter();
        for i in iter.by_ref() {
            requirements.push(self.proof(solution, i));
            let last = requirements.last().unwrap();
            if last.answer().is_none() {
                trace!(
                    target: "rule_selection",
                    "term {} rejected, requirement not proven {}",
                    hypothesis.resolution,
                    last.task.purpose
                );
                break;
            }
            proof_res += 1;
        }
        for req in iter {
            requirements.push(SharedSolution::new(Solution::new(Task::from(
                TermProps::from(Term::symbol("proof").with_child(req)),
            ))));
        }
        props.inference = TermInference::Rule {
            rule: hypothesis.rule.clone(),
            params: hypothesis.params.clone(),
            parent: parent_idx,
            requirements,
        };

        self.tracer
            .on_hypothesis_finish(&hypothesis, self.current_cycle(), proof_res);

        if props.inference.is_proven() {
            trace!(
                target: "rule_selection",
                "hypothesis {hypothesis} proven, resolution {} applied",
                hypothesis.resolution
            );
        }
        props
    }

    fn proof(&self, solution: &Solution, mut term: Term) -> SharedSolution {
        is_replace(&mut term.as_subterm_mut());
        let term = term.normalize(NormalizationLevel::max());
        let proof_purpose = SharedTerm::new(Term::symbol("proof").with_child(term));
        // TODO: fast check truth
        self.solve_subtask(solution, proof_purpose)
    }

    fn transform(&mut self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        let term = &mut solution[index];
        if term.filters.is_simplified() {
            return None;
        }
        term.filters.mark_simplified();

        let use_answer = term.term.as_subterm().data().is_symbol_name("answer");
        let to_transform = if use_answer {
            term.term.as_subterm().first_arg().unwrap().to_term()
        } else {
            term.term.as_subterm().to_term()
        };
        let task = SharedTerm::new(Term::symbol("transform").with_child(to_transform));
        let subtask_solution = self.solve_subtask(solution, task.clone());

        let mut answer = subtask_solution.answer()?.as_ref().clone();
        if use_answer {
            answer = Term::symbol("answer").with_child(answer);
        }

        if *solution[index].term == answer {
            return None;
        }
        let mut result = TermProps::from(answer);
        result.inference = TermInference::Transform {
            parent:   index,
            solution: subtask_solution,
        };
        result
            .filters
            .blocked_rules
            .clone_from(&solution[index].filters.blocked_rules);
        result.filters.mark_simplified();
        if solution[index].filters.is_purpose() {
            result.filters.mark_purpose();
        }

        Some(result)
    }

    fn solve_subtask(&self, solution: &Solution, task: SharedTerm) -> SharedSolution {
        if let Some(x) = self.cache.status(&task) {
            return x.clone();
        }

        if !self.cache.add(task.as_subterm().to_term()) {
            // TODO: recursion
            unimplemented!("subtask recursion");
        }

        let mut subtask_solver = Solver::new(
            self.rules_engine.clone(),
            self.tracer.clone(),
            self.execution_deadline,
        );
        subtask_solver.cycle_counter = self.cycle_counter;
        subtask_solver.cache = self.cache.clone();

        let subtask = Self::subtask(solution, task.clone());
        let subtask_solution = subtask_solver.solve(subtask);
        self.cache.update_status(&task, subtask_solution.clone());
        if let SolutionStatus::Err(e) = subtask_solution.status {
            trace!("Can't proof {task}: {e}");
            if e == SolveError::MaxSubtaskLevelExceed {
                self.cache.remove(&task);
            }
        }
        subtask_solution
    }

    fn subtask(solution: &Solution, task: SharedTerm) -> Task {
        TaskBuilder::default()
            .with_purpose(TermProps::from(task.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                solution
                    .terms
                    .iter()
                    .filter(|x| x.inference.is_proven())
                    .filter(|x| {
                        !(x.filters.is_purpose() ||
                            x.term.as_subterm().data().is_symbol_name("answer"))
                    })
                    .cloned(),
            )
            .with_level(solution.task.subtask_level + 1)
            .build()
            .expect("Can't build subtask")
    }

    fn check_if_answer(&self, solution: &mut Solution, index: usize) -> bool {
        if solution[index].filters.is_purpose() {
            return false;
        }
        if self.check_answer_term(solution, index) {
            return true;
        }

        match solution.purpose.clone() {
            Purpose::Find(x) => self.check_find_answer(solution, index, x.term.as_ref()),
            Purpose::Proof(_) => self.check_proof_answer(solution, index),
            Purpose::Transform(_) => self.check_transform_answer(solution),
        }
    }

    fn check_answer_term(&self, solution: &mut Solution, index: usize) -> bool {
        let term = &solution[index];
        let term_root = term.term.as_subterm();

        if solution.purpose.is_transform() {
            return false;
        }
        if term_root.data().is_symbol_name("answer") && term_root.degree() == 1 {
            let mut resolution =
                TermProps::from(term.term.as_subterm().first_arg().unwrap().to_term());
            resolution.inference = term.inference.clone();
            // TODO: remove unwrap
            let idx = solution.add_term(resolution).unwrap();
            solution.status = SolutionStatus::Answer(idx);
            return true;
        }
        false
    }

    fn check_find_answer(&self, solution: &mut Solution, index: usize, find: &Term) -> bool {
        let term = solution[index].term.as_subterm();

        if term.degree() != 2 {
            return false;
        }
        if !term.data().is_symbol_name("==") && !term.data().is_symbol_name("in") {
            return false;
        }

        if term.first_arg().unwrap() == find.as_subterm() {
            let is_known = Term::symbol("is")
                .with_child(term.last_arg().unwrap().to_term())
                .with_child(Term::symbol("known"));
            if self.proof(solution, is_known).answer().is_some() {
                solution.status = SolutionStatus::Answer(index);
                return true;
            }
        }
        false
    }

    fn check_proof_answer(&self, solution: &mut Solution, index: usize) -> bool {
        let term = &solution[index];

        // TODO: теперь тут бывают целевые термы, поэтому надо сделать две проверки:
        // что терм есть среди целей
        // что цель тривиальная истина
        for i in solution.purpose_index.values() {
            if term.term == solution[*i].term {
                solution.status = SolutionStatus::Answer(index);
                return true;
            }
            if solution[*i].term.as_subterm().truth().is_true() {
                solution.status = SolutionStatus::Answer(*i);
                // TODO: надо заполнить правильно
                // согласовать с выводом решения по шагам
                // res.inference = TermInference::Condition;
                return true;
            }
        }
        false
    }

    fn check_transform_answer(&self, solution: &mut Solution) -> bool {
        let Some(index) = solution.pick_purpose_term() else {
            return false;
        };

        if solution[index].filters.weight >= MAX_LEVEL {
            let mut iter = solution
                .terms
                .iter()
                .rev()
                .filter(|x| x.inference.is_proven());
            let res = iter.find(|x| x.filters.is_purpose()).map(|x| x.id).unwrap();
            // TODO: надо заполнить правильно
            // согласовать с выводом решения по шагам
            // res.inference = TermInference::Condition;
            solution.status = SolutionStatus::Answer(res);
            return true;
        }
        false
    }

    fn current_cycle(&self) -> usize {
        self.cycle_counter
    }

    fn increment_cycle_counter(&mut self) -> Result<(), SolveError> {
        self.cycle_counter += 1;
        if self.current_cycle() > self.execution_deadline {
            return Err(SolveError::ExecutionDeadline);
        }
        Ok(())
    }
}

fn is_replace(root: &mut SubtermMut) {
    if !root.data().is_symbol_name("is") || root.degree() != 2 {
        return;
    }

    match root.last_arg().unwrap().data().symbol() {
        Some(name) if name == "true" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.as_subterm_mut());
        }
        Some(name) if name == "false" => {
            let child = root.pop_first_arg().unwrap();
            root.swap(&mut Term::symbol("!").with_child(child).as_subterm_mut());
        }
        _ => {}
    }
}

#[cfg(test)]
mod solution_tests {
    use std::sync::Arc;

    use super::SolutionTracer;
    use crate::{
        rule::RulesEngine,
        task::{parse_task, Solver},
        term::term_with_vars,
    };

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task {purpose find(x); x == 1;}");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules, SolutionTracer::default(), usize::MAX);
        let solution = solver.solve(task);
        assert_eq!(
            *solution.answer().expect("task is not solved"),
            term_with_vars("x == 1")
        );
    }

    #[test]
    fn check_answer_proof_test() {
        let task = parse_task("task { purpose proof(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules, SolutionTracer::default(), usize::MAX);
        let solution = solver.solve(task);
        assert!(solution.answer().is_some());
    }
}
