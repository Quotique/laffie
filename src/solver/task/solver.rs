use std::{collections::HashMap, sync::Arc};

use itertools::Itertools;

use super::{
    props::TermInference, Goal, SharedSolution, Solution, SolveError, Task, TaskBuilder, TermIdx,
    TermProps, TracerHub,
};
use crate::{
    rule::{Hypothesis, HypothesisIterator, RuleAttr, RuleId, RulesEngine, SharedRule},
    task::{solution::SolutionStatus, Tracer},
    term::{SharedTerm, SubtermMut, Term},
    NormalizationLevel,
};

/// Maximum depth allowed for nested subtasks.
///
/// When the subtask level exceeds this value the solver aborts with
/// `SolveError::MaxSubtaskLevelExceed`.
pub const MAX_SUBTASK_LEVEL: usize = 10;

/// Default execution deadline (in cycle counts) for a solving run.
///
/// The solver stops and returns `SolveError::ExecutionDeadline` when the number
/// of performed cycles exceeds this value.
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

struct SolutionState {
    execution_deadline: usize,
    cycle_counter:      usize,
    cache:              HashMap<Term, SharedSolution>,
    tracer:             TracerHub,
}

pub struct Solver {
    rules_engine: Arc<RulesEngine>,

    local_rules: Vec<SharedRule>,

    unknown_terms: Vec<Term>,
}

impl Solver {
    /// Creates a new `Solver` instance.
    ///
    /// # Arguments
    ///
    /// * `rules` – An `Arc` pointing to a `RulesEngine` that provides the
    ///   global rule set used during solving.
    pub fn new(rules: Arc<RulesEngine>) -> Solver {
        Solver {
            rules_engine:  rules.clone(),
            local_rules:   vec![],
            unknown_terms: vec![],
        }
    }

    /// Attempts to solve the given `task` and returns a `SharedSolution`.
    ///
    /// # Parameters
    ///
    /// * `task` – The task to be solved.
    /// * `tracer` – A `TracerHub` used for instrumentation and cancellation.
    /// * `execution_deadline` – Maximum number of cycles the solver may execute
    ///   before aborting with `SolveError::ExecutionDeadline`.
    ///
    /// The method initializes a fresh `Solution` and `SolutionState`, and then
    /// runs the main solving loop. The resulting `SharedSolution` contains
    /// either the answer or an error status.
    pub fn solve(
        &mut self,
        task: Task,
        tracer: TracerHub,
        execution_deadline: usize,
    ) -> SharedSolution {
        let mut solution = Solution::new(task);
        let mut state = SolutionState {
            execution_deadline,
            cycle_counter: Default::default(),
            cache: Default::default(),
            tracer,
        };

        if solution.goal.is_find() {
            self.unknown_terms
                .push(solution.goal.term().term.as_ref().clone());
        }

        self.solve_impl(&mut solution, &mut state);
        solution.into()
    }

    fn solve_impl(&mut self, solution: &mut Solution, state: &mut SolutionState) {
        if solution.task.subtask_level > MAX_SUBTASK_LEVEL {
            solution.status = SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed);
            return;
        }
        state
            .tracer
            .on_subtask_start(&solution.task, state.current_cycle());

        solution.start_cycle = state.cycle_counter;

