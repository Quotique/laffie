use std::{collections::HashMap, rc::Rc, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use utils::VecDisplay;

use super::{
    builder::TaskBuilder, cache::TaskStatus, purpose::Purpose, tracing::SolutionTracer, Solution,
    Task,
};
use crate::{
    rule::{Hypothesis, HypothesisIterator, Rule, RuleAttr, RulesEngine, SharedRule},
    term::{SubtermMut, Term, TermProps},
    NormalizationLevel, RuleId,
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

            let level = solution.terms[index].weight;
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
                    solution.terms[index].replaced = true;
                    self.add_main(solution, simplified).unwrap();
                    continue;
                } else {
                    solution.terms[index].simplified = true;
                }
            }

            if let Some(mut hypothesis) = self.is_answer(solution, index) {
                if self.hypothesis_proof(solution, &mut hypothesis).is_some() {
                    trace!("Resolution: {}", hypothesis.resolution);
                    if solution.terms[index] == hypothesis.resolution {
                        trace!("Equivalence");
                        solution.answer = Some(index);
                    } else {
                        // TODO: return index
                        let _ = self.add_main(solution, hypothesis.resolution.clone());
                        solution.answer = Some(
                            solution
                                .terms
                                .iter()
                                .find(|x| x.term == hypothesis.resolution.term)
                                .unwrap()
                                .id,
                        );
                    }
                    trace!(
                        "Solved {}. Answer: {}",
                        solution.task.subtask_level,
                        solution.answer().unwrap()
                    );
                    return Ok(solution.answer().unwrap());
                }
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
                        solution.terms[index].clone(),
                        &solution.task.purpose,
                    )
                    .filter(|hypothesis| !self.main_index.contains_key(&hypothesis.resolution.term))
                    .filter_map(|mut hypothesis| self.hypothesis_proof(solution, &mut hypothesis))
                    .next()
                    {
                        Some(s) => {
                            trace!("{} => {}", solution.terms[index], s);
                            self.add_main(solution, s)?;
                            added = true;
                            break;
                        }
                        None => {
                            solution.terms[index].applied_rules.insert(rule.id);
                        }
                    }
                }
                if !added {
                    solution.terms[index].weight += 1;
                }
            } else {
                solution.terms[index].weight += 1;
            }
        }
    }

    fn add_main(&mut self, solution: &mut Solution, term: TermProps) -> Result<(), SolverError> {
        if self.main_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(solution, term)?;
        self.main_index.insert(key, id);
        Ok(())
    }

    fn add_purpose(
        &mut self,
        solution: &mut Solution,
        mut term: TermProps,
    ) -> Result<(), SolverError> {
        term.is_purpose = true;
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
                .parent
                .map(|id| solution.terms[id].clone())
                .unwrap_or_else(|| TermProps::from(Rc::new(Term::zero()))),
        );

        let id = solution.terms.len();
        term.id = id;
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
            .suggest_rules(&solution.terms[index], purpose);
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
        solution: &mut Solution,
        hypothesis: &mut Hypothesis,
    ) -> Option<TermProps> {
        if let (Some(index), Some(rule)) = (hypothesis.parent_idx(), hypothesis.rule()) {
            trace!(target: "rule_selection", "new hypothesis {hypothesis}, rule {rule}, term: {}", solution.terms[index]);
            self.tracer.on_new_hypothesis(
                solution.terms[index].term.clone(),
                rule.clone(),
                hypothesis,
                solution.current_cycles(),
            );
            solution.profiler.on_new_hypothesis(
                solution.terms[index].term.clone(),
                rule.clone(),
                hypothesis,
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

        if hypothesis.parent_idx().is_some() && hypothesis.rule().is_some() {
            self.tracer
                .on_hypothesis_finish(hypothesis, solution.current_cycles(), proof_res);
            solution.profiler.on_hypothesis_finish(
                hypothesis,
                solution.current_cycles(),
                proof_res,
            );
        }

        if proof_res == hypothesis.requirements.len() {
            hypothesis.resolution.requirements = hypothesis.requirements.clone();
            trace!(target: "rule_selection", "hypothesis {hypothesis} proven, resolution {} applied", hypothesis.resolution);
            Some(hypothesis.resolution.clone())
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
        if solution.terms[index].simplified {
            return None;
        }
        solution.terms[index].simplified = true;

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
            .blocked_rules
            .clone_from(&solution.terms[index].blocked_rules);
        result.simplified = true;
        result.parent = Some(solution.terms[index].id);
        result.requirements.push(task);

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
                        !(x.is_purpose || x.term.as_subterm().data().is_symbol_name("answer"))
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
            .filter(|x| !x.replaced && x.is_purpose)
            .min_by_key(|x| x.weight)
            .map(|x| x.id)
    }

    fn prepare_purpose(&mut self, solution: &mut Solution, level: usize) {
        match &solution.purpose {
            Purpose::Find(_) => {}
            Purpose::Proof(_) => {
                while let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].weight > level {
                        return;
                    }

                    if let Some(simplified) = self.transform(solution, index) {
                        solution.terms[index].replaced = true;
                        // TODO remove unwrap
                        self.add_purpose(solution, simplified).unwrap();
                        continue;
                    } else {
                        solution.terms[index].simplified = true;
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
                            solution.terms[index].clone(),
                            &purpose,
                        )
                        .filter(|hypothesis| {
                            !self.purpose_index.contains_key(&hypothesis.resolution.term)
                        })
                        .filter_map(|mut hypothesis| {
                            self.hypothesis_proof(solution, &mut hypothesis)
                        })
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", solution.terms[index], s);
                                let _ = self.add_purpose(solution, s);
                                added = true;
                            }
                            None => {
                                solution.terms[index].applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        solution.terms[index].weight += 1;
                    }
                }
            }
            Purpose::Transform(_) => {
                while let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].weight > level {
                        return;
                    }

                    let mut added = false;
                    for rule in self.suggest_rules(solution, index, |_| true, None) {
                        match HypothesisIterator::new(
                            rule.clone(),
                            solution.terms[index].clone(),
                            &solution.task.purpose,
                        )
                        .filter(|hypothesis| {
                            !self.purpose_index.contains_key(&hypothesis.resolution.term)
                        })
                        .filter_map(|mut hypothesis| {
                            self.hypothesis_proof(solution, &mut hypothesis)
                        })
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", solution.terms[index], s);
                                if self.add_purpose(solution, s).is_ok() {
                                    solution.terms[index].weight = MAX_LEVEL + 1;
                                    break;
                                }
                                added = true;
                            }
                            None => {
                                solution.terms[index].applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        solution.terms[index].weight += 1;
                    }
                }
            }
        }
    }

    fn is_answer(&self, solution: &mut Solution, index: usize) -> Option<Hypothesis> {
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
            if let Some(parent) = term.parent {
                resolution = resolution.with_parent(parent);
            }
            return Some(Hypothesis {
                requirements: vec![],
                resolution,
                params: Default::default(),
            });
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

                    return Some(Hypothesis {
                        requirements: vec![Rc::new(is_known)],
                        resolution:   term.clone(),
                        params:       Default::default(),
                    });
                }
                None
            }
            Purpose::Proof(_) => {
                for i in self.purpose_index.values() {
                    if term_root == solution.terms[*i].term.as_subterm() {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   term.clone(),
                            params:       Default::default(),
                        });
                    }
                    if solution.terms[*i].term.as_subterm().truth().is_true() {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   solution.terms[*i].clone().without_parents(),
                            params:       Default::default(),
                        });
                    }
                }
                None
            }
            Purpose::Transform(_) => {
                if let Some(index) = self.pick_purpose_term(solution) {
                    if solution.terms[index].weight > MAX_LEVEL {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   solution
                                .terms
                                .iter()
                                .rev()
                                .find(|x| x.is_purpose)
                                .unwrap()
                                .clone()
                                .without_parents(),
                            params:       Default::default(),
                        });
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
