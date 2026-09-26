use crate::{
    NormLevel,
    term::{SharedTerm, Term, TermBuf, Truth, TruthCtx, domain},
};

/// What the task presupposes about its own statement.
///
/// INVARIANT: conditions are normalized, distinct, none trivially true.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Premises {
    // CONTEXT: linear dedup — a task yields a handful of conditions.
    parts: Vec<Premise>,
}

/// A condition and the term it was read off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Premise {
    pub condition: SharedTerm,
    pub source:    SharedTerm,
}

impl Premises {
    pub fn iter(&self) -> impl Iterator<Item = &Premise> {
        self.parts.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Admits the condition unless it is trivially true or already held.
    ///
    /// An identically false one is kept with a warning: an empty domain or a
    /// `.sym` bug.
    pub(crate) fn insert(&mut self, condition: TermBuf, source: SharedTerm, ctx: TruthCtx) -> bool {
        let condition = domain::marked(condition.term())
            .map(|c| c.to_owned())
            .unwrap_or(condition)
            .normalize(NormLevel::Full);

        match condition.term().truth(ctx) {
            Truth::True => return false,
            Truth::False => warn!(
                target: "premises",
                "premise from {source} is identically false: {condition}"
            ),
            Truth::Unknown => {}
        }

        if self.parts.iter().any(|p| *p.condition == condition) {
            return false;
        }
        self.parts.push(Premise {
            condition: SharedTerm::new(condition),
            source,
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Premises;
    use crate::term::{SharedTerm, TruthCtx, domain, term_with_vars};

    fn source() -> SharedTerm {
        SharedTerm::new(term_with_vars("sqrt(a)"))
    }

    fn insert(premises: &mut Premises, condition: &'static str) -> bool {
        premises.insert(term_with_vars(condition), source(), TruthCtx::default())
    }

    #[test]
    fn an_undecidable_condition_is_admitted() {
        let mut premises = Premises::default();
        assert!(insert(&mut premises, "0 <= a"));
        assert_eq!(premises.iter().count(), 1);
        assert_eq!(
            premises.iter().next().unwrap().condition.to_string(),
            "0<=a"
        );
    }

    #[test]
    fn a_trivially_true_condition_is_dropped() {
        let mut premises = Premises::default();
        assert!(!insert(&mut premises, "0 <= 4"));
        assert!(premises.is_empty());
    }

    #[test]
    fn a_duplicate_is_dropped() {
        let mut premises = Premises::default();
        assert!(insert(&mut premises, "0 <= a"));
        assert!(!insert(&mut premises, "0 <= a"));
        assert_eq!(premises.iter().count(), 1);
    }

    /// Admission normalizes, so two spellings collapse.
    #[test]
    fn a_duplicate_in_another_form_is_dropped() {
        let mut premises = Premises::default();
        assert!(insert(&mut premises, "0 <= a"));
        assert!(!insert(&mut premises, "0 <= a + 0"));
        assert_eq!(premises.iter().count(), 1);
    }

    #[test]
    fn an_identically_false_condition_is_kept() {
        let mut premises = Premises::default();
        assert!(insert(&mut premises, "0 <= -4"));
        assert_eq!(premises.iter().count(), 1);
    }

    #[test]
    fn the_marker_is_stripped() {
        let mut premises = Premises::default();
        let wrapped = domain::mark(term_with_vars("0 <= a"));
        assert!(premises.insert(wrapped, source(), TruthCtx::default()));
        assert_eq!(
            premises.iter().next().unwrap().condition.to_string(),
            "0<=a"
        );
    }

    #[test]
    fn the_source_is_kept_beside_the_condition() {
        let mut premises = Premises::default();
        insert(&mut premises, "0 <= a");
        assert_eq!(
            premises.iter().next().unwrap().source.to_string(),
            "sqrt(a)"
        );
    }
}
