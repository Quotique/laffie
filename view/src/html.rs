use std::fmt;

use html_escape::encode_text;

use mcore::problem::Solution;

pub struct Html<'a>(pub &'a Solution);

impl<'a> fmt::Display for Html<'a> {
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
                    "\n<u>{}</u>\n<b>{}</b>",
                    encode_text(&self.0.stack[parent].statement.to_string()),
                    encode_text(&self.0.stack[id].statement.to_string())
                )?;
                parent = id;
            }
            writeln!(
                f,
                "<b>SOLVED!</b> [{} cycles, {}ms]",
                self.0.perf_stats.cycles_count, self.0.perf_stats.absolute_time
            )
        } else {
            writeln!(f)?;
            writeln!(f, "<b>NOT SOLVED!</b>")
        }
    }
}
