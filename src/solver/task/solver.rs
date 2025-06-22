use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::Display;
use itertools::Itertools;

use utils::VecDisplay;

use super::{
    builder::TaskBuilder,
    cache::{TaskStatus, TasksCache},
    purpose::Purpose,
    tracing::SolutionTracer,
    Task,
};
use crate::{
    rule::{Hypothesis, HypothesisIterator, Rule, RuleAttr, RulesEngine, SharedRule},
    symbol::{normalize, swap_node, SymbolNodeMut},
    term::{Term, TermProps},
    NormalizationLevel, RuleId,
};

pub const MAX_SUBTASK_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;
pub const STACK_SIZE: usize = 2048;
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

pub struct Solver {
    pub task: Task,

    pub terms:     Vec<TermProps>,
    main_index:    HashMap<Rc<Term>, usize>,
    purpose_index: HashMap<Rc<Term>, usize>,

    pub cache: Arc<TasksCache>,

    local_rules: Vec<SharedRule>,
    pub purpose: Purpose,

    pub answer: Option<usize>,

    pub cycles:         Rc<RefCell<usize>>,
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
        task: Task,
        rules: Arc<RulesEngine>,
        tracer: SolutionTracer,
        execution_deadline: usize,
        cache: Arc<TasksCache>,
    ) -> Solver {
        let purpose = Purpose::try_from((*task.purpose.term).clone()).unwrap();

        let (root, mut childs) = (*task.purpose.term).clone().destruct();

        let terms = if root.data().is_symbol_name("find") ||
            root.data().is_symbol_name("proof") ||
            root.data().is_symbol_name("transform")
        {
            if childs.degree() != 1 {
                panic!("wrong arg count");
            }
            TermProps::from(Rc::new(Term::from(childs.pop_front().unwrap())))
        } else {
            panic!("unexpected word {}", root);
        };

        let conditions = task.conditions.clone();

        let mut result = Solver {
            // TODO: init values
            terms: Default::default(),
            main_index: Default::default(),
            purpose_index: Default::default(),

            cache,

            local_rules: vec![],
            purpose,
            answer: None,

            cycles: RefCell::new(0).into(),
            task,
            rules_engine: rules.clone(),
            tracer,
            execution_deadline,
        };
        for i in conditions.into_iter() {
            let _ = result.add_main(i);
        }
        let _ = result.add_purpose(terms);
        result
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.main_index.clear();
        self.purpose_index.clear();
        self.terms.clear();

        let conditions = self.task.conditions.clone();
        for i in conditions.into_iter() {
            let _ = self.add_main(i);
        }
        let (_, mut purpose) = (*self.task.purpose.term).clone().destruct();
        let _ = self.add_purpose(TermProps::from(Rc::new(Term::from(
            purpose.pop_front().unwrap(),
        ))));

        self.local_rules.clear();
        *self.cycles.borrow_mut() = 0;
        self.answer = None;
    }

    pub fn replace_rules(&mut self, rules: Arc<RulesEngine>) {
        self.rules_engine = rules;
    }

    pub fn answer(&self) -> Option<Rc<Term>> {
        self.answer.map(|i| self.terms[i].term.clone())
    }

    pub fn validate_answer(&self) -> bool {
        if self.task.possible_answers.is_empty() {
            return true;
        }

        if let Some(answer) = self.answer() {
            if self
                .task
                .possible_answers
                .iter()
                .any(|x| x == answer.as_ref())
            {
                return true;
            }
            // TODO: есть проблема с неправильным преобразованием дерева, что приводит к
            // некорректному прямому сравнению дерева.
            return self
                .task
                .possible_answers
                .iter()
                .any(|x| x.to_string() == answer.to_string());
        }
        false
    }

    pub fn solve(&mut self) -> Result<Rc<Term>, SolverError> {
        trace!(target: "subtask", "Subtask: {}, {}", self.purpose, VecDisplay(&self.task.conditions));
        if self.task.subtask_level > MAX_SUBTASK_LEVEL {
            return Err(SolverError::MaxSubtaskLevelExceed);
        }

        self.tracer
            .on_subtask_start(&self.task, self.current_cycles());

        let result = self.solver_loop();

        self.tracer.clone().on_subtask_end(self);
        result
    }

    #[inline]
    pub fn current_cycles(&self) -> usize {
        *self.cycles.as_ref().borrow()
    }

    fn solver_loop(&mut self) -> Result<Rc<Term>, SolverError> {
        loop {
            *self.cycles.as_ref().borrow_mut() += 1;
            if self.current_cycles() > self.execution_deadline {
                return Err(SolverError::ExecutionDeadline);
            }

            let index = self
                .terms
                .iter()
                .filter(|x| !(x.replaced || x.is_purpose))
                .min_by_key(|x| x.weight)
                .map(|x| x.id)
                .ok_or(SolverError::NoConditions)?;
            self.tracer.on_term_focus(&self.terms[index]);

            let level = self.terms[index].weight;
            trace!(
                target: "subtask",
                "[{}]({}) Level: {} -> {}",
                self.task.subtask_level,
                self.current_cycles(),
                level, self.terms[index]
            );
            if level > MAX_LEVEL {
                return Err(SolverError::NoSolutionsFound);
            }
            self.prepare_purpose(level);

            if !self.purpose.is_transform() {
                if let Some(simplified) = self.transform(index) {
                    self.terms[index].replaced = true;
                    self.add_main(simplified).unwrap();
                    continue;
                } else {
                    self.terms[index].simplified = true;
                }
            }

            if let Some(mut hypothesis) = self.is_answer(&self.terms[index]) {
                if self.hypothesis_proof(&mut hypothesis).is_some() {
                    trace!("Resolution: {}", hypothesis.resolution);
                    if self.terms[index] == hypothesis.resolution {
                        trace!("Equivalence");
                        self.answer = Some(index);
                    } else {
                        // TODO: return index
                        let _ = self.add_main(hypothesis.resolution.clone());
                        self.answer = Some(
                            self.terms
                                .iter()
                                .find(|x| x.term == hypothesis.resolution.term)
                                .unwrap()
                                .id,
                        );
                    }
                    trace!(
                        "Solved {}. Answer: {}",
                        self.task.subtask_level,
                        self.answer().unwrap()
                    );
                    return Ok(self.answer().unwrap());
                }
            }
            if let Some(r) = self.terms[index].rule(
                RuleId::new(0x80_00_00_00_00_00_00_00, self.local_rules.len() as u64 + 1),
                (level + 1) as u64,
            ) {
                // TODO: check dups
                self.local_rules.push(r);
            }

            if !self.purpose.is_transform() {
                let mut added = false;
                for rule in self.suggest_rules(index, |_| true, None) {
                    match HypothesisIterator::new(
                        rule.clone(),
                        self.terms[index].clone(),
                        &self.task.purpose,
                    )
                    .filter(|hypothesis| !self.main_index.contains_key(&hypothesis.resolution.term))
                    .filter_map(|mut hypothesis| self.hypothesis_proof(&mut hypothesis))
                    .next()
                    {
                        Some(s) => {
                            trace!("{} => {}", self.terms[index], s);
                            self.add_main(s)?;
                            added = true;
                            break;
                        }
                        None => {
                            self.terms[index].applied_rules.insert(rule.id);
                        }
                    }
                }
                if !added {
                    self.terms[index].weight += 1;
                }
            } else {
                self.terms[index].weight += 1;
            }
        }
    }

    fn add_main(&mut self, term: TermProps) -> Result<(), SolverError> {
        if self.main_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(term)?;
        self.main_index.insert(key, id);
        Ok(())
    }

    fn add_purpose(&mut self, mut term: TermProps) -> Result<(), SolverError> {
        term.is_purpose = true;
        if self.purpose_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(term)?;
        self.purpose_index.insert(key, id);
        Ok(())
    }

    fn add_term(&mut self, mut term: TermProps) -> Result<usize, SolverError> {
        self.tracer.on_new_term(
            &term,
            &term
                .parent
                .map(|id| self.terms[id].clone())
                .unwrap_or_else(|| TermProps::from(Rc::new(Term::zero()))),
        );

        let id = self.terms.len();
        term.id = id;
        if self.terms.len() + 1 > STACK_SIZE {
            return Err(SolverError::StackOverflow);
        }
        self.terms.push(term);
        Ok(id)
    }

    fn suggest_rules(
        &self,
        index: usize,
        filter: impl Fn(&Rule) -> bool,
        purpose: Option<&TermProps>,
    ) -> Vec<SharedRule> {
        let purpose = purpose.unwrap_or(&self.task.purpose);
        let engine_rules = self.rules_engine.suggest_rules(&self.terms[index], purpose);
        let suggested_rules: Vec<_> = engine_rules
            .into_iter()
            .chain(self.local_rules.iter().unique().cloned())
            .filter(|rule| filter(rule))
            .collect();
        trace!(target: "rule_selection",
               "purpose: {purpose}, term: {}, suggested_rules: {}",
               self.terms[index],
               VecDisplay(&suggested_rules)
        );
        suggested_rules
    }

    fn hypothesis_proof(&self, hypothesis: &mut Hypothesis) -> Option<TermProps> {
        if let (Some(index), Some(rule)) = (hypothesis.parent_idx(), hypothesis.rule()) {
            trace!(target: "rule_selection", "new hypothesis {hypothesis}, rule {rule}, term: {}", self.terms[index]);
            self.tracer.on_new_hypothesis(
                self.terms[index].term.clone(),
                rule.clone(),
                hypothesis,
                *self.cycles.borrow(),
            );
        }

        let mut proof_res = 0;
        for (num, req) in hypothesis.requirements.iter().enumerate() {
            if self.proof(req).is_none() {
                trace!(target: "rule_selection", "term {} rejected, requirement not proven {req}", hypothesis.resolution);
                break;
            }
            proof_res = num + 1;
        }

        if hypothesis.parent_idx().is_some() && hypothesis.rule().is_some() {
            self.tracer
                .on_hypothesis_finish(hypothesis, *self.cycles.borrow(), proof_res);
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
    fn proof(&self, term: &Term) -> Option<Rc<Term>> {
        let mut clone = term.root().deep_clone();
        is_replace(&mut clone.root_mut());
        // TODO: normalization level
        normalize(&mut clone.root_mut(), NormalizationLevel::max());

        let proof_purpose = Rc::new(Term::func("proof").with_child(clone));

        if term.root().check_truth().is_true() {
            return Some(proof_purpose);
        }
        self.solve_subtask(proof_purpose.clone())
            .map(|_| proof_purpose)
    }

    fn transform(&mut self, index: usize) -> Option<TermProps> {
        if self.terms[index].simplified {
            return None;
        }
        self.terms[index].simplified = true;

        let (answer_wrap, to_transform) = if self.terms[index]
            .term
            .root()
            .data()
            .is_symbol_name("answer")
        {
            (
                true,
                self.terms[index].term.root().front().unwrap().deep_clone(),
            )
        } else {
            (false, self.terms[index].term.root().deep_clone())
        };

        let task = Rc::new(Term::func("transform").with_child(to_transform));

        let subtask_solver = self.solve_subtask(task.clone())?;

        let mut answer = subtask_solver.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = Term::func("answer");
            swap_node(&mut answer.root_mut(), &mut tmp.root_mut());
            answer.root_mut().push_back(tmp);
        }

        if *self.terms[index].term == answer {
            return None;
        }
        let mut result = TermProps::from(Rc::new(answer));
        result
            .blocked_rules
            .clone_from(&self.terms[index].blocked_rules);
        result.simplified = true;
        result.parent = Some(self.terms[index].id);
        result.requirements.push(task);

        Some(result)
    }

    fn solve_subtask(&self, task: Rc<Term>) -> Option<Rc<Solver>> {
        match self.cache.status(&task) {
            Some(TaskStatus::Solved(x)) => return Some(x),
            Some(_) => return None,
            None => {}
        }

        self.cache.add(task.root().deep_clone());

        let subtask = TaskBuilder::default()
            .with_purpose(TermProps::from(task.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                self.terms
                    .iter()
                    .filter(|x| !(x.is_purpose || x.term.root().data().is_symbol_name("answer")))
                    .cloned(),
            )
            .with_level(self.task.subtask_level + 1)
            .build()
            .expect("Can't build subtask");
        let mut subtask_solver = Solver::new(
            subtask,
            self.rules_engine.clone(),
            self.tracer.clone(),
            self.execution_deadline,
            self.cache.clone(),
        );
        subtask_solver.cycles = self.cycles.clone();

        match subtask_solver.solve() {
            Ok(_) => {
                let solution = Rc::new(subtask_solver);
                self.cache
                    .update_status(&task, TaskStatus::Solved(solution.clone()));
                Some(solution)
            }
            Err(SolverError::MaxSubtaskLevelExceed) => {
                trace!(
                    "Can't proof {}: {}",
                    task,
                    SolverError::MaxSubtaskLevelExceed
                );
                self.cache.remove(&task);
                None
            }
            Err(e) => {
                trace!("Can't proof {}: {}", task, e);
                self.cache.update_status(&task, TaskStatus::NotSolved);
                None
            }
        }
    }

    fn pick_purpose_term(&self) -> Option<usize> {
        self.terms
            .iter()
            .filter(|x| !x.replaced && x.is_purpose)
            .min_by_key(|x| x.weight)
            .map(|x| x.id)
    }

    fn prepare_purpose(&mut self, level: usize) {
        match &self.purpose {
            Purpose::Find(_) => {}
            Purpose::Proof(_) => {
                while let Some(index) = self.pick_purpose_term() {
                    if self.terms[index].weight > level {
                        return;
                    }

                    if let Some(simplified) = self.transform(index) {
                        self.terms[index].replaced = true;
                        // TODO remove unwrap
                        self.add_purpose(simplified).unwrap();
                        continue;
                    } else {
                        self.terms[index].simplified = true;
                    }

                    let purpose = TermProps::from(Rc::new(
                        Term::func("proof").with_child(self.terms[index].term.root().deep_clone()),
                    ));
                    let mut added = false;
                    for rule in self.suggest_rules(
                        index,
                        |rule| rule.contains_attribute(&RuleAttr::Equivalence),
                        Some(&purpose),
                    ) {
                        match HypothesisIterator::new(
                            rule.clone(),
                            self.terms[index].clone(),
                            &purpose,
                        )
                        .filter(|hypothesis| {
                            !self.purpose_index.contains_key(&hypothesis.resolution.term)
                        })
                        .filter_map(|mut hypothesis| self.hypothesis_proof(&mut hypothesis))
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", self.terms[index], s);
                                let _ = self.add_purpose(s);
                                added = true;
                            }
                            None => {
                                self.terms[index].applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        self.terms[index].weight += 1;
                    }
                }
            }
            Purpose::Transform(_) => {
                while let Some(index) = self.pick_purpose_term() {
                    if self.terms[index].weight > level {
                        return;
                    }

                    let mut added = false;
                    for rule in self.suggest_rules(index, |_| true, None) {
                        match HypothesisIterator::new(
                            rule.clone(),
                            self.terms[index].clone(),
                            &self.task.purpose,
                        )
                        .filter(|hypothesis| {
                            !self.purpose_index.contains_key(&hypothesis.resolution.term)
                        })
                        .filter_map(|mut hypothesis| self.hypothesis_proof(&mut hypothesis))
                        .next()
                        {
                            Some(s) => {
                                trace!("{} => {}", self.terms[index], s);
                                if self.add_purpose(s).is_ok() {
                                    self.terms[index].weight = MAX_LEVEL + 1;
                                    break;
                                }
                                added = true;
                            }
                            None => {
                                self.terms[index].applied_rules.insert(rule.id);
                            }
                        }
                    }
                    if !added {
                        self.terms[index].weight += 1;
                    }
                }
            }
        }
    }

    fn is_answer(&self, term: &TermProps) -> Option<Hypothesis> {
        let term_root = term.term.root();

        if !self.purpose.is_transform() &&
            term_root.data().is_symbol_name("answer") &&
            term_root.degree() == 1
        {
            let mut resolution = TermProps::from(Rc::from(
                (*term.term).clone().root_mut().pop_front().unwrap(),
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

        match &self.purpose {
            Purpose::Find(x) => {
                if term_root.degree() != 2 ||
                    (!term_root.data().is_symbol_name("==") &&
                        !term_root.data().is_symbol_name("in"))
                {
                    return None;
                }

                if term_root.front().unwrap() == x.term.root() {
                    let is_known = Term::func("is")
                        .with_child(term_root.back().unwrap().deep_clone())
                        .with_child(Term::func("known"));

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
                    if term_root == self.terms[*i].term.root() {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   term.clone(),
                            params:       Default::default(),
                        });
                    }
                    if self.terms[*i].term.root().check_truth().is_true() {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   self.terms[*i].clone().without_parents(),
                            params:       Default::default(),
                        });
                    }
                }
                None
            }
            Purpose::Transform(_) => {
                if let Some(index) = self.pick_purpose_term() {
                    if self.terms[index].weight > MAX_LEVEL {
                        return Some(Hypothesis {
                            requirements: vec![],
                            resolution:   self
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

fn is_replace(root: &mut SymbolNodeMut) {
    if !root.data().is_symbol_name("is") || root.degree() != 2 {
        return;
    }

    match root
        .back()
        .unwrap()
        .data()
        .func_symbol()
        .map(|x| x.name.clone())
    {
        Some(name) if name == "true" => {
            let mut child = root.pop_front().unwrap();
            swap_node(root, &mut child.root_mut());
        }
        Some(name) if name == "false" => {
            let child = root.pop_front().unwrap();
            swap_node(root, &mut Term::func("!").with_child(child).root_mut());
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
        let mut solution = Solver::new(
            task,
            rules,
            SolutionTracer::default(),
            usize::MAX,
            Default::default(),
        );
        assert!(solution.solve().is_ok());
        assert_eq!(*solution.answer().unwrap(), term_with_vars("x == 1"));
    }

    #[test]
    fn check_answer_proof_test() {
        let task = parse_task("task { purpose proof(x > 0); x == 2; }");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solver::new(
            task,
            rules,
            SolutionTracer::default(),
            usize::MAX,
            Default::default(),
        );
        assert!(solution.solve().is_ok());
        assert!(solution.answer().is_some());
    }
}
