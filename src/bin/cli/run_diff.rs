use std::fmt;

use colored::*;

use database::Run;
use solver::term::TermBuf;

/// Regression diff of the current run against each task's last stored run.
///
/// Renders (via [`fmt::Display`]) the non-empty buckets followed by a one-line
/// count summary.
#[derive(Default)]
pub struct RunDiff {
    newly_failing:  Vec<String>,
    newly_solved:   Vec<String>,
    answer_changed: Vec<String>,
    slower:         Vec<String>,
}

impl RunDiff {
    /// Classifies one task against its previous run (`None` = no baseline yet).
    pub fn record(
        &mut self,
        label: &str,
        prev: Option<&Run>,
        now_answer: Option<&TermBuf>,
        now_ms: u64,
    ) {
        let Some(prev) = prev else {
            return;
        };
        let was_solved = prev.stats.status.is_answer();
        let is_solved = now_answer.is_some();
        match (was_solved, is_solved) {
            (true, false) => self.newly_failing.push(label.to_owned()),
            (false, true) => self.newly_solved.push(label.to_owned()),
            (true, true) if prev.stats.answer.as_ref() != now_answer => {
                self.answer_changed.push(label.to_owned());
            }
            _ => {}
        }
        if let Some(prev_ms) = prev.stats.duration_ms &&
            prev_ms > 0 &&
            now_ms > prev_ms.saturating_mul(2)
        {
            self.slower
                .push(format!("{label} ({prev_ms}ms -> {now_ms}ms)"));
        }
    }

    /// `true` when a task that used to be solved no longer is (a regression).
    pub fn has_regression(&self) -> bool {
        !self.newly_failing.is_empty()
    }
}

impl fmt::Display for RunDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn section(
            f: &mut fmt::Formatter<'_>,
            title: &str,
            color: Color,
            items: &[String],
        ) -> fmt::Result {
            if items.is_empty() {
                return Ok(());
            }
            writeln!(f, "{} ({})", title.bold().color(color), items.len())?;
            for i in items {
                writeln!(f, "  {i}")?;
            }
            Ok(())
        }

        section(f, "NEWLY FAILING", Color::Red, &self.newly_failing)?;
        section(f, "NEWLY SOLVED", Color::Green, &self.newly_solved)?;
        section(
            f,
            "ANSWER CHANGED (vs last run)",
            Color::Yellow,
            &self.answer_changed,
        )?;
        section(f, "SLOWER >2x", Color::Magenta, &self.slower)?;
        write!(
            f,
            "diff vs last run: {} newly failing, {} newly solved, {} answer changed, {} slower",
            self.newly_failing.len(),
            self.newly_solved.len(),
            self.answer_changed.len(),
            self.slower.len(),
        )
    }
}
