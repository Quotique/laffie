use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use bincode::{Decode, Encode};
use derive_more::Display;
use trees::tr;

use utils::VecDisplay;

use super::{
    builder::TaskBuilder,
    cache::{TaskStatus, TasksCache},
    purpose::Purpose,
    tracing::{SolutionTracer, Tracer},
    Task,
};
use crate::{
    rule::{Rule, RuleAttr, RulesEngine, SharedRule, Suppose},
    symbol::{normalize, Symbol, SymbolNode},
    term::{swap_node, NodeMapping, Term, TermProps},
    NormalizationLevel, RuleId,
};

pub const MAX_SUBTASK_LEVEL: usize = 10;
pub const MAX_LEVEL: usize = 20;
pub const STACK_SIZE: usize = 2048;
pub const EXECUTION_DEADLINE_DEFAULT: usize = 100_000;

pub struct Solution {
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
    pub dumper:         SolutionTracer,
}

#[derive(Debug, Display, Clone, Copy, Encode, Decode)]
pub enum SolutionError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
    ExecutionDeadline,
}

impl Solution {
    pub fn new(
        task: Task,
        rules: Arc<RulesEngine>,
        dumper: SolutionTracer,
        execution_deadline: usize,
        cache: Arc<TasksCache>,
    ) -> Solution {
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

        let mut result = Solution {
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
            dumper,
            execution_deadline,
        };
        for i in conditions.into_iter() {
            let _ = result.add_main(i);
        }
        let _ = result.add_purpose(terms);
        result
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

    pub fn solve(&mut self) -> Result<Rc<Term>, SolutionError> {
        self.dumper
            .on_subtask_start(&self.task, self.current_cycles());
        trace!(target: "subtask", "Subtask: {}, {}", self.purpose, VecDisplay(&self.task.conditions));
        if self.task.subtask_level > MAX_SUBTASK_LEVEL {
            return Err(SolutionError::MaxSubtaskLevelExceed);
        }

        let result = self.solution_loop();

        self.dumper.clone().on_subtask_end(self);
        result
    }

    #[inline]
    pub fn current_cycles(&self) -> usize {
        *self.cycles.as_ref().borrow()
    }

    fn solution_loop(&mut self) -> Result<Rc<Term>, SolutionError> {
        loop {
            *self.cycles.as_ref().borrow_mut() += 1;
            if self.current_cycles() > self.execution_deadline {
                return Err(SolutionError::ExecutionDeadline);
            }

            let index = self
                .terms
                .iter()
                .filter(|x| !(x.replaced || x.is_purpose))
                .min_by_key(|x| x.weight)
                .map(|x| x.id)
                .ok_or(SolutionError::NoConditions)?;
            self.dumper.on_term_focus(&self.terms[index]);

            let level = self.terms[index].weight;
            trace!(
                target: "subtask",
                "[{}] Level: {} -> {}",
                self.task.subtask_level,
                level, self.terms[index]
            );
            if level > MAX_LEVEL {
                return Err(SolutionError::NoSolutionsFound);
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

            if let Some(suppose) = self.is_answer(&self.terms[index]) {
                if self.suppose_proof(&suppose).is_some() {
                    trace!("Resolution: {}", suppose.resolution);
                    if self.terms[index] == suppose.resolution {
                        trace!("Equivalence");
                        self.answer = Some(index);
                    } else {
                        // TODO: return index
                        let _ = self.add_main(suppose.resolution.clone());
                        self.answer = Some(
                            self.terms
                                .iter()
                                .find(|x| x.term == suppose.resolution.term)
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
                let terms = self.next_term(index);
                if terms.is_empty() {
                    self.terms[index].weight += 1;
                }
                for s in terms {
                    trace!("{} => {}", self.terms[index], s);
                    self.add_main(s)?;
                }
            } else {
                self.terms[index].weight += 1;
            }
        }
    }

    fn add_main(&mut self, term: TermProps) -> Result<(), SolutionError> {
        if self.main_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(term)?;
        self.main_index.insert(key, id);
        Ok(())
    }

    fn add_purpose(&mut self, mut term: TermProps) -> Result<(), SolutionError> {
        term.is_purpose = true;
        if self.purpose_index.contains_key(&term.term) {
            return Ok(());
        }
        let key = term.term.clone();
        let id = self.add_term(term)?;
        self.purpose_index.insert(key, id);
        Ok(())
    }

    fn add_term(&mut self, mut term: TermProps) -> Result<usize, SolutionError> {
        self.dumper.on_new_term(
            &term,
            &term
                .parent
                .map(|id| self.terms[id].clone())
                .unwrap_or_else(|| TermProps::from(Rc::new(Term::zero()))),
        );

        let id = self.terms.len();
        term.id = id;
        if self.terms.len() + 1 > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }
        self.terms.push(term);
        Ok(id)
    }

    fn next_term(&mut self, index: usize) -> Vec<TermProps> {
        self.next_term_with_filter(index, |_| true)
    }

    fn next_term_with_filter(
        &mut self,
        index: usize,
        filter: impl Fn(&Rule) -> bool,
    ) -> Vec<TermProps> {
        let engine_rules = self
            .rules_engine
            .suggest_rules(&self.terms[index], &self.task.purpose);
        let suggested_rules: Vec<_> = engine_rules
            .into_iter()
            .chain(self.local_rules.iter().cloned())
            .inspect(|rule| trace!(target: "rule_selection", "Rule: {}", rule))
            .filter(|rule| filter(rule))
            .collect();

        for rule in suggested_rules {
            self.dumper.on_rule_selection(rule.clone());
            let supposes = match rule.apply(&mut self.terms[index], &self.task.purpose) {
                Ok(x) => x,
                Err(e) => {
                    trace!(target: "rule_selection", "Rule not applied: {:?}", e);
                    continue;
                }
            };

            let mut dumper = self.dumper.clone();
            let res: Vec<_> = supposes
                .into_iter()
                .filter(|suppose| !self.main_index.contains_key(&suppose.resolution.term))
                .inspect({
                    let mut dumper = self.dumper.clone();
                    let rule = rule.clone();
                    move |suppose| {
                        trace!(target: "rule_selection", "Suppose: {}", suppose);
                        dumper.on_new_suppose(rule.clone(), suppose)
                    }
                })
                .filter_map(|mut suppose| {
                    if let Some(proofed) = self.suppose_proof(&suppose) {
                        suppose.resolution.requirements = proofed;
                        dumper.on_suppose_finish(&suppose, true);
                        trace!(target: "rule_selection", "Suppose: proofed, resolution applied");
                        Some(suppose)
                    } else {
                        dumper.on_suppose_finish(&suppose, false);
                        None
                    }
                })
                .map(|mut suppose| {
                    suppose.resolution.rule = Some(rule.clone());
                    suppose.resolution
                })
                .collect();
            if !res.is_empty() {
                return res;
            }
        }
        vec![]
    }

    fn suppose_proof(&self, suppose: &Suppose) -> Option<Vec<Rc<Term>>> {
        let mut result = vec![];
        for req in suppose.requirements.iter() {
            if let Some(proofed) = self.proof(req) {
                result.push(proofed);
            } else {
                trace!(target: "rule_selection", "Can't proof: {}", req);
                return None;
            }
        }
        Some(result)
    }

    // Returns proof purpose (is a key for tasks cache)
    fn proof(&self, term: &Term) -> Option<Rc<Term>> {
        let mut clone = term.root().deep_clone();
        is_replace(&mut clone.root_mut());
        // TODO: normalization level
        normalize(&mut clone.root_mut(), NormalizationLevel::max());

        let proof_purpose = Rc::new(Term::from(tr(Symbol::with_func_symbol("proof")) / clone));

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

        let task = Rc::new(Term::from(
            tr(Symbol::with_func_symbol("transform")) / to_transform,
        ));

        let solution = self.solve_subtask(task.clone())?;

        let mut answer = solution.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = tr(Symbol::with_func_symbol("answer"));
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

    fn solve_subtask(&self, task: Rc<Term>) -> Option<Rc<Solution>> {
        match self.cache.status(&task) {
            Some(TaskStatus::Solved(x)) => return Some(x),
            Some(_) => return None,
            None => {}
        }

        self.cache.add(Term::from(task.root().deep_clone()));

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
        let mut solution = Solution::new(
            subtask,
            self.rules_engine.clone(),
            self.dumper.clone(),
            self.execution_deadline,
            self.cache.clone(),
        );
        solution.cycles = self.cycles.clone();

        match solution.solve() {
            Ok(_) => {
                let solution = Rc::new(solution);
                self.cache
                    .update_status(&task, TaskStatus::Solved(solution.clone()));
                Some(solution)
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

                    let new_states = self.next_term_with_filter(index, |rule| {
                        rule.contains_attribute(&RuleAttr::Equivalence)
                    });
                    if new_states.is_empty() {
                        self.terms[index].weight += 1;
                    }
                    for s in new_states {
                        let _ = self.add_purpose(s);
                    }
                }
            }
            Purpose::Transform(_) => {
                while let Some(index) = self.pick_purpose_term() {
                    if self.terms[index].weight > level {
                        return;
                    }
                    let new_states = self.next_term(index);

                    if new_states.is_empty() {
                        self.terms[index].weight += 1;
                    }
                    for s in new_states {
                        if self.purpose_index.contains_key(&s.term) {
                            continue;
                        }

                        if self.add_purpose(s).is_ok() {
                            self.terms[index].weight = MAX_LEVEL + 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    fn is_answer(&self, term: &TermProps) -> Option<Suppose> {
        let term_root = term.term.root();

        if !self.purpose.is_transform() &&
            term_root.data().is_symbol_name("answer") &&
            term_root.degree() == 1
        {
            let mut resolution = TermProps::from(Rc::from(Term::from(
                (*term.term).clone().root_mut().pop_front().unwrap(),
            )));
            if let Some(parent) = term.parent {
                resolution = resolution.with_parent(parent);
            }
            return Some(Suppose {
                requirements: vec![],
                resolution,
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
                    let is_known = tr(Symbol::with_func_symbol("is")) /
                        term_root.back().unwrap().deep_clone() /
                        tr(Symbol::with_func_symbol("known"));

                    return Some(Suppose {
                        requirements: vec![Rc::new(Term::from(is_known))],
                        resolution:   term.clone(),
                    });
                }
                None
            }
            Purpose::Proof(_) => {
                for i in self.purpose_index.values() {
                    if term_root == self.terms[*i].term.root() {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   term.clone(),
                        });
                    }
                    if self.terms[*i].term.root().check_truth().is_true() {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   self.terms[*i].clone().without_parents(),
                        });
                    }
                }
                None
            }
            Purpose::Transform(_) => {
                if let Some(index) = self.pick_purpose_term() {
                    if self.terms[index].weight > MAX_LEVEL {
                        return Some(Suppose {
                            requirements: vec![],
                            resolution:   self
                                .terms
                                .iter()
                                .rev()
                                .find(|x| x.is_purpose)
                                .unwrap()
                                .clone()
                                .without_parents(),
                        });
                    }
                }
                None
            }
        }
    }
}

fn is_replace(root: &mut SymbolNode) {
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
            let mut neg = tr(Symbol::with_func_symbol("!")) / child;
            swap_node(root, &mut neg.root_mut());
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
        task::{parse_task, Solution},
        term::term_with_vars,
    };

    #[test]
    fn check_answer_find_test() {
        let task = parse_task("task {purpose find(x); x == 1;}");
        let rules = Arc::new(RulesEngine::default());
        let mut solution = Solution::new(
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
        let mut solution = Solution::new(
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
