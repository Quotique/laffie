use std::fmt;

use html_escape::encode_text;

use mcore::problem::{Frame, Solution};

pub struct Html<'a>(pub &'a Solution);

impl<'a> fmt::Display for Html<'a> {
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
                    writeln!(
                        f,
                        "<u>{}</u>,",
                        encode_text(&self.0.stack[*p].statement.to_string())
                    )?;
                }
                writeln!(
                    f,
                    "<b>{}</b>",
                    encode_text(&self.0.stack[t.0].statement.to_string())
                )?;
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
