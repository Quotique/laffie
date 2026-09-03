use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use indexmap::IndexMap;
use itertools::Itertools;

use super::{
    SharedSolution, Solution, SolveError, TermIdx, TermProps, TracerHub, props::TermInference,
};
use crate::{
    NormLevel, Rational,
    engine::{Tracer, solution::SolutionStatus},
    rule::{
        GroundedHypothesis, Hypothesis, HypothesisIterator, Level, RuleAttr, RuleId, RulesEngine,
        SharedRule,
    },
    task::{Goal, GoalKind, Task, goal::Recognized},
    term::{
        Atom, Param, SharedTerm, Substitute, Term, TermBuf, TermMut, TermRef, Truth, TruthCtx,
        answer, match_term,
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

/// How often (in grounded hypotheses) `produce` polls the run limits.
const DEADLINE_CHECK_INTERVAL: usize = 64;

/// Id namespace for a frame's own rules, clear of the ids the rule engine
/// hands out.
const LOCAL_RULE_MASK: u64 = 0x80_00_00_00_00_00_00_00;

/// External cancellation handle. Cloning shares the same flag, so a caller can
/// hold a clone and cancel a run from another thread.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Signals cancellation; every clone observes it.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Stop conditions for a solving run: cycle budget, wall-clock deadline, and
/// external cancellation — checked together on every cycle.
#[derive(Clone)]
pub struct RunControl {
    execution_deadline: usize,
    deadline_at:        Instant,
    cancel:             CancelToken,
}

impl RunControl {
    /// Builds a control and returns a [`CancelToken`] sharing its cancel flag.
    ///
    /// * `execution_deadline` – max cycles before `SolveError::ExecutionDeadline`.
    /// * `time_limit` – wall-clock budget before `SolveError::TimeDeadline`.
    pub fn init(execution_deadline: usize, time_limit: Duration) -> (Self, CancelToken) {
        let cancel = CancelToken::new();
        let control = Self {
            execution_deadline,
            deadline_at: Instant::now() + time_limit,
            cancel: cancel.clone(),
        };
        (control, cancel)
    }

    /// `Err` with the specific reason if the run must stop at `cycle`.
    /// Cancellation takes precedence over the budgets.
    fn check(&self, cycle: usize) -> Result<(), SolveError> {
        if self.cancel.is_cancelled() {
            return Err(SolveError::Canceled);
        }
        if cycle > self.execution_deadline {
            return Err(SolveError::ExecutionDeadline);
        }
        if Instant::now() >= self.deadline_at {
            return Err(SolveError::TimeDeadline);
        }
        Ok(())
    }
}

/// Shared by every frame of one `solve`. Splitting any of it per subtask
/// changes the cycle count and the cache hits.
struct Run {
    control: RunControl,
    cycle:   usize,
    cache:   HashMap<CacheKey, SharedSolution>,
    tracer:  TracerHub,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum CacheKey {
    Goal(Goal),
    SolveBlock(TermBuf),
}

enum SubtaskEntry {
    Occupied(SharedSolution),
    Vacant(CacheSlot),
}

#[must_use]
struct CacheSlot(CacheKey);

enum AnswerCheck {
    No,
    Found(TermIdx),
    // A derived answer happens at most once per solve.
    Derived(Box<TermProps>),
}

/// Rules derived from a frame's own terms. Insertion-ordered, because the
/// order rules are offered in is part of the search's identity.
#[derive(Default)]
struct LocalRules {
    rules:     IndexMap<RuleId, SharedRule>,
    max_level: Level,
}

/// The rule set a run is solved against, with no state of its own between
/// tasks.
pub struct Solver {
    rules_engine: Arc<RulesEngine>,
}

impl Solver {
    /// Creates a new `Solver` instance.
    ///
    /// # Arguments
    ///
    /// * `rules` – An `Arc` pointing to a `RulesEngine` that provides the global rule set used
    ///   during solving.
    pub fn new(rules: Arc<RulesEngine>) -> Solver {
        Solver {
            rules_engine: rules,
        }
    }

    /// Attempts to solve the given `task` and returns a `SharedSolution`.
    ///
    /// # Parameters
    ///
    /// * `task` – The task to be solved.
    /// * `tracer` – A `TracerHub` used for instrumentation.
    /// * `control` – Run limits (cycle budget, wall-clock deadline, cancellation); build one with
    ///   [`RunControl::init`].
    ///
    /// The returned `SharedSolution` carries either the answer or an error
    /// status.
    pub fn solve(&self, task: Task, tracer: TracerHub, control: RunControl) -> SharedSolution {
        let mut solution = Solution::new(task);
        let mut state = Run {
            control,
            cycle: Default::default(),
            cache: Default::default(),
            tracer,
        };

        self.solve_impl(&mut solution, &mut state);
        solution.into()
    }

    /// Runs one (sub)task to a terminal status.
    fn solve_impl(&self, solution: &mut Solution, run: &mut Run) {
        if solution.task.subtask_level > MAX_SUBTASK_LEVEL {
            solution.status = SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed);
            return;
        }
        let mut local = LocalRules::default();
        run.tracer.on_subtask_start(&solution.task, run.cycle());

        solution.start_cycle = run.cycle;

        // TODO: can be replaced with try { .. } in future
        // track: https://github.com/rust-lang/rust/issues/31436
        let mut main_loop = || loop {
            run.begin_cycle()?;
            let index = self.try_focus_term(solution, run, &local)?;
            if self.try_simplify(index, solution, run, &mut local)? {
                continue;
            }
            let found = match self.check_if_answer(solution, run, index) {
                AnswerCheck::No => None,
                AnswerCheck::Found(i) => Some(i),
                AnswerCheck::Derived(props) => Some(solution.add_term(*props)?),
            };
            if let Some(i) = found {
                solution.status = SolutionStatus::Answer(i);
                let level = solution.task.subtask_level;
                trace!("Solved {level}. Answer: {}", solution[i].term);
                break Ok(());
            }
            self.add_local_rule(&mut solution[index], &mut local);
            self.try_infer_new_terms(index, solution, run, &mut local)?;
        };
        if let Err(e) = main_loop() {
            solution.status = SolutionStatus::Err(e);
        }
        solution.end_cycle = run.cycle;
        run.tracer.on_subtask_end(solution);
    }

    fn try_focus_term(
        &self,
        solution: &mut Solution,
        run: &mut Run,
        local: &LocalRules,
    ) -> Result<TermIdx, SolveError> {
        let index = solution.pick_next().ok_or(SolveError::NoConditions)?;
        run.tracer.on_term_focus(&solution[index], run.cycle());
        let level = solution[index].filters.level;

        trace!(target: "subtask",
            "[{}]({}) Level: {level} -> {}",
            solution.task.subtask_level,
            run.cycle(),
            solution[index]
        );

        if level > std::cmp::max(local.max_level, self.rules_engine.max_level()).next() {
            return Err(SolveError::NoSolutionsFound);
        }
        Ok(index)
    }

    fn try_simplify(
        &self,
        index: usize,
        solution: &mut Solution,
        run: &mut Run,
        local: &mut LocalRules,
    ) -> Result<bool, SolveError> {
        if solution.goal().kind() == GoalKind::Transform {
            Ok(false)
        } else if let Some(simplified) = self.transform(solution, run, index) {
            solution[index].filters.mark_replaced();
            self.add_term(simplified, solution, run, local)?;
            Ok(true)
        } else {
            solution[index].filters.mark_simplified();
            Ok(false)
        }
    }

    fn add_term(
        &self,
        term: TermProps,
        s: &mut Solution,
        run: &mut Run,
        local: &mut LocalRules,
    ) -> Result<TermIdx, SolveError> {
        run.tracer.on_new_term(
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
            self.add_local_rule(&mut s[index], local);
        }
        Ok(index)
    }

    fn add_local_rule(&self, term: &mut TermProps, local: &mut LocalRules) {
        if term.filters.is_goal() {
            return;
        }
        let level = term.filters.level;
        if let Some(r) = term.rule(
            RuleId::new(LOCAL_RULE_MASK, local.rules.len() as u64 + 1),
            level.next(),
        ) {
            local.insert(r);
        }
    }

    // TODO: единый обьект для goal
    fn suggest_rules(
        &self,
        term: &TermProps,
        goal_term: &TermBuf,
        kind: GoalKind,
        local: &LocalRules,
    ) -> Vec<SharedRule> {
        if kind == GoalKind::Transform && !term.filters.is_goal() {
            return vec![];
        }

        let local_rules = local
            .rules
            .values()
            .filter(|rule| rule.try_filter(&term.filters, goal_term).is_ok())
            .cloned();
        let rules = self.rules_engine.suggest_rules(&term.filters, goal_term);
        let rules = rules.into_iter().chain(local_rules);

        let rules: Vec<_> = if kind == GoalKind::Prove && term.filters.is_goal() {
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
        &self,
        index: TermIdx,
        solution: &mut Solution,
        run: &mut Run,
        local: &mut LocalRules,
    ) -> Result<(), SolveError> {
        let is_goal = solution[index].filters.is_goal();
        let kind = solution.goal().kind();
        let goal_term = if is_goal && kind == GoalKind::Prove {
            SharedTerm::new(TermBuf::symbol("prove").arg(solution[index].term.as_ref().clone()))
        } else {
            solution.task.goal().term().clone()
        };

        let mut added = false;
        for rule in self.suggest_rules(&solution[index], goal_term.as_ref(), kind, local) {
            match self.produce(&rule, solution, run, index, goal_term.as_ref())? {
                Some(s) => {
                    trace!("{} => {s}", solution[index]);
                    self.add_term(s, solution, run, local)?;
                    if is_goal && kind == GoalKind::Transform {
                        // TODO: унифицировать weight = MAX_LEVEL и REPLACED
                        solution[index].filters.level = self.rules_engine.max_level().next();
                        solution.requeue(index);
                        // CONTEXT: the break skips `added = true` on purpose.
                        // The tail then raises the level again, and the search
                        // order depends on that.
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
        run: &mut Run,
        index: TermIdx,
        goal_term: &TermBuf,
    ) -> Result<Option<TermProps>, SolveError> {
        let is_goal = s[index].filters.is_goal();
        // Collected up front to release the `&s` borrow before the loop mutates
        // `s`. The rest is lazy: `resolve_solve_in_hypothesis` and `ground()`
        // run only for hypotheses actually reached, short-circuiting on proof.
        let raw_hypotheses: Vec<Hypothesis> =
            HypothesisIterator::new(rule.clone(), &s[index].term, &s[index].filters, goal_term)
                .collect();
        let mut grounded_seen = 0usize;
        for mut h in raw_hypotheses {
            resolve_parents_in_hypothesis(&mut h, s[index].term.as_ref());
            let Some(h) = self.resolve_solve_in_hypothesis(h, s, run) else {
                continue;
            };
            for hypothesis in h.ground(&s.known_vars) {
                grounded_seen += 1;
                if grounded_seen.is_multiple_of(DEADLINE_CHECK_INTERVAL) {
                    run.control.check(run.cycle)?;
                }
                let is_dub = if is_goal {
                    s.goal_index.contains_key(&hypothesis.resolution)
                } else {
                    s.main_index.contains_key(&hypothesis.resolution)
                };
                if is_dub {
                    continue;
                }
                let props = self.try_prove_hypothesis(index, s, hypothesis, run);
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
        run: &mut Run,
    ) -> TermProps {
        trace!(
            target: "rule_selection",
            "new hypothesis {hypothesis}, rule {}, term: {}",
            hypothesis.rule, solution[parent_idx]
        );
        run.tracer
            .on_new_hypothesis(solution[parent_idx].term.clone(), &hypothesis, run.cycle());

        let mut props = TermProps::from(hypothesis.resolution.clone());
        props.filters.blocked_rules = hypothesis.blocked_rules.iter().cloned().collect();
        if solution[parent_idx].filters.is_goal() {
            props.filters.mark_goal();
        }
        let mut req_proofs = vec![];
        let mut iter = hypothesis.requirements.clone().into_iter();
        for i in iter.by_ref() {
            req_proofs.push(self.prove(solution, i, run));
            let last = req_proofs.last().unwrap();
            if last.answer().is_none() {
                trace!(
                    target: "rule_selection",
                    "term {} rejected, requirement not proven {}",
                    hypothesis.resolution,
                    last.task.goal()
                );
                break;
            }
        }
        for req in iter {
            req_proofs.push(SharedSolution::new(Solution::new(Task::from_goal(
                Goal::prove(req),
            ))));
        }
        props.inference = TermInference::Rule {
            rule:         hypothesis.rule.clone(),
            params:       hypothesis.params.clone(),
            parent:       parent_idx,
            requirements: req_proofs,
        };

        run.tracer
            .on_hypothesis_finish(&props.inference, run.cycle());

        if props.inference.is_proven() {
            trace!(
                target: "rule_selection",
                "hypothesis {hypothesis} proven, resolution {} applied",
                hypothesis.resolution
            );
        }
        props
    }

    fn prove(&self, solution: &Solution, mut term: TermBuf, run: &mut Run) -> SharedSolution {
        is_replace(&mut term.term_mut());
        let term = term.normalize(NormLevel::Full);

        match term.term().truth(TruthCtx::new(&solution.known_vars)) {
            // The proven term is itself the answer.
            Truth::True => {
                let mut trivial_solution =
                    Solution::new(Task::from_goal(Goal::prove(term.clone())));
                trivial_solution.status = match trivial_solution.add_term(TermProps::from(term)) {
                    Ok(idx) => SolutionStatus::Answer(idx),
                    Err(e) => SolutionStatus::Err(e),
                };
                SharedSolution::new(trivial_solution)
            }
            // Unprovable — fail fast instead of searching.
            Truth::False => {
                let mut no_solution = Solution::new(Task::from_goal(Goal::prove(term)));
                no_solution.status = SolutionStatus::Err(SolveError::NoSolutionsFound);
                SharedSolution::new(no_solution)
            }
            Truth::Unknown => self.solve_subtask(solution, Goal::prove(term), HashSet::new(), run),
        }
    }

    fn transform(&self, solution: &mut Solution, run: &mut Run, index: usize) -> Option<TermProps> {
        let term = &mut solution[index];
        if term.filters.is_simplified() {
            return None;
        }
        term.filters.mark_simplified();

        let inner = answer::marked(term.term.term());
        let use_answer = inner.is_some();
        let to_transform = inner.unwrap_or(term.term.term()).to_owned();
        let blocked = solution[index].filters.blocked_rules.clone();
        let subtask_solution =
            self.solve_subtask(solution, Goal::transform(to_transform), blocked, run);

        let mut transformed = subtask_solution.answer()?.as_ref().clone();
        if use_answer {
            transformed = answer::mark(transformed);
        }

        if *solution[index].term == transformed {
            return None;
        }
        let mut result = TermProps::from(transformed);
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

    /// Runs `goal` as a nested task, cached by the goal term.
    fn solve_subtask(
        &self,
        solution: &Solution,
        goal: Goal,
        blocked_rules: HashSet<RuleId>,
        run: &mut Run,
    ) -> SharedSolution {
        let slot = match run.subtask_entry(CacheKey::Goal(goal.clone()), &goal) {
            SubtaskEntry::Occupied(cached) => return cached,
            SubtaskEntry::Vacant(slot) => slot,
        };

        let mut subtask_solution = solution.subtask(goal.clone(), blocked_rules, Vec::new());
        self.solve_impl(&mut subtask_solution, run);
        let subtask_solution = SharedSolution::new(subtask_solution);
        if let SolutionStatus::Err(e) = subtask_solution.status {
            trace!("Can't prove {}: {e}", goal.term());
        }
        run.fill(slot, subtask_solution)
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
        run: &mut Run,
    ) -> SharedSolution {
        let block_goal = match Goal::parse(goal.clone()) {
            Ok(g) => g,
            Err(e) => {
                error!("solve block goal is not a goal: {e}");
                return Self::errored_subtask(Goal::prove(goal), SolveError::Internal);
            }
        };

        let slot = match run.subtask_entry(CacheKey::SolveBlock(cache_key), &block_goal) {
            SubtaskEntry::Occupied(cached) => return cached,
            SubtaskEntry::Vacant(slot) => slot,
        };

        let mut subtask_solution = parent.subtask(
            block_goal,
            HashSet::new(),
            eqs.into_iter().map(SharedTerm::new).collect(),
        );
        self.solve_impl(&mut subtask_solution, run);
        run.fill(slot, SharedSolution::new(subtask_solution))
    }

    /// Replaces `solve(...) == Param` requirements with subtask answer.
    fn resolve_solve_in_hypothesis(
        &self,
        mut hyp: Hypothesis,
        parent: &Solution,
        run: &mut Run,
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

            let result = self.run_solve_block(cache_key, goal, eqs, parent, run);
            let answer_term = result.answer()?;
            let answer_buf = answer::marked(answer_term.term())
                .unwrap_or(answer_term.term())
                .to_owned();

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

    /// A subtask solution carrying only an error status, for when the subtask
    /// could not even be built.
    fn errored_subtask(goal: Goal, e: SolveError) -> SharedSolution {
        let mut solution = Solution::new(Task::from_goal(goal));
        solution.status = SolutionStatus::Err(e);
        SharedSolution::new(solution)
    }

    fn check_if_answer(&self, solution: &mut Solution, run: &mut Run, index: usize) -> AnswerCheck {
        if solution[index].filters.is_goal() {
            // Prove goal reduced to a trivial truth: solved even with no
            // non-goal term to focus.
            if solution.goal().kind() == GoalKind::Prove &&
                let Some(i) = solution.goal_index.values().copied().find(|i| {
                    solution[*i]
                        .term
                        .term()
                        .truth(TruthCtx::new(&solution.known_vars))
                        .is_true()
                })
            {
                return AnswerCheck::Found(i);
            }
            return AnswerCheck::No;
        }
        if let AnswerCheck::Derived(props) = self.check_answer_term(solution, index) {
            return AnswerCheck::Derived(props);
        }

        // The arms need the solution mutably, and the kind is `Copy`.
        match solution.goal().kind() {
            GoalKind::Find => self.check_find_answer(solution, run, index),
            GoalKind::Prove => self.check_prove_answer(solution, index),
            GoalKind::Transform => self.check_transform_answer(solution),
        }
    }

    fn check_answer_term(&self, solution: &Solution, index: usize) -> AnswerCheck {
        let term = &solution[index];
        let term_root = term.term.term();

        if solution.goal().kind() == GoalKind::Transform {
            return AnswerCheck::No;
        }
        let Some(inner) = answer::marked(term_root) else {
            return AnswerCheck::No;
        };
        let mut resolution = TermProps::from(inner.to_owned());
        resolution.inference = term.inference.clone();
        AnswerCheck::Derived(Box::new(resolution))
    }

    fn check_find_answer(
        &self,
        solution: &mut Solution,
        run: &mut Run,
        index: usize,
    ) -> AnswerCheck {
        let recognized = {
            let mut known = |t: TermRef| self.is_provably_known(t, solution, run);
            solution
                .goal()
                .recognize(solution[index].term.term(), &mut known)
        };
        match recognized {
            Recognized::No => AnswerCheck::No,
            // Single target: the whole answer at once, flat or piecewise.
            Recognized::Whole => AnswerCheck::Found(index),
            Recognized::Binding(target) => {
                if solution.find_bindings.contains_key(&target) {
                    return AnswerCheck::No;
                }
                solution.find_bindings.insert(target, index);
                let targets = solution
                    .goal()
                    .targets()
                    .expect("a find goal carries its targets");
                if solution.find_bindings.len() < targets.len() {
                    return AnswerCheck::No;
                }
                AnswerCheck::Derived(Box::new(self.build_multi_find_answer(solution, &targets)))
            }
        }
    }

    /// One-level answer form for a single `target`: an answer leaf, a `&&`
    /// branch (one leaf + `is known` guards), or a `||` of such branches.
    /// `true` if `term is known` is provable.
    fn is_provably_known(&self, term: TermRef, solution: &Solution, run: &mut Run) -> bool {
        let query = TermBuf::symbol("is")
            .arg(term.to_owned())
            .arg(TermBuf::symbol("known"));
        self.prove(solution, query, run).answer().is_some()
    }

    fn build_multi_find_answer(&self, solution: &Solution, targets: &[TermBuf]) -> TermProps {
        let mut iter = targets.iter();
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

    fn check_prove_answer(&self, solution: &Solution, index: usize) -> AnswerCheck {
        let term = &solution[index];

        // A candidate with unresolved requirements is not a proof — accepting it would close
        // circular hypotheses (e.g. `[x^2=a] => x^2=a`) where a rule's own resolution proves its
        // own requirement.
        if !term.is_proven() {
            return AnswerCheck::No;
        }

        // TODO: теперь тут бывают целевые термы, поэтому надо сделать две проверки:
        // что терм есть среди целей
        // что цель тривиальная истина
        for i in solution.goal_index.values() {
            if term.term == solution[*i].term {
                return AnswerCheck::Found(index);
            }
            // A derived numeric bound (`x > 2`) proves a weaker goal bound
            // (`x > 0`) on the same expression, even without a syntactic match.
            if bound_implies(term.term.term(), solution[*i].term.term()) {
                return AnswerCheck::Found(index);
            }
            if solution[*i]
                .term
                .term()
                .truth(TruthCtx::new(&solution.known_vars))
                .is_true()
            {
                // TODO: у этого терма нет происхождения — заполнить, когда вывод решения по шагам
                // станет его показывать.
                return AnswerCheck::Found(*i);
            }
        }
        AnswerCheck::No
    }

    fn check_transform_answer(&self, solution: &Solution) -> AnswerCheck {
        let Some(index) = solution.pick_goal_term() else {
            return AnswerCheck::No;
        };
        if solution[index].filters.level < self.rules_engine.max_level() {
            return AnswerCheck::No;
        }
        // TODO: у этого терма нет происхождения — заполнить, когда вывод решения по шагам станет
        // его показывать.
        match solution
            .terms
            .iter()
            .rev()
            .filter(|x| x.is_proven())
            .find(|x| x.filters.is_goal())
        {
            Some(res) => AnswerCheck::Found(res.id),
            None => AnswerCheck::No,
        }
    }
}

impl LocalRules {
    fn insert(&mut self, rule: SharedRule) {
        let level = rule.level;
        if self.rules.insert(rule.id, rule).is_none() {
            self.max_level = std::cmp::max(self.max_level, level);
        }
    }
}

impl Run {
    fn cycle(&self) -> usize {
        self.cycle
    }

    fn subtask_entry(&mut self, key: CacheKey, goal: &Goal) -> SubtaskEntry {
        match self.cache.entry(key) {
            Entry::Occupied(e) => SubtaskEntry::Occupied(e.get().clone()),
            Entry::Vacant(e) => {
                let key = e.key().clone();
                e.insert(Solution::new(Task::from_goal(goal.clone())).into());
                SubtaskEntry::Vacant(CacheSlot(key))
            }
        }
    }

    fn fill(&mut self, slot: CacheSlot, solution: SharedSolution) -> SharedSolution {
        if let SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed) = solution.status {
            self.cache.remove(&slot.0);
        } else {
            self.cache.insert(slot.0, solution.clone());
        }
        solution
    }

    /// Counts the cycle, then checks cancellation, the cycle budget and the
    /// wall clock, in that order.
    fn begin_cycle(&mut self) -> Result<(), SolveError> {
        self.cycle += 1;
        self.control.check(self.cycle)
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

/// A numeric bound on some expression, extracted from a comparison term.
enum BoundKind {
    Lower { strict: bool },
    Upper { strict: bool },
}

/// Parses a comparison term into (expression, bound kind, numeric bound).
///
/// `E > c` / `E >= c` → Lower; `E < c` / `E <= c` → Upper. A number on the
/// left flips the kind: `c < E` reads as `E > c` → Lower for `E`. Returns
/// `None` unless exactly one side is a numeric literal.
fn as_bound<'a>(t: TermRef<'a>) -> Option<(TermRef<'a>, BoundKind, Rational)> {
    if t.degree() != 2 {
        return None;
    }
    let data = t.data();
    let strict = if data.is_symbol_name(">") || data.is_symbol_name("<") {
        true
    } else if data.is_symbol_name(">=") || data.is_symbol_name("<=") {
        false
    } else {
        return None;
    };
    let greaterish = data.is_symbol_name(">") || data.is_symbol_name(">=");

    let lhs = t.first_arg()?;
    let rhs = t.last_arg()?;
    if let Some(c) = rhs.data().number() {
        let kind = if greaterish {
            BoundKind::Lower { strict }
        } else {
            BoundKind::Upper { strict }
        };
        Some((lhs, kind, c.clone()))
    } else if let Some(c) = lhs.data().number() {
        // A number on the left flips the orientation for the expression `rhs`.
        let kind = if greaterish {
            BoundKind::Upper { strict }
        } else {
            BoundKind::Lower { strict }
        };
        Some((rhs, kind, c.clone()))
    } else {
        None
    }
}

/// `derived ⇒ goal` for two comparisons over the same expression and numeric
/// bounds. `x > 2` implies `x > 0`; `x >= 2` implies `x > 0`; `x >= 0` implies
/// `x >= 0`. Mismatched expressions or bound kinds imply nothing.
fn bound_implies(derived: TermRef, goal: TermRef) -> bool {
    let Some((de, dk, dc)) = as_bound(derived) else {
        return false;
    };
    let Some((ge, gk, gc)) = as_bound(goal) else {
        return false;
    };
    if de != ge {
        return false;
    }
    match (dk, gk) {
        (BoundKind::Lower { strict: ds }, BoundKind::Lower { strict: gs }) => {
            dc > gc || (dc == gc && (ds || !gs))
        }
        (BoundKind::Upper { strict: ds }, BoundKind::Upper { strict: gs }) => {
            dc < gc || (dc == gc && (ds || !gs))
        }
        _ => false,
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
        engine::{RunControl, Solver, TIME_LIMIT_DEFAULT},
        rule::RulesEngine,
        task::parse_task,
        term::term_with_vars,
    };

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task { goal find(x); x == 1; }");
        let rules = Arc::new(RulesEngine::default());
        let solver = Solver::new(rules);
        let solution = solver.solve(
            task,
            Default::default(),
            RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
        );
        assert_eq!(
            *solution.answer().expect("task is not solved"),
            term_with_vars("x == 1")
        );
    }

    #[test]
    fn check_answer_prove_test() {
        let task = parse_task("task { goal prove(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let solver = Solver::new(rules);
        let solution = solver.solve(
            task,
            Default::default(),
            RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
        );
        assert!(solution.answer().is_some());
    }

    fn prove_solved(src: &'static str) -> bool {
        let task = parse_task(src);
        let rules = Arc::new(RulesEngine::default());
        let solver = Solver::new(rules);
        let solution = solver.solve(
            task,
            Default::default(),
            RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
        );
        solution.answer().is_some()
    }

    #[test]
    fn bound_implies_lower_strict() {
        // x > 2 proves x > 0.
        assert!(prove_solved("task { goal prove(x > 0); x > 2; }"));
    }

    #[test]
    fn bound_implies_strict_to_nonstrict_equal() {
        // x > 0 proves x >= 0 (equal bounds, strict implies non-strict).
        assert!(prove_solved("task { goal prove(x >= 0); x > 0; }"));
    }

    #[test]
    fn bound_implies_upper() {
        // x <= -1 proves x < 0.
        assert!(prove_solved("task { goal prove(x < 0); x <= -1; }"));
    }

    #[test]
    fn bound_implies_number_on_left() {
        // 0 < x proves x > 0.
        assert!(prove_solved("task { goal prove(x > 0); 0 < x; }"));
    }

    #[test]
    fn bound_implies_rejects_stronger_goal() {
        // x > 2 does not prove x > 3.
        assert!(!prove_solved("task { goal prove(x > 3); x > 2; }"));
    }

    #[test]
    fn bound_implies_rejects_mismatched_kind() {
        // x < 5 (upper) does not prove x > 0 (lower).
        assert!(!prove_solved("task { goal prove(x > 0); x < 5; }"));
    }

    #[test]
    fn prove_trivial_truth_without_condition() {
        // A goal that reduces to a trivial truth is solved even with no
        // non-goal term to focus (no fictitious witness condition needed).
        assert!(prove_solved("task { goal prove(1 > 0); }"));
    }

    #[test]
    fn check_answer_multi_var_find() {
        let task = parse_task("task { goal find(x, y); x == 3; y == 4; }");
        let rules = Arc::new(RulesEngine::default());
        let solver = Solver::new(rules);
        let solution = solver.solve(
            task,
            Default::default(),
            RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
        );
        let answer = solution
            .answer()
            .expect("multi-var find task is not solved");
        assert_eq!(*answer, term_with_vars("x == 3 && y == 4"));
    }

    /// A rule derived from the first task's terms may not reach the second.
    #[test]
    fn solver_carries_nothing_between_tasks() {
        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let control = || RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT).0;

        let _first = solver.solve(
            parse_task("task { goal find(x); x == 1; }"),
            Default::default(),
            control(),
        );
        let second = solver.solve(
            parse_task("task { goal find(y); y == 2; }"),
            Default::default(),
            control(),
        );

        let fresh = Solver::new(Arc::new(RulesEngine::default())).solve(
            parse_task("task { goal find(y); y == 2; }"),
            Default::default(),
            control(),
        );

        assert_eq!(
            *second.answer().expect("solved"),
            *fresh.answer().expect("solved")
        );
        assert_eq!(second.cycles(), fresh.cycles());
    }

    #[test]
    fn cancelled_token_aborts_solve() {
        use crate::engine::{SolutionStatus, SolveError};

        let task = parse_task("task { goal find(x); x == 1; }");
        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let (control, cancel) = RunControl::init(usize::MAX, TIME_LIMIT_DEFAULT);
        // Cancel before solving: the first cycle check must abort the run.
        cancel.cancel();
        let solution = solver.solve(task, Default::default(), control);
        assert!(matches!(
            solution.status,
            SolutionStatus::Err(SolveError::Canceled)
        ));
    }
}

#[cfg(test)]
mod cache_tests {
    use std::time::Duration;

    use super::{
        CacheKey, Run, RunControl, SharedSolution, Solution, SolutionStatus, SolveError,
        SubtaskEntry, Task,
    };
    use crate::{
        engine::TracerHub,
        task::Goal,
        term::{TermBuf, term_with_vars},
    };

    fn run() -> Run {
        Run {
            control: RunControl::init(usize::MAX, Duration::from_secs(60)).0,
            cycle:   0,
            cache:   Default::default(),
            tracer:  TracerHub::default(),
        }
    }

    fn goal(src: &'static str) -> Goal {
        Goal::parse(term_with_vars(src)).expect("a goal")
    }

    #[test]
    fn a_second_reservation_of_one_key_hits_the_placeholder() {
        let mut run = run();
        let g = goal("prove(x > 0)");

        let SubtaskEntry::Vacant(_slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("the first reservation takes the key");
        };
        let SubtaskEntry::Occupied(placeholder) = run.subtask_entry(CacheKey::Goal(g.clone()), &g)
        else {
            panic!("the second must hit, not take the key again");
        };
        // No answer, so the caller drops its hypothesis instead of looping.
        assert!(placeholder.answer().is_none());
    }

    #[test]
    fn a_depth_failure_releases_the_key() {
        let mut run = run();
        let g = goal("prove(x > 0)");
        let SubtaskEntry::Vacant(slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };

        let mut failed = Solution::new(Task::from_goal(g.clone()));
        failed.status = SolutionStatus::Err(SolveError::MaxSubtaskLevelExceed);
        run.fill(slot, SharedSolution::new(failed));

        // Met higher up, the same subtask has to be solvable afresh.
        assert!(matches!(
            run.subtask_entry(CacheKey::Goal(g.clone()), &g),
            SubtaskEntry::Vacant(_)
        ));
    }

    #[test]
    fn any_other_failure_stays_cached() {
        let mut run = run();
        let g = goal("prove(x > 0)");
        let SubtaskEntry::Vacant(slot) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };

        let mut failed = Solution::new(Task::from_goal(g.clone()));
        failed.status = SolutionStatus::Err(SolveError::NoSolutionsFound);
        run.fill(slot, SharedSolution::new(failed));

        let SubtaskEntry::Occupied(cached) = run.subtask_entry(CacheKey::Goal(g.clone()), &g)
        else {
            panic!("a settled subtask stays settled");
        };
        assert!(matches!(
            cached.status,
            SolutionStatus::Err(SolveError::NoSolutionsFound)
        ));
    }

    #[test]
    fn a_goal_and_a_solve_call_are_different_keys() {
        let mut run = run();
        let g = goal("find(x)");
        let call = TermBuf::symbol("solve").arg(term_with_vars("find(x)"));

        let SubtaskEntry::Vacant(_) = run.subtask_entry(CacheKey::Goal(g.clone()), &g) else {
            panic!("reserved");
        };
        assert!(
            matches!(
                run.subtask_entry(CacheKey::SolveBlock(call), &g),
                SubtaskEntry::Vacant(_)
            ),
            "a solve(...) call must not collide with the goal inside it"
        );
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
        engine::{CancelToken, RunControl, Solution, Solver, TracerHub},
        rule::{Hypothesis, RulesEngine, SharedRule, parse_rule},
        task::parse_task,
        term::{ParamSubstitution, TermBuf, TermPath, term_with_vars},
    };

    use super::Run;

    /// Two `solve(...) == Param` requirements must both reach the resolution.
    /// The old single-binding code kept only the last and dropped the first.
    #[test]
    fn two_solve_bindings_both_apply() {
        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let parent = Solution::new(parse_task("task { goal find(z); }"));
        let mut state = Run {
            control: RunControl::init(usize::MAX, Duration::from_secs(60)).0,
            cycle:   0,
            cache:   Default::default(),
            tracer:  TracerHub::default(),
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
        use crate::engine::{SolveError, TermProps};

        let solver = Solver::new(Arc::new(RulesEngine::default()));
        let mut solution = Solution::new(parse_task("task { goal find(z); }"));
        // Focus term for the rule, plus a term equal to every grounding's resolution so each
        // hypothesis is a duplicate and the loop keeps going.
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

        let mut state = Run {
            // Deadline already in the past: produce's poll must bail out.
            control: RunControl {
                execution_deadline: usize::MAX,
                deadline_at:        Instant::now() - Duration::from_secs(1),
                cancel:             CancelToken::default(),
            },
            cycle:   0,
            cache:   Default::default(),
            tracer:  TracerHub::default(),
        };

        let goal_term = solution.task.goal().term().clone();
        let err = solver
            .produce(&rule, &mut solution, &mut state, index, goal_term.as_ref())
            .expect_err("produce should abort on a passed deadline");
        assert!(matches!(err, SolveError::TimeDeadline));
    }
}
