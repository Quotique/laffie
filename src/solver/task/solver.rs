use std::{collections::HashMap, rc::Rc, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use utils::VecDisplay;

use super::{cache::TaskStatus, Purpose, Solution, SolutionTracer, Task, TaskBuilder, TermProps};
use crate::{
    rule::{Hypothesis, HypothesisIterator, Rule, RuleAttr, RuleId, RulesEngine, SharedRule},
    term::{SubtermMut, Term},
    NormalizationLevel,
};

pub const MAX_SUBTASK_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;
pub const STACK_SIZE: usize = 2048;
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

pub struct Solver {
    main_index:    HashMap<Rc<Term>, usize>,
    purpose_index: HashMap<Rc<Term>, usize>,

    local_rules: Vec<SharedRule>,

    execution_deadline: usize,
    rules_engine:       Arc<RulesEngine>,
    pub tracer:         SolutionTracer,
}

#[derive(Debug, Display, Clone, Copy, Encode, Decode)]
pub enum SolverError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
    ExecutionDeadline,
}

impl Solver {
    pub fn new(
        rules: Arc<RulesEngine>,
        tracer: SolutionTracer,
        execution_deadline: usize,
    ) -> Solver {
        Solver {
            main_index: Default::default(),
            purpose_index: Default::default(),

            local_rules: vec![],

            rules_engine: rules.clone(),
            tracer,
            execution_deadline,
        }
    }

    // TODO: fix it
    #[allow(clippy::result_large_err)]
    pub fn solve(&mut self, task: Task) -> Result<Solution, (Solution, SolverError)> {
        let mut solution = Solution::new(task);
        match self.solve_alt(&mut solution) {
            Ok(_) => Ok(solution),
            Err(err) => Err((solution, err)),
        }
    }

    pub fn solve_alt(&mut self, solution: &mut Solution) -> Result<Rc<Term>, SolverError> {
        let conditions = solution.task.conditions.clone();
        for i in conditions.into_iter() {
            let _ = self.add_main(solution, i);
        }
        let _ = self.add_purpose(solution, solution.purpose.term().clone());

        trace!(
            target: "subtask",
            "Subtask: {}, {}",
            solution.purpose,
            VecDisplay(&solution.task.conditions)
        );
        if solution.task.subtask_level > MAX_SUBTASK_LEVEL {
            return Err(SolverError::MaxSubtaskLevelExceed);
        }

        self.tracer
            .on_subtask_start(&solution.task, solution.current_cycles());
        solution
            .profiler
            .on_subtask_start(&solution.task, solution.current_cycles());

        let result = self.solver_loop(solution);

        self.tracer.clone().on_subtask_end(solution);
        solution.profiler.on_subtask_end(
            solution.current_cycles(),
            solution.answer().map(|x| x.to_string()),
        );
        result
    }

    fn solver_loop(&mut self, solution: &mut Solution) -> Result<Rc<Term>, SolverError> {
        loop {
            solution.increment_cycles();
            if solution.current_cycles() > self.execution_deadline {
                return Err(SolverError::ExecutionDeadline);
            }

            let index = solution.pick_term().ok_or(SolverError::NoConditions)?;
            self.tracer.on_term_focus(&solution.terms[index]);

            let level = solution.terms[index].filters.weight;
            trace!(
                target: "subtask",
                "[{}]({}) Level: {} -> {}",
                solution.task.subtask_level,
                solution.current_cycles(),
                level, solution.terms[index]
            );
            if level > MAX_LEVEL {
                return Err(SolverError::NoSolutionsFound);
            }
            self.prepare_purpose(solution, level);

            if !solution.purpose.is_transform() {
                if let Some(simplified) = self.transform(solution, index) {
                    solution.terms[index].filters.mark_replaced();
                    self.add_main(solution, simplified).unwrap();
                    continue;
                } else {
                    solution.terms[index].filters.mark_simplified();
                }
            }

            if let Some(term) = self.is_answer(solution, index) {
                trace!("Resolution: {}", term);
                if *solution.terms[index].term == *term.term {
                    trace!("Equivalence");
                    solution.answer = Some(index);
                } else {
                    let id = self.add_main(solution, term)?;
                    solution.answer = Some(id);
                }
                trace!(
                    "Solved {}. Answer: {}",
                    solution.task.subtask_level,
                    solution.answer().unwrap()
                );
                return Ok(solution.answer().unwrap());
            }
            if let Some(r) = solution.terms[index].rule(
                RuleId::new(0x80_00_00_00_00_00_00_00, self.local_rules.len() as u64 + 1),
                (level + 1) as u64,
            ) {
                // TODO: check dups
                self.local_rules.push(r);
            }

            if !solution.purpose.is_transform() {
                let mut added = false;
                for rule in self.suggest_rules(solution, index, |_| true, None) {
                    match HypothesisIterator::new(
                        rule.clone(),
                        &solution.terms[index].term,
                        &solution.terms[index].filters,
                        &solution.task.purpose.term,
                    )
                    .filter(|hypothesis| !self.main_index.contains_key(&hypothesis.resolution))
                    .filter_map(|hypothesis| {
                        self.hypothesis_proof(Some(index), solution, hypothesis)
                    })
                    .next()
                    {
                        Some(s) => {
                            trace!("{} => {}", solution.terms[index], s);
                            self.add_main(solution, s)?;
                            added = true;
                            break;
                        }
                        None => {
                            solution.terms[index].filters.applied_rules.insert(rule.id);
                        }
                    }
                }
                if !added {
                    solution.terms[index].filters.weight += 1;
                }
            } else {
                solution.terms[index].filters.weight += 1;
            }
        }
    }

