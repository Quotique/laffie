use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use colored::*;

use solver::{
    rule::{RuleAttr, RuleAttrValue, RuleId, RulesEngine, SharedRule},
    task::{TermInference, Tracer},
};

const LABEL_WIDTH: usize = 88;

/// Corpus-wide rule usage tally for `--coverage`; one handle is shared across
/// every task's `TracerHub`.
#[derive(Clone, Default)]
pub struct CoverageTracer(Arc<Mutex<HashMap<RuleId, Tally>>>);

/// Rendered rule-coverage summary.
pub struct CoverageReport {
    total: usize,
    used:  usize,
    dead:  Vec<DeadRule>,
}

#[derive(Clone, Copy, Default)]
struct Tally {
    accepted: usize,
    rejected: usize,
}

/// A rule with no accepted hypothesis. `rejected == 0` = never fired;
/// `rejected > 0` = fired but all rejected.
struct DeadRule {
    label:    String,
    rejected: usize,
}

impl CoverageTracer {
    /// Classifies every loaded rule as used (≥1 accepted) or dead.
    pub fn report(&self, engine: &RulesEngine) -> CoverageReport {
        let tally = self.0.lock().unwrap();
        let mut total = 0;
        let mut used = 0;
        let mut dead = Vec::new();
        for rule in engine.iter() {
            total += 1;
            let t = tally.get(&rule.id).copied().unwrap_or_default();
            if t.accepted > 0 {
                used += 1;
            } else {
                dead.push(DeadRule {
                    label:    rule_label(&rule),
                    rejected: t.rejected,
                });
            }
        }
        // Fired-but-useless first, then never-fired.
        dead.sort_by(|a, b| {
            b.rejected
                .cmp(&a.rejected)
                .then_with(|| a.label.cmp(&b.label))
        });
        CoverageReport { total, used, dead }
    }
}

impl Tracer for CoverageTracer {
    fn on_hypothesis_finish(&mut self, inference: &TermInference, _cycle: usize) {
        let Some(rule) = inference.rule() else {
            return;
        };
        let proven = inference.is_proven();
        let mut tally = self.0.lock().unwrap();
        let t = tally.entry(rule.id).or_default();
        if proven {
            t.accepted += 1;
        } else {
            t.rejected += 1;
        }
    }
}

fn rule_label(rule: &SharedRule) -> String {
    let name = rule
        .attribute(&RuleAttr::Id)
        .filter_map(RuleAttrValue::str)
        .next();
    let mut label = match name {
        Some(name) => format!("{name}: {rule}"),
        None => rule.to_string(),
    };
    if label.chars().count() > LABEL_WIDTH {
        label = label.chars().take(LABEL_WIDTH - 1).collect::<String>() + "…";
    }
    label
}

impl fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let never_fired = self.dead.iter().filter(|d| d.rejected == 0).count();
        writeln!(f, "{}", "── rule coverage ──".bold().blue())?;
        writeln!(
            f,
            "{} rules over the tasks run: {} used, {} dead ({} never fired, {} fired but useless)",
            self.total,
            self.used.to_string().green(),
            self.dead.len().to_string().red(),
            never_fired,
            self.dead.len() - never_fired,
        )?;
        for d in &self.dead {
            let tag = if d.rejected == 0 {
                "never fired    ".dimmed()
            } else {
                format!("{:>5} rej      ", d.rejected).yellow()
            };
            writeln!(f, "  {tag}  {}", d.label)?;
        }
        Ok(())
    }
}
