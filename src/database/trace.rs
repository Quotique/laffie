use std::{collections::HashMap, sync::Arc};

use serde_derive::{Deserialize, Serialize};

use solver::{
    rule::{Rule, RuleAttr, RuleAttrValue},
    task::{SharedSolution, Solution, SolutionStatus, SolveError, TermInference, TermProps},
    term::{ParamSubstitution, TermBuf},
};

/// Sub-solution `Arc` identity → its slot in the global arena (dedup).
type Interner = HashMap<*const Solution, usize>;

/// Read-only mirror of [`solver::task::Solution`] suitable for persistence.
///
/// Strips runtime-only state (caches, `Arc` graphs of `SharedRule`/
/// `SharedSolution`) and replaces them with index-based references into
/// `terms` and `sub_solutions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionTrace {
    pub status: TraceStatus,

    pub terms: Vec<TraceTerm>,

    /// Flat sub-solution arena, populated only on the root trace; all
    /// `requirements` / `sub_solution` indices address it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_solutions: Vec<SolutionTrace>,

    /// `(target_var, idx into self.terms)` for `find(x, y, ...)` goals.
    /// Empty for `prove`/`transform` solutions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub find_bindings: Vec<(TermBuf, usize)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TraceStatus {
    NotDone,
    /// Index into [`SolutionTrace::terms`] holding the answer term.
    Answer(usize),
    Err(TraceError),
}

/// Serializable mirror of [`solver::task::SolveError`]. `Unknown` is the
/// forward-compatible fallback: a variant added to `SolveError` later
/// deserializes here instead of failing to load an older trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceError {
    StackOverflow,
    MaxSubtaskLevelExceed,
    NoConditions,
    NoSolutionsFound,
    ExecutionDeadline,
    Canceled,
    TimeDeadline,
    Internal,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceTerm {
    pub term:      TermBuf,
    pub inference: TraceInference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TraceInference {
    Condition,
    Rule {
        parent:       usize,
        rule_ref:     RuleRef,
        params:       TraceParams,
        /// Indices into the root [`SolutionTrace::sub_solutions`] arena.
        requirements: Vec<usize>,
    },
    Transform {
        parent:       usize,
        /// Index into the root [`SolutionTrace::sub_solutions`] arena.
        sub_solution: usize,
    },
}

/// Stable reference to a rule.
///
/// `Named` carries the `id` attribute set in a `.sym` file. `Anonymous`
/// holds the first 8 bytes of a blake3 hash over the rule's term tree —
/// stable across runs and sufficient for grouping usage statistics
/// (~3×10⁻¹² collision risk at 10⁴ rules).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleRef {
    Named(String),
    Anonymous([u8; 8]),
}

/// Flat, serializable mirror of [`solver::term::ParamSubstitution`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TraceParams {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params:   Vec<(String, TermBuf)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arglists: Vec<(u64, Vec<TermBuf>)>,
}

impl From<&Solution> for SolutionTrace {
    fn from(s: &Solution) -> Self {
        let mut arena: Vec<SolutionTrace> = Vec::new();
        let mut interner: Interner = HashMap::new();
        let mut root = SolutionTrace::build(s, &mut arena, &mut interner);
        root.sub_solutions = arena;
        root
    }
}

impl From<SolutionStatus> for TraceStatus {
    fn from(s: SolutionStatus) -> Self {
        match s {
            SolutionStatus::NotDone => Self::NotDone,
            SolutionStatus::Answer(idx) => Self::Answer(idx),
            SolutionStatus::Err(e) => Self::Err(e.into()),
        }
    }
}

impl From<SolveError> for TraceError {
    fn from(e: SolveError) -> Self {
        match e {
            SolveError::StackOverflow => Self::StackOverflow,
            SolveError::MaxSubtaskLevelExceed => Self::MaxSubtaskLevelExceed,
            SolveError::NoConditions => Self::NoConditions,
            SolveError::NoSolutionsFound => Self::NoSolutionsFound,
            SolveError::ExecutionDeadline => Self::ExecutionDeadline,
            SolveError::Canceled => Self::Canceled,
            SolveError::TimeDeadline => Self::TimeDeadline,
            SolveError::Internal => Self::Internal,
        }
    }
}

impl TraceStatus {
    /// `true` iff the trace finished without an error or pending state.
    pub fn is_answer(&self) -> bool {
        matches!(self, Self::Answer(_))
    }

    /// Converts back into a [`SolveError`] when the status carries a known one;
    /// `None` for a non-error status or an `Unknown` (forward-compat) variant.
    pub fn as_solve_error(&self) -> Option<SolveError> {
        let Self::Err(e) = self else {
            return None;
        };
        Some(match e {
            TraceError::StackOverflow => SolveError::StackOverflow,
            TraceError::MaxSubtaskLevelExceed => SolveError::MaxSubtaskLevelExceed,
            TraceError::NoConditions => SolveError::NoConditions,
            TraceError::NoSolutionsFound => SolveError::NoSolutionsFound,
            TraceError::ExecutionDeadline => SolveError::ExecutionDeadline,
            TraceError::Canceled => SolveError::Canceled,
            TraceError::TimeDeadline => SolveError::TimeDeadline,
            TraceError::Internal => SolveError::Internal,
            TraceError::Unknown => return None,
        })
    }
}

