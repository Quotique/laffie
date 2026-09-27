use crate::{Location, syntax::Span};

/// Parsed text, for turning spans back into locations.
pub(crate) struct Source<'a> {
    text:           &'a str,
    new_line_poses: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            new_line_poses: Location::new_line_poses(text),
        }
    }

    pub fn loc(&self, span: Span) -> Location {
        Location::from_position(
            self.text,
            span.start,
            span.end - span.start,
            &self.new_line_poses,
        )
    }
}
