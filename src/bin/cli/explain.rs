use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use colored::*;

use solver::{
    engine::{Solution, TermInference, TermProps, Tracer},
    rule::{RuleId, SharedRule},
    task::Task,
};

const MAX_TERMS: usize = 10;
const MAX_RULES: usize = 15;
const LABEL_WIDTH: usize = 72;

/// Aggregating tracer for `--explain`: accounts for where a single solve spent
/// its cycles. One clone lives inside the `TracerHub`, another stays in `main`
/// to render the report after `solve` returns.
#[derive(Clone, Default)]
pub struct ExplainTracer(Arc<Mutex<ExplainData>>);

/// Rendered, sorted view of [`ExplainData`]; see its [`fmt::Display`] impl.
pub struct ExplainReport {
    total_cycles:   usize,
    subtasks:       usize,
    max_depth:      usize,
    subtask_solved: usize,
    subtask_failed: usize,
    top_terms:      Vec<(String, usize)>,
    dropped_terms:  usize,
    rules:          Vec<RuleStat>,
    dropped_rules:  usize,
}

#[derive(Default)]
struct ExplainData {
    total_cycles:   usize,
    focus_counts:   HashMap<String, usize>,
    rule_stats:     HashMap<RuleId, RuleStat>,
    subtasks:       usize,
    max_depth:      usize,
    subtask_solved: usize,
    subtask_failed: usize,
}

#[derive(Clone, Default)]
struct RuleStat {
    label:    String,
    accepted: usize,
    rejected: usize,
}

impl ExplainTracer {
    /// Snapshots the accumulated data into a printable, sorted report.
    pub fn report(&self) -> ExplainReport {
        let data = self.0.lock().unwrap();

        let mut top_terms: Vec<_> = data
            .focus_counts
            .iter()
            .map(|(t, c)| (t.clone(), *c))
            .collect();
        top_terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let dropped_terms = top_terms.len().saturating_sub(MAX_TERMS);
        top_terms.truncate(MAX_TERMS);

        let mut rules: Vec<_> = data.rule_stats.values().cloned().collect();
        rules.sort_by(|a, b| {
            b.rejected
                .cmp(&a.rejected)
                .then_with(|| b.accepted.cmp(&a.accepted))
                .then_with(|| a.label.cmp(&b.label))
        });
        let dropped_rules = rules.len().saturating_sub(MAX_RULES);
        rules.truncate(MAX_RULES);

        ExplainReport {
            total_cycles: data.total_cycles,
            subtasks: data.subtasks,
            max_depth: data.max_depth,
            subtask_solved: data.subtask_solved,
            subtask_failed: data.subtask_failed,
            top_terms,
            dropped_terms,
            rules,
            dropped_rules,
        }
    }
}

impl Tracer for ExplainTracer {
    fn on_subtask_start(&mut self, task: &Task, _cycle: usize) {
        // level 0 is the root task; deeper levels are spawned subtasks.
        if task.subtask_level == 0 {
            return;
        }
        let mut data = self.0.lock().unwrap();
        data.subtasks += 1;
        data.max_depth = data.max_depth.max(task.subtask_level);
    }

    fn on_subtask_end(&mut self, status: &Solution) {
        let mut data = self.0.lock().unwrap();
        // The cycle counter is shared across the whole tree, so the root's
        // end_cycle is the grand total.
        if status.task.subtask_level == 0 {
            data.total_cycles = status.end_cycle;
        } else if status.answer().is_some() {
            data.subtask_solved += 1;
        } else {
            data.subtask_failed += 1;
        }
    }

    fn on_term_focus(&mut self, term: &TermProps, _cycle: usize) {
        let mut data = self.0.lock().unwrap();
        *data.focus_counts.entry(term.to_string()).or_default() += 1;
    }

    fn on_hypothesis_finish(&mut self, inference: &TermInference, _cycle: usize) {
        let Some(rule) = inference.rule() else {
            return;
        };
        let proven = inference.is_proven();
        let mut data = self.0.lock().unwrap();
        let stat = data.rule_stats.entry(rule.id).or_default();
        if stat.label.is_empty() {
            stat.label = rule_label(&rule);
        }
        if proven {
            stat.accepted += 1;
        } else {
            stat.rejected += 1;
        }
    }
}

fn rule_label(rule: &SharedRule) -> String {
    let mut label = rule.to_string();
    if label.chars().count() > LABEL_WIDTH {
        label = label.chars().take(LABEL_WIDTH - 1).collect::<String>() + "…";
    }
    label
}

impl fmt::Display for ExplainReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "── explain ──".bold().blue())?;
        writeln!(f, "cycles: {}", self.total_cycles.to_string().bold())?;
        writeln!(
            f,
            "subtasks: {} (max depth {}, {} solved / {} failed)",
            self.subtasks, self.max_depth, self.subtask_solved, self.subtask_failed,
        )?;

        writeln!(f, "{}", "top focused terms (cycles ≈ focuses):".bold())?;
        for (term, count) in &self.top_terms {
            writeln!(f, "  {:>5}  {term}", count.to_string().yellow())?;
        }
        if self.dropped_terms > 0 {
            writeln!(f, "  (+{} more terms)", self.dropped_terms)?;
        }

        writeln!(f, "{}", "rules by rejected hypotheses:".bold())?;
        for r in &self.rules {
            let total = r.accepted + r.rejected;
            let pct = (r.rejected * 100).checked_div(total).unwrap_or(0);
            writeln!(
                f,
                "  {} rej {} acc ({}% rej)  {}",
                format!("{:>5}", r.rejected).red(),
                format!("{:>5}", r.accepted).green(),
                pct,
                r.label,
            )?;
        }
        if self.dropped_rules > 0 {
            write!(f, "  (+{} more rules)", self.dropped_rules)?;
        }
        Ok(())
    }
}