    fn add_main(&mut self, solution: &mut Solution, term: TermProps) -> Result<usize, SolverError> {
        if let Some(id) = self.main_index.get(&term.term) {
            return Ok(*id);
        }
        let key = term.term.clone();
        let id = self.add_term(solution, term)?;
        self.main_index.insert(key, id);
        Ok(id)
    }

    fn add_purpose(
        &mut self,
        solution: &mut Solution,
        mut term: TermProps,
    ) -> Result<(), SolverError> {
        term.filters.mark_purpose();
        if self.purpose_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(solution, term)?;
        self.purpose_index.insert(key, id);
        Ok(())
    }

    fn add_term(
        &mut self,
        solution: &mut Solution,
        mut term: TermProps,
    ) -> Result<usize, SolverError> {
        self.tracer.on_new_term(
            &term,
            &term
                .inference
                .parent
                .map(|id| solution.terms[id].clone())
                .unwrap_or_else(|| TermProps::from(Rc::new(Term::zero()))),
        );

        let id = solution.terms.len();
        term.inference.id = id;
        if solution.terms.len() + 1 > STACK_SIZE {
            return Err(SolverError::StackOverflow);
        }
        solution.terms.push(term);
        Ok(id)
    }

    fn suggest_rules(
        &self,
        solution: &mut Solution,
        index: usize,
        filter: impl Fn(&Rule) -> bool,
        purpose: Option<&TermProps>,
    ) -> Vec<SharedRule> {
        let purpose = purpose.unwrap_or(&solution.task.purpose);
        let engine_rules = self
            .rules_engine
            .suggest_rules(&solution.terms[index].filters, &purpose.term);
        let suggested_rules: Vec<_> = engine_rules
            .into_iter()
            .chain(self.local_rules.iter().unique().cloned())
            .filter(|rule| filter(rule))
            .collect();
        trace!(target: "rule_selection",
               "purpose: {purpose}, term: {}, suggested_rules: {}",
               solution.terms[index],
               VecDisplay(&suggested_rules)
        );
        suggested_rules
    }

    fn hypothesis_proof(
        &self,
        parent_idx: Option<usize>,
        solution: &mut Solution,
        hypothesis: Hypothesis,
    ) -> Option<TermProps> {
        if let Some(index) = parent_idx {
            trace!(
                target: "rule_selection",
                "new hypothesis {hypothesis}, rule {}, term: {}",
                hypothesis.rule, solution.terms[index]
            );

            self.tracer.on_new_hypothesis(
                solution.terms[index].term.clone(),
                hypothesis.rule.clone(),
                &hypothesis,
                solution.current_cycles(),
            );
            solution.profiler.on_new_hypothesis(
                solution.terms[index].term.clone(),
                hypothesis.rule.clone(),
                &hypothesis,
                solution.current_cycles(),
            );
        }

        let mut proof_res = 0;
        for (num, req) in hypothesis.requirements.iter().enumerate() {
            if self.proof(solution, req).is_none() {
                trace!(target: "rule_selection", "term {} rejected, requirement not proven {req}", hypothesis.resolution);
                break;
            }
            proof_res = num + 1;
        }

        if parent_idx.is_some() {
            self.tracer
                .on_hypothesis_finish(&hypothesis, solution.current_cycles(), proof_res);
            solution.profiler.on_hypothesis_finish(
                &hypothesis,
                solution.current_cycles(),
                proof_res,
            );
        }

        if proof_res == hypothesis.requirements.len() {
            trace!(
                target: "rule_selection",
                "hypothesis {hypothesis} proven, resolution {} applied",
                hypothesis.resolution
            );

            let mut props =
                TermProps::from(Rc::new(hypothesis.resolution)).with_rule(hypothesis.rule);
            if let Some(id) = parent_idx {
                props = props.with_parent(id);
            }
            props.filters.blocked_rules = hypothesis.blocked_rules.into_iter().collect();
            props.inference.requirements =
                hypothesis.requirements.into_iter().map(Rc::new).collect();

            Some(props)
        } else {
            None
        }
    }

