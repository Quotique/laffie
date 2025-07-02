use serde_derive::Deserialize;

use super::{
    file::FileDumpTracer,
    tracer::{EmptyTracer, SolutionTracer},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub sink:     String,
    pub filename: Option<String>,
}

impl Config {
    pub fn build(self) -> SolutionTracer {
        match self.sink.as_str() {
            "file" => SolutionTracer::new(FileDumpTracer::new(&self.filename.unwrap_or_default())),
            "none" => SolutionTracer::new(EmptyTracer {}),
            _ => SolutionTracer::new(EmptyTracer {}),
        }
    }
}
