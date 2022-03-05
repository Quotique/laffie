use std::fmt;

use colored::*;

use mcore::problem::{Frame, Solution};

pub struct Console<'a>(pub &'a Solution);

impl<'a> fmt::Display for Console<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.0.answer.as_ref() {
            let mut trace: Vec<(usize, Vec<usize>)> = vec![];

            fn visitor(stack: &Frame, id: usize, trace: &mut Vec<(usize, Vec<usize>)>) {
                trace.push((id, stack[id].parents.clone()));
                for i in stack[id].parents.iter() {
                    visitor(stack, *i, trace);
                }
            }

            visitor(&self.0.stack, *a, &mut trace);

            while let Some(t) = trace.pop() {
                writeln!(f)?;
                for p in t.1.iter() {
                    writeln!(f, "{},", self.0.stack[*p].statement.to_string().underline())?;
                }
                writeln!(
                    f,
                    "{}",
                    self.0.stack[t.0].statement.to_string().bold().yellow()
                )?;
            }
            writeln!(
                f,
                "{} {}",
                "SOLVED!".green(),
                format!(
                    "[{} cycles, {}ms]",
                    self.0.perf_stats.cycles_count, self.0.perf_stats.absolute_time
                )
                .yellow()
            )
        } else {
            writeln!(f)?;
            writeln!(f, "{}", "NOT SOLVED!".bold().blink().red())
        }
    }
}