        // TODO: can be replaced with try { .. } in future
        // track: https://github.com/rust-lang/rust/issues/31436
        let mut main_loop = || loop {
            state.increment_cycle_counter()?;
            let index = self.try_focus_term(solution, state)?;
            if state.tracer.is_cancelled() {
                return Err(SolveError::Canceled);
            }
            if self.try_simplify(index, solution, state)? {
                continue;
            }
            if self.check_if_answer(solution, state, index)? {
                let level = solution.task.subtask_level;
                let answer = solution.answer().unwrap();
                trace!("Solved {level}. Answer: {answer}",);
                break Ok(());
            }
            self.add_local_rule(&mut solution[index]);
            self.try_infer_new_terms(index, solution, state)?;
        };
        if let Err(e) = main_loop() {
            solution.status = SolutionStatus::Err(e);
        }
        solution.end_cycle = state.cycle_counter;
        state.tracer.on_subtask_end(solution);
    }

    fn try_focus_term(
        &self,
        solution: &Solution,
        state: &mut SolutionState,
    ) -> Result<TermIdx, SolveError> {
        let index = solution.pick_next().ok_or(SolveError::NoConditions)?;
        state
            .tracer
            .on_term_focus(&solution[index], state.current_cycle());
        let level = solution[index].filters.level;

        trace!(target: "subtask",
            "[{}]({}) Level: {level} -> {}",
            solution.task.subtask_level,
            state.current_cycle(),
            solution[index]
        );

        let local_levels = self.local_rules.iter().map(|x| x.level);
        let max_local = local_levels.max().unwrap_or(0.into());
        if level > std::cmp::max(max_local, self.rules_engine.max_level()).next() {
            println!("level: {level}");
            return Err(SolveError::NoSolutionsFound);
        }
        Ok(index)
    }

    fn try_simplify(
        &mut self,
        index: usize,
        solution: &mut Solution,
        state: &mut SolutionState,
    ) -> Result<bool, SolveError> {
        if solution.goal.is_transform() {
            Ok(false)
        } else if let Some(simplified) = self.transform(solution, state, index) {
            solution[index].filters.mark_replaced();
            self.add_term(simplified, solution, state)?;
            Ok(true)
        } else {
            solution[index].filters.mark_simplified();
            Ok(false)
        }
    }

    fn add_term(
        &mut self,
        term: TermProps,
        s: &mut Solution,
        state: &mut SolutionState,
    ) -> Result<TermIdx, SolveError> {
        state.tracer.on_new_term(
            &term,
            &term
                .inference
                .parent_id()
                .map(|parent| s[parent].clone())
                .unwrap_or_else(|| TermProps::from(Term::zero())),
        );

        let is_goal = term.filters.is_goal();
        let index = s.add_term(term)?;
        if !is_goal {
            self.add_local_rule(&mut s[index]);
        }
        Ok(index)
    }

    fn add_local_rule(&mut self, term: &mut TermProps) {
        if term.filters.is_goal() {
            return;
        }
        let level = term.filters.level;
        if let Some(r) = term.rule(
            RuleId::new(0x80_00_00_00_00_00_00_00, self.local_rules.len() as u64 + 1),
            level.next(),
        ) {
            // TODO: check dups
            self.local_rules.push(r);
        }
    }

    // TODO: единый обьект для goal
    fn suggest_rules(
        &self,
        term: &TermProps,
        goal_term: &TermProps,
        goal: &Goal,
    ) -> Vec<SharedRule> {
        if goal.is_transform() && !term.filters.is_goal() {
            return vec![];
        }

        let local_rules = self.local_rules.iter().unique();
        let local_rules = local_rules
            .filter(|rule| rule.try_filter(&term.filters, &goal_term.term).is_ok())
            .cloned();
        let rules = self
            .rules_engine
            .suggest_rules(&term.filters, &goal_term.term);
        let rules = rules.into_iter().chain(local_rules);

        let rules: Vec<_> = if goal.is_prove() && term.filters.is_goal() {
            rules
                .filter(|rule| rule.contains_attribute(&RuleAttr::Equivalence))
                .collect()
        } else {
            rules.collect()
        };

        trace!(target: "rule_selection",
            "goal: {goal_term}, term: {term}, suggested_rules: {}",
            rules.iter().format(", ")
        );
        rules
    }

    fn try_infer_new_terms(
        &mut self,
        index: TermIdx,
        solution: &mut Solution,
        state: &mut SolutionState,
    ) -> Result<(), SolveError> {
        let is_goal = solution[index].filters.is_goal();
        let prove_goal = TermProps::from(
            Term::symbol("prove").with_child(solution[index].term.as_ref().clone()),
        );

        let mut added = false;
        for rule in self.suggest_rules(
            &solution[index],
            if is_goal && solution.goal.is_prove() {
                &prove_goal
            } else {
                &solution.task.goal
            },
            &solution.goal,
        ) {
            match self.produce(&rule, solution, state, index) {
                Some(s) => {
                    trace!("{} => {s}", solution[index]);
                    self.add_term(s, solution, state)?;
                    if is_goal && solution.goal.is_transform() {
                        // TODO: унифицировать weight = MAX_LEVEL и REPLACED
                        solution[index].filters.level = self.rules_engine.max_level().next();
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
            solution[index].filters.level.increment();
        }
        Ok(())
    }

    fn produce(
        &self,
        rule: &SharedRule,
        s: &mut Solution,
        state: &mut SolutionState,
        index: TermIdx,
    ) -> Option<TermProps> {
        let is_goal = s[index].filters.is_goal();
        let prove_goal = Term::symbol("prove").with_child(s[index].term.as_ref().clone());
        HypothesisIterator::new(
            rule.clone(),
            &s[index].term,
            &s[index].filters,
            if is_goal && s.goal.is_prove() {
                &prove_goal
            } else {
                &s.task.goal.term
            },
        )
        .filter_map(|hypothesis| {
            let is_dub = if is_goal {
                s.goal_index.contains_key(&hypothesis.resolution)
            } else {
                s.main_index.contains_key(&hypothesis.resolution)
            };
            if is_dub {
                return None;
            }
            let props = self.try_prove_hypothesis(index, s, hypothesis, state);
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
        state: &mut SolutionState,
    ) -> TermProps {
        trace!(
            target: "rule_selection",
            "new hypothesis {hypothesis}, rule {}, term: {}",
            hypothesis.rule, solution[parent_idx]
        );
        state.tracer.on_new_hypothesis(
            solution[parent_idx].term.clone(),
            &hypothesis,
            state.current_cycle(),
        );

        let mut props = TermProps::from(hypothesis.resolution.clone());
        props.filters.blocked_rules = hypothesis.blocked_rules.iter().cloned().collect();
        if solution[parent_idx].filters.is_goal() {
            props.filters.mark_goal();
        }
        let mut req_proofs = vec![];
        let mut iter = hypothesis.requirements.clone().into_iter();
        for i in iter.by_ref() {
            req_proofs.push(self.prove(solution, i, state));
            let last = req_proofs.last().unwrap();
            if last.answer().is_none() {
                trace!(
                    target: "rule_selection",
                    "term {} rejected, requirement not proven {}",
                    hypothesis.resolution,
                    last.task.goal
                );
                break;
            }
        }
        for req in iter {
            req_proofs.push(SharedSolution::new(Solution::new(Task::from(
                TermProps::from(Term::symbol("prove").with_child(req)),
            ))));
        }
        props.inference = TermInference::Rule {
            rule:         hypothesis.rule.clone(),
            params:       hypothesis.params.clone(),
            parent:       parent_idx,
            requirements: req_proofs,
        };

        state
            .tracer
            .on_hypothesis_finish(&props.inference, state.current_cycle());

        if props.inference.is_proven() {
            trace!(
                target: "rule_selection",
                "hypothesis {hypothesis} proven, resolution {} applied",
                hypothesis.resolution
            );
        }
        props
    }

    fn prove(
        &self,
        solution: &Solution,
        mut term: Term,
        state: &mut SolutionState,
    ) -> SharedSolution {
        is_replace(&mut term.as_subterm_mut());
        let term = term.normalize(NormalizationLevel::max());
        let prove_goal = SharedTerm::new(Term::symbol("prove").with_child(term.clone()));

        if term.as_subterm().truth().is_true() {
            // Build a minimal solution whose answer is the proven term
            let mut trivial_solution = Solution::new(Task::from(TermProps::from(prove_goal)));
            // Add the term to the solution and mark it as the answer
            let idx = trivial_solution
                .add_term(TermProps::from(term.clone()))
                .expect("failed to add trivial term");
            trivial_solution.status = SolutionStatus::Answer(idx);
            return SharedSolution::new(trivial_solution);
        }

        // Check if argument of "term is known" doesn't contains any of unknown_terms.
        if !self.unknown_terms.is_empty() {
            let term_root = term.as_subterm();
            if term_root.data().is_symbol_name("is") &&
                term_root.degree() == 2 &&
                term_root
                    .last_arg()
                    .expect("missing arg")
                    .data()
                    .is_symbol_name("known")
            {
                let first = term_root.first_arg().expect("missing arg");

                if self
                    .unknown_terms
                    .iter()
                    .any(|i| first.contains(&i.as_subterm()))
                {
                    let mut no_solution = Solution::new(Task::from(TermProps::from(prove_goal)));
                    no_solution.status = SolutionStatus::Err(SolveError::NoSolutionsFound);
                    return SharedSolution::new(no_solution);
                }
            }
        }

        self.solve_subtask(solution, prove_goal, state)
    }

    fn transform(
        &mut self,
        solution: &mut Solution,
        state: &mut SolutionState,
        index: usize,
    ) -> Option<TermProps> {
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
        let subtask_solution = self.solve_subtask(solution, task.clone(), state);

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
        if solution[index].filters.is_goal() {
            result.filters.mark_goal();
        }

        Some(result)
    }

    fn solve_subtask(
        &self,
        solution: &Solution,
        task: SharedTerm,
        state: &mut SolutionState,
    ) -> SharedSolution {
        if let Some(x) = state.cache.get(&task) {
            return x.clone();
        }

        let goal = task.as_subterm().to_term();
        if state
            .cache
            .insert(
                goal.clone(),
                Solution::new(Task::from(TermProps::from(goal))).into(),
            )
            .is_some()
        {
            // TODO: recursion
            unimplemented!("subtask recursion");
        }

        let mut subtask_solver = Solver::new(self.rules_engine.clone());
        subtask_solver.unknown_terms = self.unknown_terms.clone();

        let subtask = Self::subtask(solution, task.clone());
        let mut subtask_solution = Solution::new(subtask);
        subtask_solver.solve_impl(&mut subtask_solution, state);
        let subtask_solution = SharedSolution::new(subtask_solution);
        *state.cache.get_mut(&task).unwrap() = subtask_solution.clone();
        if let SolutionStatus::Err(e) = subtask_solution.status {
            trace!("Can't prove {task}: {e}");
            if e == SolveError::MaxSubtaskLevelExceed {
                state.cache.remove(&task);
            }
        }
        subtask_solution
    }

    fn subtask(solution: &Solution, task: SharedTerm) -> Task {
        TaskBuilder::default()
            .with_goal(TermProps::from(task.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                solution
                    .terms
                    .iter()
                    .filter(|x| x.inference.is_proven())
                    .filter(|x| {
                        !(x.filters.is_goal() ||
                            x.term.as_subterm().data().is_symbol_name("answer"))
                    })
                    .cloned(),
            )
            .with_level(solution.task.subtask_level + 1)
            .build()
            .expect("Can't build subtask")
    }

    fn check_if_answer(
        &self,
        solution: &mut Solution,
        state: &mut SolutionState,
        index: usize,
    ) -> Result<bool, SolveError> {
        if solution[index].filters.is_goal() {
            return Ok(false);
        }
        if self.check_answer_term(solution, index)? {
            return Ok(true);
        }

        Ok(match solution.goal.clone() {
            Goal::Find(x) => self.check_find_answer(solution, state, index, x.term.as_ref()),
            Goal::Prove(_) => self.check_prove_answer(solution, index),
            Goal::Transform(_) => self.check_transform_answer(solution),
        })
    }

    fn check_answer_term(
        &self,
        solution: &mut Solution,
        index: usize,
    ) -> Result<bool, SolveError> {
        let term = &solution[index];
        let term_root = term.term.as_subterm();

        if solution.goal.is_transform() {
            return Ok(false);
        }
        if term_root.data().is_symbol_name("answer") && term_root.degree() == 1 {
            let mut resolution =
                TermProps::from(term.term.as_subterm().first_arg().unwrap().to_term());
            resolution.inference = term.inference.clone();
            let idx = solution.add_term(resolution)?;
            solution.status = SolutionStatus::Answer(idx);
            return Ok(true);
        }
        Ok(false)
    }

    fn check_find_answer(
        &self,
        solution: &mut Solution,
        state: &mut SolutionState,
        index: usize,
        find: &Term,
    ) -> bool {
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
            if self.prove(solution, is_known, state).answer().is_some() {
                solution.status = SolutionStatus::Answer(index);
                return true;
            }
        }
        false
    }

    fn check_prove_answer(&self, solution: &mut Solution, index: usize) -> bool {
        let term = &solution[index];

        // TODO: теперь тут бывают целевые термы, поэтому надо сделать две проверки:
        // что терм есть среди целей
        // что цель тривиальная истина
        for i in solution.goal_index.values() {
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
        let Some(index) = solution.pick_goal_term() else {
            return false;
        };

        if solution[index].filters.level >= self.rules_engine.max_level() {
            let mut iter = solution
                .terms
                .iter()
                .rev()
                .filter(|x| x.inference.is_proven());
            let res = iter.find(|x| x.filters.is_goal()).map(|x| x.id).unwrap();
            // TODO: надо заполнить правильно
            // согласовать с выводом решения по шагам
            // res.inference = TermInference::Condition;
            solution.status = SolutionStatus::Answer(res);
            return true;
        }
        false
    }
}

impl SolutionState {
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

    use crate::{
        rule::RulesEngine,
        task::{parse_task, Solver},
        term::term_with_vars,
    };

    #[test]
    fn unknown_terms_find_goal() {
        let task = parse_task("task { goal find(x); x == 1; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);

        let _ = solver.solve(task, Default::default(), usize::MAX);

        assert_eq!(solver.unknown_terms.len(), 1);
        assert_eq!(solver.unknown_terms[0], term_with_vars("x"));
    }

    #[test]
    fn unknown_terms_prove_goal() {
        let task = parse_task("task { goal prove(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);

        let _ = solver.solve(task, Default::default(), usize::MAX);

        assert!(
            solver.unknown_terms.is_empty(),
            "unknown_terms should be empty for prove goal"
        );
    }

    #[test]
    fn unknown_terms_transform_goal() {
        let task = parse_task("task { goal transform(1 + 2); }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);

        let _ = solver.solve(task, Default::default(), usize::MAX);

        assert!(
            solver.unknown_terms.is_empty(),
            "unknown_terms should be empty for transform goal"
        );
    }

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task { goal find(x); x == 1; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);
        let solution = solver.solve(task, Default::default(), usize::MAX);
        assert_eq!(
            *solution.answer().expect("task is not solved"),
            term_with_vars("x == 1")
        );
    }

    #[test]
    fn check_answer_prove_test() {
        let task = parse_task("task { goal prove(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);
        let solution = solver.solve(task, Default::default(), usize::MAX);
        assert!(solution.answer().is_some());
    }
}