impl SolutionTrace {
    /// Builds a trace body; referenced sub-solutions are interned into `arena`.
    fn build(s: &Solution, arena: &mut Vec<SolutionTrace>, interner: &mut Interner) -> Self {
        let terms = s
            .terms
            .iter()
            .map(|tp| TraceTerm::from_props(tp, arena, interner))
            .collect();
        let find_bindings = s
            .find_bindings
            .iter()
            .map(|(t, idx)| (t.clone(), *idx))
            .collect();
        SolutionTrace {
            status: TraceStatus::from(s.status),
            terms,
            sub_solutions: Vec::new(),
            find_bindings,
        }
    }

    /// Arena index of `sub`, built once and reused on repeat references.
    fn intern(
        sub: &SharedSolution,
        arena: &mut Vec<SolutionTrace>,
        interner: &mut Interner,
    ) -> usize {
        let key = Arc::as_ptr(sub);
        if let Some(&idx) = interner.get(&key) {
            return idx;
        }
        // Reserve the slot before recursing to break any self-reference.
        let idx = arena.len();
        arena.push(SolutionTrace {
            status:        TraceStatus::NotDone,
            terms:         Vec::new(),
            sub_solutions: Vec::new(),
            find_bindings: Vec::new(),
        });
        interner.insert(key, idx);
        let body = SolutionTrace::build(sub.as_ref(), arena, interner);
        arena[idx] = body;
        idx
    }
}

impl TraceTerm {
    fn from_props(tp: &TermProps, arena: &mut Vec<SolutionTrace>, interner: &mut Interner) -> Self {
        let inference = match &tp.inference {
            TermInference::Condition => TraceInference::Condition,
            TermInference::Rule {
                parent,
                params,
                rule,
                requirements,
            } => {
                let req_indices = requirements
                    .iter()
                    .map(|sub| SolutionTrace::intern(sub, arena, interner))
                    .collect();
                TraceInference::Rule {
                    parent:       *parent,
                    rule_ref:     RuleRef::from(rule.as_ref()),
                    params:       TraceParams::from(params),
                    requirements: req_indices,
                }
            }
            TermInference::Transform { parent, solution } => {
                let idx = SolutionTrace::intern(solution, arena, interner);
                TraceInference::Transform {
                    parent:       *parent,
                    sub_solution: idx,
                }
            }
        };
        TraceTerm {
            term: (*tp.term).clone(),
            inference,
        }
    }
}

impl From<&Rule> for RuleRef {
    fn from(rule: &Rule) -> Self {
        if let Some(s) = rule
            .attribute(&RuleAttr::Id)
            .filter_map(RuleAttrValue::str)
            .next()
        {
            return RuleRef::Named(s.to_owned());
        }

        let bytes = serde_json::to_vec(&rule.term).expect("TermBuf serialization is infallible");
        let hash = blake3::hash(&bytes);
        let mut id = [0u8; 8];
        id.copy_from_slice(&hash.as_bytes()[..8]);
        RuleRef::Anonymous(id)
    }
}

impl From<&ParamSubstitution> for TraceParams {
    fn from(s: &ParamSubstitution) -> Self {
        TraceParams {
            params:   s
                .params
                .iter()
                .map(|(k, v)| (k.as_ref().as_str().to_owned(), v.as_ref().clone()))
                .collect(),
            arglists: s
                .arglists
                .iter()
                .map(|(k, v)| (u64::from(*k), v.clone()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_ref_named_round_trip() {
        let r = RuleRef::Named("comm_plus".to_owned());
        let json = serde_json::to_string(&r).unwrap();
        let back: RuleRef = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn rule_ref_anonymous_round_trip() {
        let r = RuleRef::Anonymous([0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
        let json = serde_json::to_string(&r).unwrap();
        let back: RuleRef = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn trace_error_round_trip_and_solve_error() {
        let status = TraceStatus::Err(TraceError::StackOverflow);
        let json = serde_json::to_string(&status).unwrap();
        let back: TraceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_solve_error(), Some(SolveError::StackOverflow));
    }

    #[test]
    fn trace_error_unknown_fallback() {
        // A variant a newer solver might emit deserializes to `Unknown`
        // instead of failing to load an older trace.
        let back: TraceStatus = serde_json::from_str(r#"{"Err":"SomeFutureError"}"#).unwrap();
        assert!(matches!(back, TraceStatus::Err(TraceError::Unknown)));
        assert_eq!(back.as_solve_error(), None);
    }

    #[test]
    fn empty_trace_roundtrip() {
        let trace = SolutionTrace {
            status:        TraceStatus::NotDone,
            terms:         vec![TraceTerm {
                term:      TermBuf::variable("x"),
                inference: TraceInference::Condition,
            }],
            sub_solutions: vec![],
            find_bindings: vec![],
        };
        let json = serde_json::to_vec(&trace).unwrap();
        let back: SolutionTrace = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.terms.len(), 1);
        assert!(matches!(back.status, TraceStatus::NotDone));
    }
}
