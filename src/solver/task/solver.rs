use std::{collections::HashMap, rc::Rc, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use utils::VecDisplay;

use super::{
    props::TermInference, Cause, Purpose, Solution, SolutionTracer, Task, TaskBuilder, TermProps,
};
use crate::{
    rule::{Hypothesis, HypothesisIterator, RuleAttr, RuleId, RulesEngine, SharedRule},
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

    pub fn solve(&mut self, task: Task) -> Result<Rc<Solution>, (Rc<Solution>, SolverError)> {
        let mut solution = Solution::new(task);
        match self.solve_alt(&mut solution) {
            Ok(_) => Ok(Rc::new(solution)),
            Err(err) => Err((Rc::new(solution), err)),
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

        let result = self.solver_loop(solution);

        self.tracer.clone().on_subtask_end(solution);
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
                trace!("Resolution: {term}");
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
                for rule in self.suggest_rules(&solution.terms[index], &solution.task.purpose) {
                    match HypothesisIterator::new(
                        rule.clone(),
                        &solution.terms[index].term,
                        &solution.terms[index].filters,
                        &solution.task.purpose.term,
                    )
                    .filter(|h| !self.main_index.contains_key(&h.resolution))
                    .filter_map(|h| self.hypothesis_proof(index, solution, h))
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

    fn add_term(&self, solution: &mut Solution, mut term: TermProps) -> Result<usize, SolverError> {
        self.tracer.on_new_term(
            &term,
            &term
                .inference
                .as_ref()
                .map(|i| solution.terms[i.parent].clone())
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

    fn suggest_rules(&self, term: &TermProps, purpose: &TermProps) -> Vec<SharedRule> {
        let engine_rules = self
            .rules_engine
            .suggest_rules(&term.filters, &purpose.term);
        let suggested_rules: Vec<_> = engine_rules
            .into_iter()
            .chain(self.local_rules.iter().unique().cloned())
            .collect();
        trace!(target: "rule_selection",
               "purpose: {purpose}, term: {term}, suggested_rules: {}",
               VecDisplay(&suggested_rules)
        );
        suggested_rules
    }

    fn hypothesis_proof(
        &self,
        parent_idx: usize,
        solution: &mut Solution,
        hypothesis: Hypothesis,
    ) -> Option<TermProps> {
        trace!(
            target: "rule_selection",
            "new hypothesis {hypothesis}, rule {}, term: {}",
            hypothesis.rule, solution.terms[parent_idx]
        );

        self.tracer.on_new_hypothesis(
            solution.terms[parent_idx].term.clone(),
            hypothesis.rule.clone(),
            &hypothesis,
            solution.current_cycles(),
        );

        let mut proof_res = 0;
        let mut solved = vec![];
        let mut requirements_iter = hypothesis.requirements.clone().into_iter();
        for req in requirements_iter.by_ref() {
            let (task, sol) = self.proof(solution, req);
            solved.push((task, Some(sol)));
            let last = solved.last().unwrap();
            if last.1.as_ref().unwrap().answer().is_none() {
                trace!(
                    target: "rule_selection",
                    "term {} rejected, requirement not proven {}",
                    hypothesis.resolution,
                    last.0
                );
                break;
            }
            proof_res += 1;
        }
        for req in requirements_iter {
            solved.push((Rc::new(req), None));
        }

        self.tracer
            .on_hypothesis_finish(&hypothesis, solution.current_cycles(), proof_res);

        if solved
            .iter()
            .all(|x| x.1.as_ref().map(|x| x.answer().is_some()).unwrap_or(false))
        {
            trace!(
                target: "rule_selection",
                "hypothesis {hypothesis} proven, resolution {} applied",
                hypothesis.resolution
            );

            let mut props = TermProps::from(Rc::new(hypothesis.resolution));
            props.inference = Some(TermInference {
                rule:         Cause::Rule(hypothesis.rule),
                parent:       parent_idx,
                requirements: solved,
            });
            props.filters.blocked_rules = hypothesis.blocked_rules.into_iter().collect();

            Some(props)
        } else {
            None
        }
    }

    fn proof(&self, solution: &mut Solution, mut term: Term) -> (Rc<Term>, Rc<Solution>) {
        is_replace(&mut term.as_subterm_mut());
        let term = term.normalize(NormalizationLevel::max());
        let proof_purpose = Rc::new(Term::symbol("proof").with_child(term));
        // TODO: fast check truth
        let subtask_solution = self.solve_subtask(solution, proof_purpose.clone());
        (proof_purpose, subtask_solution)
    }

    fn transform(&mut self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        let term = &mut solution.terms[index];
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
        let task = Rc::new(Term::symbol("transform").with_child(to_transform));
        let subtask_solution = self.solve_subtask(solution, task.clone());

        let mut answer = subtask_solution.answer()?.as_ref().clone();
        if use_answer {
            answer = Term::symbol("answer").with_child(answer);
        }

        if *solution.terms[index].term == answer {
            return None;
        }
        let mut result = TermProps::from(answer);
        result.inference = Some(TermInference {
            rule:         Cause::Transform,
            parent:       index,
            requirements: vec![(task, Some(subtask_solution))],
        });
        result
            .filters
            .blocked_rules
            .clone_from(&solution.terms[index].filters.blocked_rules);
        result.filters.mark_simplified();

        Some(result)
    }

    fn subtask(solution: &mut Solution, task: Rc<Term>) -> Task {
        TaskBuilder::default()
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
            .expect("Can't build subtask")
    }

    fn solve_subtask(&self, solution: &mut Solution, task: Rc<Term>) -> Rc<Solution> {
        if let Some(x) = solution.cache.status(&task).flatten() {
            return x.clone();
        }

        if !solution.cache.add(task.as_subterm().to_term()) {
            unimplemented!("subtask recursion");
            // TODO: recursion
        }
        let subtask = Self::subtask(solution, task.clone());

        let mut subtask_solver = Solver::new(
            self.rules_engine.clone(),
            self.tracer.clone(),
            self.execution_deadline,
        );

        let mut subtask_solution = Solution::new(subtask);
        subtask_solution.cycles = solution.cycles;
        subtask_solution.cache = solution.cache.clone();

        let result = subtask_solver.solve_alt(&mut subtask_solution);
        let subtask_solution = Rc::new(subtask_solution);

        solution
            .cache
            .update_status(&task, subtask_solution.clone());
        match result {
            Err(SolverError::MaxSubtaskLevelExceed) => {
                trace!("Can't proof {task}: MaxSubtaskLevelExceed");
                solution.cache.remove(&task);
            }
            Err(e) => {
                trace!("Can't proof {task}: {e}");
            }
            Ok(_) => {}
        }
        subtask_solution
    }

    fn pick_purpose_term(&self, solution: &mut Solution) -> Option<usize> {
        solution
            .terms
            .iter()
            .filter(|x| !x.filters.is_replaced() && x.filters.is_purpose())
            .min_by_key(|x| x.filters.weight)
            .map(|x| x.id)
    }

    fn prepare_purpose(&mut self, solution: &mut Solution, level: usize) {
        match &solution.purpose {
            Purpose::Find(_) => {}
            Purpose::Proof(_) => self.prepare_proof(solution, level),
            Purpose::Transform(_) => self.prepare_transform(solution, level),
        }
    }

    fn prepare_proof(&mut self, solution: &mut Solution, level: usize) {
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

            let purpose = TermProps::from(
                Term::symbol("proof").with_child(solution.terms[index].term.as_ref().clone()),
            );
            let mut added = false;
            for rule in self
                .suggest_rules(&solution.terms[index], &purpose)
                .into_iter()
                .filter(|rule| rule.contains_attribute(&RuleAttr::Equivalence))
            {
                match HypothesisIterator::new(
                    rule.clone(),
                    &solution.terms[index].term,
                    &solution.terms[index].filters,
                    &purpose.term,
                )
                .filter(|h| !self.purpose_index.contains_key(&h.resolution))
                .filter_map(|hypothesis| self.hypothesis_proof(index, solution, hypothesis))
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

    fn prepare_transform(&mut self, solution: &mut Solution, level: usize) {
        while let Some(index) = self.pick_purpose_term(solution) {
            if solution.terms[index].filters.weight > level {
                return;
            }

            let mut added = false;
            for rule in self.suggest_rules(&solution.terms[index], &solution.task.purpose) {
                match HypothesisIterator::new(
                    rule.clone(),
                    &solution.terms[index].term,
                    &solution.terms[index].filters,
                    &solution.task.purpose.term,
                )
                .filter(|h| !self.purpose_index.contains_key(&h.resolution))
                .filter_map(|h| self.hypothesis_proof(index, solution, h))
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

    fn is_answer(&self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        if let Some(term) = self.check_answer_term(solution, index) {
            return Some(term);
        }

        match solution.purpose.clone() {
            Purpose::Find(x) => self.check_find_answer(solution, index, x.term.as_ref()),
            Purpose::Proof(_) => self.check_proof_answer(solution, index),
            Purpose::Transform(_) => self.check_transform_answer(solution),
        }
    }

    fn check_answer_term(&self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        let term = &solution.terms[index];
        let term_root = term.term.as_subterm();

        if solution.purpose.is_transform() {
            return None;
        }
        if term_root.data().is_symbol_name("answer") && term_root.degree() == 1 {
            let mut clone = (*term.term).clone();
            let mut resolution = TermProps::from(clone.as_subterm_mut().pop_first_arg().unwrap());
            if let Some(inference) = &term.inference {
                resolution.inference = Some(inference.clone());
            }
            return Some(resolution);
        }
        None
    }

    fn check_proof_answer(&self, solution: &mut Solution, index: usize) -> Option<TermProps> {
        let term = &solution.terms[index];

        for i in self.purpose_index.values() {
            if term.term == solution.terms[*i].term {
                return Some(term.clone());
            }
            if solution.terms[*i].term.as_subterm().truth().is_true() {
                let mut res = solution.terms[*i].clone();
                res.inference.take();
                return Some(res);
            }
        }
        None
    }

    fn check_transform_answer(&self, solution: &mut Solution) -> Option<TermProps> {
        let index = self.pick_purpose_term(solution)?;
        if solution.terms[index].filters.weight > MAX_LEVEL {
            let mut terms_iter = solution.terms.iter().rev();
            let mut res = terms_iter.find(|x| x.filters.is_purpose()).unwrap().clone();
            res.inference.take();
            return Some(res);
        }
        None
    }

    fn check_find_answer(
        &self,
        solution: &mut Solution,
        index: usize,
        find: &Term,
    ) -> Option<TermProps> {
        let term = &solution.terms[index];
        let term_root = term.term.as_subterm();

        if term_root.degree() != 2 {
            return None;
        }

        if !term_root.data().is_symbol_name("==") && !term_root.data().is_symbol_name("in") {
            return None;
        }

        if term_root.first_arg().unwrap() == find.as_subterm() {
            let is_known = Term::symbol("is")
                .with_child(term_root.last_arg().unwrap().to_term())
                .with_child(Term::symbol("known"));

            if self.proof(solution, is_known).1.answer().is_some() {
                return Some(solution.terms[index].clone());
            }
        }
        None
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
