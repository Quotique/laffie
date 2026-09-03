use std::{collections::HashSet, sync::Arc};

use indexmap::IndexMap;
use itertools::Itertools;

use super::{
    SharedSolution, Solution, SolveError, TermIdx, TermProps, TracerHub,
    bounds::bound_implies,
    props::TermInference,
    run::{CacheKey, Limits, Run, SubtaskEntry},
};
use crate::{
    NormLevel,
    engine::{Tracer, solution::SolutionStatus},
    rule::{
        GroundedHypothesis, Hypothesis, HypothesisIterator, Level, RuleAttr, RuleId, RulesEngine,
        SharedRule,
    },
    task::{Goal, GoalKind, Task},
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

/// How often (in grounded hypotheses) `produce` polls the run limits.
const DEADLINE_CHECK_INTERVAL: usize = 64;

/// Id namespace for a frame's own rules, clear of the ids the rule engine
/// hands out.
const LOCAL_RULE_MASK: u64 = 0x80_00_00_00_00_00_00_00;

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

    /// The returned solution carries either the answer or an error status.
    pub fn solve(&self, task: Task, tracer: TracerHub, limits: Limits) -> SharedSolution {
        let mut solution = Solution::new(task);
        let mut state = Run {
            limits,
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
            h.resolve_parents(s[index].term.as_ref());
            let Some(h) = self.resolve_solve_in_hypothesis(h, s, run) else {
                continue;
            };
            for hypothesis in h.ground(&s.known_vars) {
                grounded_seen += 1;
                if grounded_seen.is_multiple_of(DEADLINE_CHECK_INTERVAL) {
                    run.limits.check(run.cycle)?;
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
        let at = {
            let mut known = |t: TermRef| self.is_provably_known(t, solution, run);
            solution
                .goal()
                .recognize(solution[index].term.term(), &mut known)
        };
        let Some(at) = at else {
            return AnswerCheck::No;
        };
        // One unknown: the focused term is the whole answer, flat or piecewise.
        if solution.goal().parts() == 1 {
            return AnswerCheck::Found(index);
        }
        let term = solution[index].term.clone();
        let Some(answer) = solution.find_answer.as_mut() else {
            return AnswerCheck::No;
        };
        if !answer.bind(at, term, index) {
            return AnswerCheck::No;
        }
        match answer.term() {
            Some(term) => AnswerCheck::Derived(Box::new(TermProps::from(term))),
            None => AnswerCheck::No,
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
mod solve_tests {
    use std::sync::Arc;

    use crate::{
        engine::{Limits, Solver, TIME_LIMIT_DEFAULT},
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
            Limits::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
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
            Limits::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
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
            Limits::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
        );
        solution.answer().is_some()
    }

    #[test]
    fn a_numeric_bound_reaches_the_prove_check() {
        assert!(prove_solved("task { goal prove(x > 0); x > 2; }"));
        assert!(!prove_solved("task { goal prove(x > 3); x > 2; }"));
        // The reversed form still works after normalization.
        assert!(prove_solved("task { goal prove(x > 0); 0 < x; }"));
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
            Limits::init(usize::MAX, TIME_LIMIT_DEFAULT).0,
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
        let limits = || Limits::init(usize::MAX, TIME_LIMIT_DEFAULT).0;

        let _first = solver.solve(
            parse_task("task { goal find(x); x == 1; }"),
            Default::default(),
            limits(),
        );
        let second = solver.solve(
            parse_task("task { goal find(y); y == 2; }"),
            Default::default(),
            limits(),
        );

        let fresh = Solver::new(Arc::new(RulesEngine::default())).solve(
            parse_task("task { goal find(y); y == 2; }"),
            Default::default(),
            limits(),
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
        let (limits, cancel) = Limits::init(usize::MAX, TIME_LIMIT_DEFAULT);
        // Cancel before solving: the first cycle check must abort the run.
        cancel.cancel();
        let solution = solver.solve(task, Default::default(), limits);
        assert!(matches!(
            solution.status,
            SolutionStatus::Err(SolveError::Canceled)
        ));
    }
}

#[cfg(test)]
mod local_rules_tests {
    use std::sync::Arc;

    use super::LocalRules;
    use crate::rule::{Level, RuleId, SharedRule, parse_rule};

    fn rule(id: u64, level: u64) -> SharedRule {
        let mut rule = parse_rule("rule { attr level(1); a + b => a == b; }");
        rule.id = RuleId::new(0, id);
        rule.level = level.into();
        Arc::new(rule)
    }

    #[test]
    fn an_empty_set_has_the_lowest_max_level() {
        assert_eq!(LocalRules::default().max_level, Level::default());
    }

    #[test]
    fn the_max_level_is_the_highest_one_inserted() {
        let mut local = LocalRules::default();
        local.insert(rule(1, 3));
        local.insert(rule(2, 1));

        assert_eq!(local.rules.len(), 2);
        assert_eq!(local.max_level, 3.into());
    }
}

#[cfg(test)]
mod resolve_solve_tests {
    use std::{sync::Arc, time::Duration};

    use crate::{
        engine::{Limits, Solution, Solver, TracerHub},
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
            limits: Limits::init(usize::MAX, Duration::from_secs(60)).0,
            cycle:  0,
            cache:  Default::default(),
            tracer: TracerHub::default(),
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
            limits: Limits::init(usize::MAX, Duration::ZERO).0,
            cycle:  0,
            cache:  Default::default(),
            tracer: TracerHub::default(),
        };

        let goal_term = solution.task.goal().term().clone();
        let err = solver
            .produce(&rule, &mut solution, &mut state, index, goal_term.as_ref())
            .expect_err("produce should abort on a passed deadline");
        assert!(matches!(err, SolveError::TimeDeadline));
    }
}
