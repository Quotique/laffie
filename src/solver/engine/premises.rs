use std::iter::once;

use crate::{
    rule::{ApplyRule, RulesEngine, TermFilters},
    task::{GoalKind, Premises, Task},
    term::{SharedTerm, TruthCtx},
};

impl Premises {
    /// Reads the premises off a task's statement — its givens and its goal.
    ///
    /// One pass closes the set while no requirement mentions another premise. An
    /// undecided requirement drops the candidate.
    pub(super) fn derive(rules: &RulesEngine, task: &Task) -> Self {
        // Under `find` the conditions ride into every subtask and exhaust the budget.
        if task.goal().kind() != GoalKind::Prove {
            return Self::default();
        }

        let known = task.known_vars();
        let mut premises = Self::default();

        let inputs = task
            .givens
            .iter()
            .cloned()
            .chain(once(SharedTerm::new(task.goal().subject().to_owned())));

        for input in inputs {
            for rule in rules.premise_rules() {
                let mut filters = TermFilters::from(input.symbols());
                filters.level = rule.level;
                let Ok(hypotheses) = rule.apply(&input, &filters, task.goal().term()) else {
                    continue;
                };
                for mut hypothesis in hypotheses {
                    hypothesis.resolve_parents(&input);
                    for grounded in hypothesis.ground(&known) {
                        if !grounded.requirements.is_empty() {
                            continue;
                        }
                        premises.insert(grounded.resolution, input.clone(), TruthCtx::new(&known));
                    }
                }
            }
        }
        premises
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        rule::{RulesEngine, parse_rule},
        task::{Premises, parse_task},
    };

    fn engine(rules: &[&'static str]) -> RulesEngine {
        rules
            .iter()
            .map(|r| parse_rule(r))
            .collect::<Result<_, _>>()
            .unwrap()
    }

    const SQRT: &str = r#"rule sqrt(a) => domain(0 <= a) { attr premise; }"#;

    #[test]
    fn a_condition_is_read_off_the_goal() {
        let premises = Premises::derive(
            &engine(&[SQRT]),
            &parse_task("task prove(sqrt(a) + 1 > 0) {}"),
        );

        assert_eq!(premises.iter().count(), 1);
        assert_eq!(
            premises.iter().next().unwrap().condition.to_string(),
            "0<=a"
        );
    }

    #[test]
    fn a_condition_is_read_off_a_given() {
        let premises = Premises::derive(
            &engine(&[SQRT]),
            &parse_task("task prove(a >= 0) { sqrt(a) + 1 < 5; }"),
        );

        assert_eq!(premises.iter().count(), 1);
        assert_eq!(
            premises.iter().next().unwrap().condition.to_string(),
            "0<=a"
        );
    }

    /// Both roots yield a condition, and the inner one is not shadowed.
    #[test]
    fn a_nested_root_yields_both_conditions() {
        let premises = Premises::derive(
            &engine(&[SQRT]),
            &parse_task("task prove(sqrt(sqrt(a) - 1) + 1 > 0) {}"),
        );

        let mut conditions: Vec<String> =
            premises.iter().map(|p| p.condition.to_string()).collect();
        conditions.sort();
        assert_eq!(conditions, vec!["0<=a", "0<=sqrt(a)-1"]);
    }

    #[test]
    fn a_find_task_yields_nothing() {
        let premises = Premises::derive(
            &engine(&[SQRT]),
            &parse_task("task find(x) { sqrt(x) == x - 2; }"),
        );

        assert!(premises.is_empty());
    }

    #[test]
    fn a_task_without_a_source_term_yields_nothing() {
        let premises = Premises::derive(
            &engine(&[SQRT]),
            &parse_task("task prove(a >= 0) { a > 1; }"),
        );

        assert!(premises.is_empty());
    }

    #[test]
    fn an_undecidable_requirement_drops_the_candidate() {
        let guarded = r#"rule sqrt(a) => domain(0 <= a) { attr premise; a is known; }"#;
        let premises = Premises::derive(
            &engine(&[guarded]),
            &parse_task("task prove(sqrt(a) + 1 > 0) {}"),
        );

        assert!(premises.is_empty());
    }

    #[test]
    fn a_satisfied_requirement_keeps_the_candidate() {
        let guarded = r#"rule sqrt(a) => domain(0 <= a) { attr premise; a is known; }"#;
        let premises = Premises::derive(
            &engine(&[guarded]),
            &parse_task("task prove(sqrt(a) + 1 > 0) { a is known; }"),
        );

        assert_eq!(premises.iter().count(), 1);
    }
}
