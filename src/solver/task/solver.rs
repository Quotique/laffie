use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use indexmap::IndexMap;
use itertools::Itertools;

use super::{
    Goal, SharedSolution, Solution, SolveError, Task, TaskBuilder, TermIdx, TermProps, TracerHub,
    goal::FindGoal, props::TermInference,
};
use crate::{
    NormalizationLevel,
    rule::{
        GroundedHypothesis, Hypothesis, HypothesisIterator, RuleAttr, RuleId, RulesEngine,
        SharedRule,
    },
    task::{Tracer, solution::SolutionStatus},
    term::{
        Atom, Param, SharedTerm, Substitute, Term, TermBuf, TermMut, TermRef, Truth, match_term,
    },
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

/// Default wall-clock budget for a solving run (effectively unlimited).
pub const TIME_LIMIT_DEFAULT: Duration = Duration::from_secs(24 * 60 * 60);

/// How often (in grounded hypotheses) `produce` polls the wall-clock deadline.
const DEADLINE_CHECK_INTERVAL: usize = 64;

struct SolutionState {
    execution_deadline: usize,
    /// Wall-clock deadline for the whole run.
    deadline_at:        Instant,
    cycle_counter:      usize,
    cache:              HashMap<TermBuf, SharedSolution>,
    tracer:             TracerHub,
}

pub struct Solver {
    rules_engine: Arc<RulesEngine>,

    local_rules: IndexMap<RuleId, SharedRule>,
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
            rules_engine: rules.clone(),
            local_rules:  IndexMap::new(),
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
    /// * `time_limit` – Wall-clock budget for the run; when exceeded the solver
    ///   aborts with `SolveError::TimeDeadline`. Pass a large value to disable.
    ///
    /// The method initializes a fresh `Solution` and `SolutionState`, and then
    /// runs the main solving loop. The resulting `SharedSolution` contains
    /// either the answer or an error status.
    pub fn solve(
        &mut self,
        task: Task,
        tracer: TracerHub,
        execution_deadline: usize,
        time_limit: Duration,
    ) -> SharedSolution {
        let mut solution = Solution::new(task);
        let mut state = SolutionState {
            execution_deadline,
            deadline_at: Instant::now() + time_limit,
            cycle_counter: Default::default(),
            cache: Default::default(),
            tracer,
        };

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
        solution: &mut Solution,
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

        let local_levels = self.local_rules.values().map(|x| x.level);
        let max_local = local_levels.max().unwrap_or(0.into());
        if level > std::cmp::max(max_local, self.rules_engine.max_level()).next() {
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
                .unwrap_or_else(|| TermProps::from(TermBuf::zero())),
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
            self.local_rules.entry(r.id).or_insert(r);
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

        let local_rules = self
            .local_rules
            .values()
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
        let prove_goal =
            TermProps::from(TermBuf::symbol("prove").arg(solution[index].term.as_ref().clone()));

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
            match self.produce(&rule, solution, state, index)? {
                Some(s) => {
                    trace!("{} => {s}", solution[index]);
                    self.add_term(s, solution, state)?;
                    if is_goal && solution.goal.is_transform() {
                        // TODO: унифицировать weight = MAX_LEVEL и REPLACED
                        solution[index].filters.level = self.rules_engine.max_level().next();
                        solution.requeue(index);
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
            solution.requeue(index);
        }
        Ok(())
    }

    fn produce(
        &self,
        rule: &SharedRule,
        s: &mut Solution,
        state: &mut SolutionState,
        index: TermIdx,
    ) -> Result<Option<TermProps>, SolveError> {
        let is_goal = s[index].filters.is_goal();
        let prove_goal = TermBuf::symbol("prove").arg(s[index].term.as_ref().clone());
        // Collected up front to release the `&s` borrow before the loop mutates
        // `s`. The rest is lazy: `resolve_solve_in_hypothesis` and `ground()`
        // run only for hypotheses actually reached, short-circuiting on proof.
        let raw_hypotheses: Vec<Hypothesis> = HypothesisIterator::new(
            rule.clone(),
            &s[index].term,
            &s[index].filters,
            if is_goal && s.goal.is_prove() {
                &prove_goal
            } else {
                &s.task.goal.term
            },
        )
        .collect();
        let mut grounded_seen = 0usize;
        for mut h in raw_hypotheses {
            resolve_parents_in_hypothesis(&mut h, s[index].term.as_ref());
            let Some(h) = self.resolve_solve_in_hypothesis(h, s, state) else {
                continue;
            };
            for hypothesis in h.ground() {
                grounded_seen += 1;
                if grounded_seen.is_multiple_of(DEADLINE_CHECK_INTERVAL) &&
                    Instant::now() >= state.deadline_at
                {
                    return Err(SolveError::TimeDeadline);
                }
                let is_dub = if is_goal {
                    s.goal_index.contains_key(&hypothesis.resolution)
                } else {
                    s.main_index.contains_key(&hypothesis.resolution)
                };
                if is_dub {
                    continue;
                }
                let props = self.try_prove_hypothesis(index, s, hypothesis, state);
                if props.inference.is_proven() {
                    return Ok(Some(props));
                }
                // TODO: option to disable
                s.add_term(props)?;
            }
        }
        Ok(None)
    }

    fn try_prove_hypothesis(
        &self,
        parent_idx: usize,
        solution: &Solution,
        hypothesis: GroundedHypothesis,
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
                TermProps::from(TermBuf::symbol("prove").arg(req)),
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
        mut term: TermBuf,
        state: &mut SolutionState,
    ) -> SharedSolution {
        is_replace(&mut term.term_mut());
        let term = term.normalize(NormalizationLevel::max());
        let prove_goal = SharedTerm::new(TermBuf::symbol("prove").arg(term.clone()));

        match term.term().truth() {
            // The proven term is itself the answer.
            Truth::True => {
                let mut trivial_solution = Solution::new(Task::from(TermProps::from(prove_goal)));
                trivial_solution.status = match trivial_solution.add_term(TermProps::from(term)) {
                    Ok(idx) => SolutionStatus::Answer(idx),
                    Err(e) => SolutionStatus::Err(e),
                };
                SharedSolution::new(trivial_solution)
            }
            // Unprovable — fail fast instead of searching.
            Truth::False => {
                let mut no_solution = Solution::new(Task::from(TermProps::from(prove_goal)));
                no_solution.status = SolutionStatus::Err(SolveError::NoSolutionsFound);
                SharedSolution::new(no_solution)
            }
            Truth::Unknown => self.solve_subtask(solution, prove_goal, HashSet::new(), state),
        }
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

        let use_answer = term.term.term().data().is_symbol_name("answer");
        let to_transform = if use_answer {
            term.term.term().first_arg().unwrap().to_owned()
        } else {
            term.term.term().to_owned()
        };
        let task = SharedTerm::new(TermBuf::symbol("transform").arg(to_transform));
        let blocked = solution[index].filters.blocked_rules.clone();
        let subtask_solution = self.solve_subtask(solution, task.clone(), blocked, state);

        let mut answer = subtask_solution.answer()?.as_ref().clone();
        if use_answer {
            answer = TermBuf::symbol("answer").arg(answer);
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
        blocked_rules: HashSet<RuleId>,
        state: &mut SolutionState,
    ) -> SharedSolution {
        if let Some(x) = state.cache.get(&task) {
            return x.clone();
        }

        let goal = task.term().to_owned();
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

        let subtask = match Self::subtask(solution, task.clone(), blocked_rules) {
            Ok(subtask) => subtask,
            Err(e) => {
                let sol = Self::errored_subtask(task.term().to_owned(), e);
                *state.cache.get_mut(&task).unwrap() = sol.clone();
                return sol;
            }
        };

        let mut subtask_solver = Solver::new(self.rules_engine.clone());
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

    /// Runs `solve(find(vars...), eqs...)` as a fresh subtask, cached
    /// by `cache_key`. Recursion on the same key hits the placeholder
    /// and drops; a smaller form gets a fresh key and proceeds.
    fn run_solve_block(
        &self,
        cache_key: TermBuf,
        goal: TermBuf,
        eqs: Vec<TermBuf>,
        parent: &Solution,
        state: &mut SolutionState,
    ) -> SharedSolution {
        if let Some(x) = state.cache.get(&cache_key) {
            return x.clone();
        }

        // Placeholder needs a valid Goal — use the inner `find` term.
        if state
            .cache
            .insert(
                cache_key.clone(),
                Solution::new(Task::from(TermProps::from(goal.clone()))).into(),
            )
            .is_some()
        {
            unimplemented!("solve-block recursion");
        }

        let mut goal_props = TermProps::from(goal);
        goal_props.filters.mark_goal();

        let task = match TaskBuilder::default()
            .with_goal(goal_props)
            .map(|builder| {
                let mut builder = builder
                    .with_level(parent.task.subtask_level + 1)
                    .with_conditions(
                        parent
                            .terms
                            .iter()
                            .filter(|x| x.is_proven())
                            .filter(|x| {
                                !(x.filters.is_goal() ||
                                    x.term.term().data().is_symbol_name("answer"))
                            })
                            .cloned(),
                    );
                for eq in eqs {
                    builder = builder.with_condition(TermProps::from(eq));
                }
                builder
            })
            .and_then(TaskBuilder::build)
        {
            Ok(task) => task,
            Err(e) => {
                error!("can't build solve subtask: {e}");
                let sol = Self::errored_subtask(cache_key.clone(), SolveError::Internal);
                *state.cache.get_mut(&cache_key).unwrap() = sol.clone();
                return sol;
            }
        };

        let mut subtask_solver = Solver::new(self.rules_engine.clone());

        let mut subtask_solution = Solution::new(task);
        subtask_solver.solve_impl(&mut subtask_solution, state);
        let subtask_solution = SharedSolution::new(subtask_solution);
        *state.cache.get_mut(&cache_key).unwrap() = subtask_solution.clone();
        if let SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed) = subtask_solution.status {
            state.cache.remove(&cache_key);
        }
        subtask_solution
    }

    /// Replaces `solve(...) == Param` requirements with subtask answer.
    fn resolve_solve_in_hypothesis(
        &self,
        mut hyp: Hypothesis,
        parent: &Solution,
        state: &mut SolutionState,
    ) -> Option<Hypothesis> {
        let mut new_reqs = Vec::with_capacity(hyp.requirements.len());
        let mut bindings: Vec<(Param, TermBuf)> = Vec::new();
        for req in hyp.requirements.drain(..) {
            let Some((solve_call, param_atom)) = match_solve_eq_param(&req) else {
                new_reqs.push(req);
                continue;
            };

            let cache_key = solve_call.to_owned();
            let goal = solve_call.first_arg()?.to_owned();
            let eqs: Vec<TermBuf> = solve_call
                .args_iter()
                .skip(1)
                .map(|c| c.to_owned())
                .collect();

            let result = self.run_solve_block(cache_key, goal, eqs, parent, state);
            let answer_term = result.answer()?;
            let mut answer_buf: TermBuf = (*answer_term).clone();
            if answer_buf.term().data().is_symbol_name("answer") && answer_buf.term().degree() == 1
            {
                answer_buf = answer_buf.term_mut().pop_first_arg().unwrap();
            }

            // `bind_equality_params` would still refuse a value that
            // contains params, so substitute directly into the hypothesis.
            bindings.push((param_atom, answer_buf));
        }
        hyp.requirements = new_reqs;
        if !bindings.is_empty() {
            let subst: crate::term::ParamSubstitution = bindings.into_iter().collect();
            hyp.substitute(&subst);
        }
        Some(hyp)
    }

    fn subtask(
        solution: &Solution,
        task: SharedTerm,
        blocked_rules: HashSet<RuleId>,
    ) -> Result<Task, SolveError> {
        let mut goal = TermProps::from(task.clone());
        goal.filters.blocked_rules = blocked_rules;
        TaskBuilder::default()
            .with_goal(goal)
            .and_then(|builder| {
                builder
                    .with_conditions(
                        solution
                            .terms
                            .iter()
                            .filter(|x| x.is_proven())
                            .filter(|x| {
                                !(x.filters.is_goal() ||
                                    x.term.term().data().is_symbol_name("answer"))
                            })
                            .cloned(),
                    )
                    .with_level(solution.task.subtask_level + 1)
                    .build()
            })
            .map_err(|e| {
                error!("can't build subtask: {e}");
                SolveError::Internal
            })
    }

    /// A subtask solution carrying only an error status, for when the subtask
    /// could not even be built.
    fn errored_subtask(goal: TermBuf, e: SolveError) -> SharedSolution {
        let mut solution = Solution::new(Task::from(TermProps::from(goal)));
        solution.status = SolutionStatus::Err(e);
        SharedSolution::new(solution)
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

        Ok(match &solution.goal.clone() {
            Goal::Find(g) => self.check_find_answer(solution, state, index, g),
            Goal::Prove(_) => self.check_prove_answer(solution, index),
            Goal::Transform(_) => self.check_transform_answer(solution),
        })
    }

    fn check_answer_term(&self, solution: &mut Solution, index: usize) -> Result<bool, SolveError> {
        let term = &solution[index];
        let term_root = term.term.term();

        if solution.goal.is_transform() {
            return Ok(false);
        }
        if term_root.data().is_symbol_name("answer") && term_root.degree() == 1 {
            let first_arg = term
                .term
                .term()
                .first_arg()
                .ok_or(SolveError::NoSolutionsFound)?
                .to_owned();
            let mut resolution = TermProps::from(first_arg);
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
        find_goal: &FindGoal,
    ) -> bool {
        // Single target: the term is the answer iff it is an answer form
        // (subsumes flat `x == k` / `x in S` and piecewise, recognized atomically).
        if find_goal.targets.len() == 1 {
            let target = find_goal.targets[0].term();
            if self.is_answer_form(solution[index].term.term(), target, solution, state) {
                solution.status = SolutionStatus::Answer(index);
                return true;
            }
            return false;
        }

        // Multi target: accumulate one flat binding per target across terms.
        let term = solution[index].term.term();
        let (lhs, rhs) = match_term!(term, "=="(lhs, rhs))
            .or_else(|| match_term!(term, "in"(lhs, rhs)))
            .unzip();
        let (Some(lhs), Some(_)) = (lhs, rhs) else {
            return false;
        };

        let target = find_goal
            .targets
            .iter()
            .find(|t| lhs == t.term() && !solution.find_bindings.contains_key(*t));
        let Some(target) = target else {
            return false;
        };
        let target = target.clone();

        if !self.is_answer_leaf(solution[index].term.term(), target.term(), solution, state) {
            return false;
        }

        solution.find_bindings.insert(target, index);

        if solution.find_bindings.len() == find_goal.targets.len() {
            let answer = self.build_multi_find_answer(solution, find_goal);
            if let Ok(idx) = solution.add_term(answer) {
                solution.status = SolutionStatus::Answer(idx);
            }
            return true;
        }
        false
    }

    /// One-level answer form for a single `target`: an answer leaf, a `&&`
    /// branch (one leaf + `is known` guards), or a `||` of such branches.
    fn is_answer_form(
        &self,
        term: TermRef,
        target: TermRef,
        solution: &Solution,
        state: &mut SolutionState,
    ) -> bool {
        if term.data().is_symbol_name("||") {
            return term.degree() > 0 &&
                term.args_iter()
                    .all(|b| self.is_answer_branch(b, target, solution, state));
        }
        self.is_answer_branch(term, target, solution, state)
    }

    /// An answer leaf, or `&&(guards..., leaf)` with exactly one
    /// target-resolving leaf and every other conjunct an `is known` guard.
    fn is_answer_branch(
        &self,
        branch: TermRef,
        target: TermRef,
        solution: &Solution,
        state: &mut SolutionState,
    ) -> bool {
        if self.is_answer_leaf(branch, target, solution, state) {
            return true;
        }
        if !branch.data().is_symbol_name("&&") {
            return false;
        }
        let mut leaf_seen = false;
        for conjunct in branch.args_iter() {
            if self.is_answer_leaf(conjunct, target, solution, state) {
                if leaf_seen {
                    return false;
                }
                leaf_seen = true;
            } else if !self.is_provably_known(conjunct, solution, state) {
                return false;
            }
        }
        leaf_seen
    }

    /// `target == <known>` or `target in <known>`.
    fn is_answer_leaf(
        &self,
        term: TermRef,
        target: TermRef,
        solution: &Solution,
        state: &mut SolutionState,
    ) -> bool {
        let Some((lhs, rhs)) =
            match_term!(term, "=="(lhs, rhs)).or_else(|| match_term!(term, "in"(lhs, rhs)))
        else {
            return false;
        };
        lhs == target && self.is_provably_known(rhs, solution, state)
    }

    /// `true` if `term is known` is provable.
    fn is_provably_known(
        &self,
        term: TermRef,
        solution: &Solution,
        state: &mut SolutionState,
    ) -> bool {
        let query = TermBuf::symbol("is")
            .arg(term.to_owned())
            .arg(TermBuf::symbol("known"));
        self.prove(solution, query, state).answer().is_some()
    }

    fn build_multi_find_answer(&self, solution: &Solution, find_goal: &FindGoal) -> TermProps {
        let mut iter = find_goal.targets.iter();
        let first = iter.next().unwrap();
        let mut result = solution[solution.find_bindings[first]]
            .term
            .term()
            .to_owned();

        for target in iter {
            let binding_term = solution[solution.find_bindings[target]]
                .term
                .term()
                .to_owned();
            result = TermBuf::symbol("&&").arg(result).arg(binding_term);
        }
        TermProps::from(result)
    }

    fn check_prove_answer(&self, solution: &mut Solution, index: usize) -> bool {
        let term = &solution[index];

        // A candidate with unresolved requirements is not a proof — accepting it
        // would close circular hypotheses (e.g. `[x^2=a] => x^2=a`) where a
        // rule's own resolution proves its own requirement.
        if !term.is_proven() {
            return false;
        }

        // TODO: теперь тут бывают целевые термы, поэтому надо сделать две проверки:
        // что терм есть среди целей
        // что цель тривиальная истина
        for i in solution.goal_index.values() {
            if term.term == solution[*i].term {
                solution.status = SolutionStatus::Answer(index);
                return true;
            }
            if solution[*i].term.term().truth().is_true() {
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
            let mut iter = solution.terms.iter().rev().filter(|x| x.is_proven());
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
        if Instant::now() >= self.deadline_at {
            return Err(SolveError::TimeDeadline);
        }
        Ok(())
    }
}

/// Resolves the `parents` requirement primitive against the match position:
/// rewrites each `parents` marker into the `set(...)` of the matched term's
/// ancestor head symbols. Like `solve`, this is computed here rather than by
/// term normalization.
fn resolve_parents_in_hypothesis(hyp: &mut Hypothesis, term: &TermBuf) {
    let marker = TermBuf::symbol("parents");
    if !hyp
        .requirements
        .iter()
        .any(|r| r.term().contains(&marker.term()))
    {
        return;
    }
    // `set(...)` of the head symbols of the match position's strict ancestors.
    let mut set = TermBuf::symbol("set");
    let mut node = term.term();
    for &i in &*hyp.pos {
        if let Some(symbol) = node.data().symbol() {
            set = set.arg(TermBuf::symbol(symbol.as_str()));
        }
        let Some(child) = node.args_iter().nth(i) else {
            break;
        };
        node = child;
    }
    for r in &mut hyp.requirements {
        r.replace(&marker, &set);
    }
}

/// Matches `solve(...) == Param` (either argument order).
fn match_solve_eq_param<'a>(req: &'a TermBuf) -> Option<(TermRef<'a>, Param)> {
    let (lhs, rhs) = match_term!(req.term(), "=="(lhs, rhs))?;
    if lhs.data().is_symbol_name("solve") &&
        let Atom::Param(p) = rhs.data()
    {
        return Some((lhs, p.clone()));
    }
    if rhs.data().is_symbol_name("solve") &&
        let Atom::Param(p) = lhs.data()
    {
        return Some((rhs, p.clone()));
    }
    None
}

fn is_replace(root: &mut TermMut) {
    if !root.data().is_symbol_name("is") || root.degree() != 2 {
        return;
    }

    match root.last_arg().unwrap().data().symbol() {
        Some(name) if name == "true" => {
            let mut child = root.pop_first_arg().unwrap();
            root.swap(&mut child.term_mut());
        }
        Some(name) if name == "false" => {
            let child = root.pop_first_arg().unwrap();
            root.swap(&mut TermBuf::symbol("!").arg(child).term_mut());
        }
        _ => {}
    }
}

#[cfg(test)]
mod solution_tests {
    use std::sync::Arc;

    use crate::{
        rule::RulesEngine,
        task::{Solver, TIME_LIMIT_DEFAULT, parse_task},
        term::term_with_vars,
    };

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task { goal find(x); x == 1; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);
        let solution = solver.solve(task, Default::default(), usize::MAX, TIME_LIMIT_DEFAULT);
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
        let solution = solver.solve(task, Default::default(), usize::MAX, TIME_LIMIT_DEFAULT);
        assert!(solution.answer().is_some());
    }

    #[test]
    fn check_answer_multi_var_find() {
        let task = parse_task("task { goal find(x, y); x == 3; y == 4; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules);
        let solution = solver.solve(task, Default::default(), usize::MAX, TIME_LIMIT_DEFAULT);
        let answer = solution
            .answer()
            .expect("multi-var find task is not solved");
        assert_eq!(*answer, term_with_vars("x == 3 && y == 4"));
    }
}

#[cfg(test)]
mod parents_tests {
    use std::sync::Arc;

    use crate::{
        rule::{Hypothesis, parse_rule},
        term::{ParamSubstitution, TermBuf, TermPath, term_with_vars},
    };

    use super::resolve_parents_in_hypothesis;

    fn hypothesis(requirements: Vec<TermBuf>, pos: Vec<usize>) -> Hypothesis {
        let rule = Arc::new(parse_rule(
            r#"rule { attr level(1); a + x == 0 => x == -a; a!=0; }"#,
        ));
        Hypothesis {
            rule,
            resolution: term_with_vars("answer(x == 1)"),
            free_params: Default::default(),
            params: ParamSubstitution::default(),
            requirements,
            blocked_rules: vec![],
            pos: TermPath::from(pos),
        }
    }

    #[test]
    fn rewrites_marker_into_ancestor_set() {
        // `a` sits under `==` inside `||`: a == b || c.
        let mut hyp = hypothesis(vec![term_with_vars("answer in parents")], vec![0, 0]);
        resolve_parents_in_hypothesis(&mut hyp, &term_with_vars("a == b || c"));

        let set = TermBuf::symbol("set")
            .arg(TermBuf::symbol("||"))
            .arg(TermBuf::symbol("=="));
        let mut expected = term_with_vars("answer in parents");
        expected.replace(&TermBuf::symbol("parents"), &set);
        assert_eq!(hyp.requirements[0], expected);
    }

    #[test]
    fn root_match_rewrites_into_empty_set() {
        let mut hyp = hypothesis(vec![term_with_vars("answer in parents")], vec![]);
        resolve_parents_in_hypothesis(&mut hyp, &term_with_vars("a == b"));
        assert_eq!(hyp.requirements[0], term_with_vars("answer in set"));
    }

    #[test]
    fn noop_without_marker() {
        let mut hyp = hypothesis(vec![term_with_vars("a != 0")], vec![0]);
        let before = hyp.requirements.clone();
        resolve_parents_in_hypothesis(&mut hyp, &term_with_vars("a == b"));
        assert_eq!(hyp.requirements, before);
    }
}

#[cfg(test)]
mod resolve_solve_tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use crate::{
        rule::{Hypothesis, RulesEngine, SharedRule, parse_rule},
        task::{Solution, Solver, TracerHub, parse_task},
        term::{ParamSubstitution, TermBuf, TermPath, term_with_vars},
    };

    use super::SolutionState;

    /// Two `solve(...) == Param` requirements must both reach the resolution.
    /// The old single-binding code kept only the last and dropped the first.
    #[test]
    fn two_solve_bindings_both_apply() {
        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let parent = Solution::new(parse_task("task { goal find(z); }"));
        let mut state = SolutionState {
            execution_deadline: usize::MAX,
            deadline_at:        Instant::now() + Duration::from_secs(60),
            cycle_counter:      0,
            cache:              Default::default(),
            tracer:             TracerHub::default(),
        };

        let solve_eq_param = |block: &'static str, param: &str| {
            TermBuf::symbol("==")
                .arg(term_with_vars(block))
                .arg(TermBuf::param(param))
        };
        let hyp = Hypothesis {
            rule:          Arc::new(parse_rule(
                "rule { attr level(1); a + x == 0 => x == -a; a!=0; }",
            )),
            resolution:    TermBuf::symbol("&&")
                .arg(TermBuf::param("p"))
                .arg(TermBuf::param("q")),
            free_params:   Default::default(),
            params:        ParamSubstitution::default(),
            requirements:  vec![
                solve_eq_param("solve(find(x), x == 1)", "p"),
                solve_eq_param("solve(find(y), y == 2)", "q"),
            ],
            blocked_rules: vec![],
            pos:           TermPath::from(vec![]),
        };

        let resolved = solver
            .resolve_solve_in_hypothesis(hyp, &parent, &mut state)
            .expect("hypothesis dropped");

        assert_eq!(resolved.resolution, term_with_vars("x == 1 && y == 2"));
    }

    /// A `produce` call grounding many hypotheses past the deadline must bail
    /// out with `TimeDeadline` instead of walking the whole product.
    #[test]
    fn produce_aborts_on_deadline() {
        use crate::task::{SolveError, TermProps};

        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let mut solution = Solution::new(parse_task("task { goal find(z); }"));
        // Focus term for the rule, plus a term equal to every grounding's
        // resolution so each hypothesis is a duplicate and the loop keeps going.
        let index = solution
            .add_term(TermProps::from(term_with_vars("y + 1")))
            .unwrap();
        solution
            .add_term(TermProps::from(term_with_vars("y == 1")))
            .unwrap();

        // Free `p` over a 70-element set → 70 groundings (all `y == 1`).
        let elems = (1..=70)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let rule_src: &'static str = Box::leak(
            format!("rule {{ attr level(0); a + b => a == b; p in set({elems}); }}")
                .into_boxed_str(),
        );
        let rule: SharedRule = Arc::new(parse_rule(rule_src));

        let mut state = SolutionState {
            execution_deadline: usize::MAX,
            deadline_at:        Instant::now() - Duration::from_secs(1),
            cycle_counter:      0,
            cache:              Default::default(),
            tracer:             TracerHub::default(),
        };

        let err = solver
            .produce(&rule, &mut solution, &mut state, index)
            .expect_err("produce should abort on a passed deadline");
        assert!(matches!(err, SolveError::TimeDeadline));
    }
}