    // Returns proof purpose (is a key for tasks cache)
    fn proof(&self, solution: &mut Solution, term: &Term) -> Option<Rc<Term>> {
        let mut clone = term.as_subterm().to_term();
        is_replace(&mut clone.as_subterm_mut());
        // TODO: normalization level
        clone.as_subterm_mut().normalize(NormalizationLevel::max());

        let proof_purpose = Rc::new(Term::symbol("proof").with_child(clone));

        if term.as_subterm().truth().is_true() {
            return Some(proof_purpose);
        }
        self.solve_subtask(solution, proof_purpose.clone())
            .map(|_| proof_purpose)
    }

    fn transform(&mut self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        if solution.terms[index].filters.is_simplified() {
            return None;
        }
        solution.terms[index].filters.mark_simplified();

        let (answer_wrap, to_transform) = if solution.terms[index]
            .term
            .as_subterm()
            .data()
            .is_symbol_name("answer")
        {
            (
                true,
                solution.terms[index]
                    .term
                    .as_subterm()
                    .first_arg()
                    .unwrap()
                    .to_term(),
            )
        } else {
            (false, solution.terms[index].term.as_subterm().to_term())
        };

        let task = Rc::new(Term::symbol("transform").with_child(to_transform));

        let subtask_solver = self.solve_subtask(solution, task.clone())?;

        let mut answer = subtask_solver.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = Term::symbol("answer");
            answer.as_subterm_mut().swap(&mut tmp.as_subterm_mut());
            answer.as_subterm_mut().push_last_arg(tmp);
        }

        if *solution.terms[index].term == answer {
            return None;
        }
        let mut result = TermProps::from(Rc::new(answer));
        result
            .filters
            .blocked_rules
            .clone_from(&solution.terms[index].filters.blocked_rules);
        result.filters.mark_simplified();
        result.inference.parent = Some(solution.terms[index].inference.id);
        result.inference.requirements.push(task);

