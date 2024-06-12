use std::{
    collections::HashMap,
    fmt,
    iter::Iterator,
    ops::{Index, IndexMut},
    rc::Rc,
    sync::Arc,
};

use trees::tr;

use crate::{
    predefine::{normalize, symbol_by_name},
    rule::{Rule, RulesEngine, SharedRule, Suppose},
    term::{
        symbol::Symbol,
        tree_utils::{swap_node, NodeMapping},
        Term, TermNode, TermProps,
    },
    utils::{Dumper, DumperSink, VecDisplay},
    NormalizationLevel,
};

use super::{
    builder::TaskBuilder,
    cache::{TaskStatus, TasksCache},
    solution::{Solution, SolutionError},
};

pub const STACK_SIZE: usize = 2048;

pub struct Frame {
    stack: Vec<TermProps>,
    index: HashMap<Rc<Term>, usize>,

    rules_engine: Arc<RulesEngine>,
    dumper:       Dumper,

    subtask_level: usize,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", VecDisplay(&self.stack))
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

fn is_replace(root: &mut TermNode) {
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
            let mut neg = tr(Symbol::FuncSymbol(symbol_by_name("!").unwrap())) / child;
            swap_node(root, &mut neg.root_mut());
        }
        _ => {}
    }
}

impl Frame {
    pub fn new(rules: Arc<RulesEngine>, dump: Dumper, level: usize) -> Self {
        Frame {
            stack:        Vec::new(),
            index:        HashMap::new(),
            rules_engine: rules,
            dumper:       dump,

            subtask_level: level,
        }
    }

    pub fn with_terms(
        rules: Arc<RulesEngine>,
        dumper: Dumper,
        terms: impl IntoIterator<Item = TermProps>,
        level: usize,
    ) -> Self {
        let mut result = Self::new(rules, dumper, level);
        for i in terms {
            // TODO: error processing
            let _ = result.add_condition(i);
        }
        result
    }

    #[inline]
    pub fn contains(&self, term: &Rc<Term>) -> bool {
        self.index.contains_key(term)
    }

    #[inline]
    pub fn find(&self, term: &Rc<Term>) -> Option<usize> {
        self.index.get(term).copied()
    }

    #[inline]
    pub fn dumper(&mut self) -> &mut Dumper {
        &mut self.dumper
    }

