use std::fmt;

use colored::*;

use mcore::{problem::Solution, utils::VecDisplay};

pub struct Console<'a>(pub &'a Solution);

impl<'a> fmt::Display for Console<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(a) = self.0.answer {
            let mut trace: Vec<usize> = vec![a];

            while let Some(parent) = self.0.stack[*trace.last().unwrap()].parent {
                trace.push(parent);
            }

            let mut parent = trace.pop().unwrap();
            while let Some(id) = trace.pop() {
                writeln!(
                    f,
                    "\n{} [requirements: {}]\n=> {}",
                    self.0.stack[parent]
                        .statement
                        .to_string()
                        .underline()
                        .bold(),
                    VecDisplay(&self.0.stack[id].requirements),
                    self.0.stack[id].statement.to_string().bold().yellow()
                )?;
                parent = id;
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