        Some(result)
    }

    fn solve_subtask(&self, solution: &mut Solution, task: Rc<Term>) -> Option<Rc<Solution>> {
        match solution.cache.status(&task) {
            Some(TaskStatus::Solved(x)) => return Some(x),
            Some(_) => return None,
            None => {}
        }

        solution.cache.add(task.as_subterm().to_term());

        let subtask = TaskBuilder::default()
            .with_purpose(TermProps::from(task.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                solution
                    .terms
                    .iter()
                    .filter(|x| {
                        !(x.filters.is_purpose() ||
                            x.term.as_subterm().data().is_symbol_name("answer"))
                    })
                    .cloned(),
            )
            .with_level(solution.task.subtask_level + 1)
            .build()
            .expect("Can't build subtask");
        let mut subtask_solver = Solver::new(
            self.rules_engine.clone(),
            self.tracer.clone(),
            self.execution_deadline,
        );

        let mut subtask_solution = Solution::new(subtask);
        subtask_solution.cycles = solution.cycles;
        subtask_solution.cache = solution.cache.clone();

        match subtask_solver.solve_alt(&mut subtask_solution) {
            Ok(_) => {
                let solution = Rc::new(subtask_solution);
                solution
                    .cache
                    .update_status(&task, TaskStatus::Solved(solution.clone()));
                Some(solution)
            }
            Err(SolverError::MaxSubtaskLevelExceed) => {
                trace!(
                    "Can't proof {}: {}",
                    task,
                    SolverError::MaxSubtaskLevelExceed
                );
                solution.cache.remove(&task);
                None
            }
            Err(e) => {
                trace!("Can't proof {}: {}", task, e);
                solution.cache.update_status(&task, TaskStatus::NotSolved);
                None
            }
        }
    }

    fn pick_purpose_term(&self, solution: &mut Solution) -> Option<usize> {
        solution
            .terms
            .iter()
            .filter(|x| !x.filters.is_replaced() && x.filters.is_purpose())
            .min_by_key(|x| x.filters.weight)
            .map(|x| x.inference.id)
    }

    fn prepare_purpose(&mut self, solution: &mut Solution, level: usize) {
        match &solution.purpose {
            Purpose::Find(_) => {}
            Purpose::Proof(_) => {
                while let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].filters.weight > level {
                        return;
                    }

                    if let Some(simplified) = self.transform(solution, index) {
                        solution.terms[index].filters.mark_replaced();
                        // TODO remove unwrap
                        self.add_purpose(solution, simplified).unwrap();
                        continue;
                    } else {
                        solution.terms[index].filters.mark_simplified();
                    }

                    let purpose = TermProps::from(Rc::new(
                        Term::symbol("proof")
                            .with_child(solution.terms[index].term.as_subterm().to_term()),
                    ));
                    let mut added = false;
                    for rule in self.suggest_rules(
                        solution,
                        index,
                        |rule| rule.contains_attribute(&RuleAttr::Equivalence),
                        Some(&purpose),
                    ) {
                        match HypothesisIterator::new(
                            rule.clone(),
                            &solution.terms[index].term,
                            &solution.terms[index].filters,
                            &purpose.term,
                        )
                        .filter(|hypothesis| {
                            !self.purpose_index.contains_key(&hypothesis.resolution)
                        })
                        .filter_map(|hypothesis| {
                            self.hypothesis_proof(Some(index), solution, hypothesis)
                        })
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", solution.terms[index], s);
                                let _ = self.add_purpose(solution, s);
                                added = true;
                            }
                            None => {
                                solution.terms[index].filters.applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        solution.terms[index].filters.weight += 1;
                    }
                }
            }
            Purpose::Transform(_) => {
                while let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].filters.weight > level {
                        return;
                    }

                    let mut added = false;
                    for rule in self.suggest_rules(solution, index, |_| true, None) {
                        match HypothesisIterator::new(
                            rule.clone(),
                            &solution.terms[index].term,
                            &solution.terms[index].filters,
                            &solution.task.purpose.term,
                        )
                        .filter(|h| !self.purpose_index.contains_key(&h.resolution))
                        .filter_map(|h| self.hypothesis_proof(Some(index), solution, h))
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", solution.terms[index], s);
                                if self.add_purpose(solution, s).is_ok() {
                                    solution.terms[index].filters.weight = MAX_LEVEL + 1;
                                    break;
                                }
                                added = true;
                            }
                            None => {
                                solution.terms[index].filters.applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        solution.terms[index].filters.weight += 1;
                    }
                }
            }
        }
    }

    fn is_answer(&self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        let term = &solution.terms[index];
        let term_root = term.term.as_subterm();

        if !solution.purpose.is_transform() &&
            term_root.data().is_symbol_name("answer") &&
            term_root.degree() == 1
        {
            let mut resolution = TermProps::from(Rc::from(
                (*term.term)
                    .clone()
                    .as_subterm_mut()
                    .pop_first_arg()
                    .unwrap(),
            ));
            if let Some(parent) = term.inference.parent {
                resolution = resolution.with_parent(parent);
            }
            return Some(resolution);
        }

        match &solution.purpose {
            Purpose::Find(x) => {
                if term_root.degree() != 2 ||
                    (!term_root.data().is_symbol_name("==") &&
                        !term_root.data().is_symbol_name("in"))
                {
                    return None;
                }

                if term_root.first_arg().unwrap() == x.term.as_subterm() {
                    let is_known = Term::symbol("is")
                        .with_child(term_root.last_arg().unwrap().to_term())
                        .with_child(Term::symbol("known"));

                    if self.proof(solution, &is_known).is_some() {
                        return Some(solution.terms[index].clone());
                    }
                }
                None
            }
            Purpose::Proof(_) => {
                for i in self.purpose_index.values() {
                    if term_root == solution.terms[*i].term.as_subterm() {
                        return Some(term.clone());
                    }
                    if solution.terms[*i].term.as_subterm().truth().is_true() {
                        return Some(solution.terms[*i].clone().without_parents());
                    }
                }
                None
            }
            Purpose::Transform(_) => {
                if let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].filters.weight > MAX_LEVEL {
                        return Some(
                            solution
                                .terms
                                .iter()
                                .rev()
                                .find(|x| x.filters.is_purpose())
                                .unwrap()
                                .clone()
                                .without_parents(),
                        );
                    }
                }
                None
            }
        }
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
        let solution = solver.solve(task).expect("task is not solved");
        assert_eq!(*solution.answer().unwrap(), term_with_vars("x == 1"));
    }

    #[test]
    fn check_answer_proof_test() {
        let task = parse_task("task { purpose proof(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solver = Solver::new(rules, SolutionTracer::default(), usize::MAX);
        let solution = solver.solve(task).expect("task is not solved");
        assert!(solution.answer().is_some());
    }
}