    pub fn add_condition(&mut self, mut term: TermProps) -> Result<(), SolutionError> {
        if self.contains(&term.term) {
            return Ok(());
        }
        self.dumper.add_term(
            &term,
            &term
                .parent
                .map(|id| self.stack[id].clone())
                .unwrap_or_else(|| TermProps::from(Rc::new(Term::zero()))),
        );

        term.id = self.stack.len();
        if self.stack.len() + 1 > STACK_SIZE {
            return Err(SolutionError::StackOverflow);
        }
        self.index.insert(term.term.clone(), self.stack.len());
        self.stack.push(term);
        Ok(())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &TermProps> {
        self.stack.iter()
    }

    #[inline]
    pub fn last(&self) -> Option<&TermProps> {
        self.stack.last()
    }

    pub fn pick_condition(&self) -> Result<usize, SolutionError> {
        self.stack
            .iter()
            .enumerate()
            .filter(|(_, x)| !x.replaced)
            .min_by_key(|(_, x)| x.weight)
            .map(|(num, _)| num)
            .ok_or(SolutionError::NoConditions)
    }

    pub fn suppose_proof(
        &self,
        suppose: &Suppose,
        cache: Arc<TasksCache>,
    ) -> Option<Vec<Rc<Term>>> {
        let mut result = vec![];
        for req in suppose.requirements.iter() {
            if let Some(proofed) = self.proof(req, cache.clone()) {
                result.push(proofed);
            } else {
                trace!(target: "rule_selection", "Can't proof: {}", req);
                return None;
            }
        }
        Some(result)
    }

    // Returns proof purpose (is a key for tasks cache)
    pub fn proof(&self, term: &Term, cache: Arc<TasksCache>) -> Option<Rc<Term>> {
        let mut clone = term.root().deep_clone();
        is_replace(&mut clone.root_mut());
        // TODO: normalization level
        normalize(&mut clone.root_mut(), NormalizationLevel::max());

        let proof_purpose = Rc::new(Term::from(tr(Symbol::with_func_symbol("proof")) / clone));

        if term.root().check_truth().is_true() {
            return Some(proof_purpose);
        }

        if let Some(status) = cache.status(&proof_purpose) {
            match status {
                TaskStatus::Solved(_) => return Some(proof_purpose),
                _ => return None,
            }
        }

        cache.add(Term::from(proof_purpose.root().deep_clone()));

        let subtask = TaskBuilder::default()
            .with_purpose(TermProps::from(proof_purpose.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                self.stack
                    .iter()
                    .filter(|x| !x.term.root().data().is_symbol_name("answer"))
                    .cloned(),
            )
            .with_level(self.subtask_level + 1)
            .build()
            .expect("Can't build subtask");
        let mut solution = Solution::new(subtask, self.rules_engine.clone(), self.dumper.clone());

        if let Err(e) = solution.solve_subtask(cache.clone()) {
            trace!("Can't proof {}: {}", term, e);
            cache.update_status(&proof_purpose, TaskStatus::NotSolved);
            return None;
        }
        cache.update_status(&proof_purpose, TaskStatus::Solved(Rc::new(solution)));
        Some(proof_purpose)
    }

    pub fn transform(&mut self, index: usize, cache: Arc<TasksCache>) -> Option<TermProps> {
        if self[index].simplified {
            return None;
        }
        self[index].simplified = true;

        let (answer_wrap, to_transform) = if self[index].term.root().data().is_symbol_name("answer")
        {
            (true, self[index].term.root().front().unwrap().deep_clone())
        } else {
            (false, self[index].term.root().deep_clone())
        };

        let task = Rc::new(Term::from(
            tr(Symbol::with_func_symbol("transform")) / to_transform,
        ));

        cache.add(Term::from(task.root().deep_clone()));

        let subtask = TaskBuilder::default()
            .with_purpose(TermProps::from(task.clone()))
            .expect("Can't build subtask")
            .with_conditions(
                self.stack
                    .iter()
                    .filter(|x| !x.term.root().data().is_symbol_name("answer"))
                    .cloned(),
            )
            .with_level(self.subtask_level + 1)
            .build()
            .expect("Can't build subtask");
        let mut solution = Solution::new(subtask, self.rules_engine.clone(), self.dumper.clone());

        solution.solve_subtask(cache.clone()).ok()?;
        let mut answer = solution.answer().unwrap().as_ref().clone();
        if answer_wrap {
            let mut tmp = tr(Symbol::with_func_symbol("answer"));
            swap_node(&mut answer.root_mut(), &mut tmp.root_mut());
            answer.root_mut().push_back(tmp);
        }
        cache.update_status(&task, TaskStatus::Solved(Rc::new(solution)));

        if *self[index].term == answer {
            return None;
        }
        let mut result = TermProps::from(Rc::new(answer));
        result.blocked_rules.clone_from(&self[index].blocked_rules);
        result.simplified = true;
        result.parent = Some(self[index].id);
        result.requirements.push(task);

        Some(result)
    }

    pub fn next_term(
        &mut self,
        local_rules: &[SharedRule],
        index: usize,
        purpose: &TermProps,
        cache: Arc<TasksCache>,
    ) -> Vec<TermProps> {
        self.next_term_with_filter(local_rules, index, purpose, |_| true, cache)
    }

    pub fn next_term_with_filter(
        &mut self,
        local_rules: &[SharedRule],
        index: usize,
        purpose: &TermProps,
        filter: impl Fn(&Rule) -> bool,
        cache: Arc<TasksCache>,
    ) -> Vec<TermProps> {
        // Need to split &mut self into (&self, &mut MarkedStatement).
        // Only list of applied rules will be chahged in MarkedStatement, so it's safe.
        let term: *mut TermProps = &mut self.stack[index];
        let term: &mut TermProps = unsafe { &mut *term };
        self.next_term_with_term(local_rules, term, purpose, filter, cache)
    }

    pub fn next_term_with_term(
        &self,
        local_rules: &[SharedRule],
        term: &mut TermProps,
        purpose: &TermProps,
        filter: impl Fn(&Rule) -> bool,
        cache: Arc<TasksCache>,
    ) -> Vec<TermProps> {
        for (rule, supposes) in self
            .rules_engine
            .suggest_rules(term, purpose)
            .iter()
            .chain(local_rules.iter())
            .inspect(|rule| trace!(target: "rule_selection", "Rule: {}", rule))
            .filter(|rule| filter(rule))
            .filter_map(|rule| {
                rule.apply(term, purpose)
                    .map_err(|e| trace!(target: "rule_selection", "Rule not applied: {:?}", e))
                    .ok()
                    .map(|supposes| (rule, supposes))
            })
        {
            let res: Vec<_> = supposes
                .into_iter()
                .filter(|suppose| !self.contains(&suppose.resolution.term))
                .inspect(|suppose| trace!(target: "rule_selection", "Suppose: {}", suppose))
                .filter_map(|mut suppose| {
                    if let Some(proofed) = self.suppose_proof(&suppose, cache.clone()) {
                        suppose.resolution.requirements = proofed;
                        Some(suppose)
                    } else {
                        None
                    }
                })
                .inspect(
                    |_| trace!(target: "rule_selection", "Suppose: proofed, resolution applied"),
                )
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
}

impl Index<usize> for Frame {
    type Output = TermProps;

    fn index(&self, index: usize) -> &Self::Output {
        self.stack.index(index)
    }
}

impl IndexMut<usize> for Frame {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.stack.index_mut(index)
    }
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        rc::Rc,
    };

    use crate::term::term_with_params;

    #[test]
    fn hash_test() {
        let term = term_with_params("a*x + c == 0");
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_1 = s.finish();

        let term = Rc::new(term);
        let mut s = DefaultHasher::new();
        term.hash(&mut s);
        let hash_2 = s.finish();

        assert_eq!(hash_1, hash_2);
    }
}
