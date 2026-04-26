use serde_derive::{Deserialize, Serialize};

use solver::{
    rule::{Rule, RuleAttr, RuleAttrValue},
    task::{Solution, SolutionStatus, SolveError, TermInference, TermProps},
    term::{ParamSubstitution, TermBuf},
};

/// Read-only mirror of [`solver::task::Solution`] suitable for persistence.
///
/// Strips runtime-only state (caches, `Arc` graphs of `SharedRule`/
/// `SharedSolution`) and replaces them with index-based references into
/// `terms` and `sub_solutions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolutionTrace {
    pub status: TraceStatus,

    pub terms:         Vec<TraceTerm>,
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
    /// `Display` form of [`solver::task::SolveError`].
    Err(String),
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
        /// Indices into [`SolutionTrace::sub_solutions`].
        requirements: Vec<usize>,
    },
    Transform {
        parent:       usize,
        /// Index into [`SolutionTrace::sub_solutions`].
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
        let mut sub_solutions: Vec<SolutionTrace> = Vec::new();

        let terms = s
            .terms
            .iter()
            .map(|tp| TraceTerm::from_props(tp, &mut sub_solutions))
            .collect();

        let find_bindings = s
            .find_bindings
            .iter()
            .map(|(t, idx)| (t.clone(), *idx))
            .collect();

        SolutionTrace {
            status: TraceStatus::from(s.status),
            terms,
            sub_solutions,
            find_bindings,
        }
    }
}

impl From<SolutionStatus> for TraceStatus {
    fn from(s: SolutionStatus) -> Self {
        match s {
            SolutionStatus::NotDone => Self::NotDone,
            SolutionStatus::Answer(idx) => Self::Answer(idx),
            SolutionStatus::Err(e) => Self::Err(e.to_string()),
        }
    }
}

impl TraceStatus {
    /// `true` iff the trace finished without an error or pending state.
    pub fn is_answer(&self) -> bool {
        matches!(self, Self::Answer(_))
    }

    /// Converts back into a [`SolveError`] when the status carries one.
    /// Loses identity — only `Display` strings are matched — so this is
    /// strictly best-effort.
    pub fn as_solve_error(&self) -> Option<SolveError> {
        let Self::Err(s) = self else {
            return None;
        };
        match s.as_str() {
            "StackOverflow" => Some(SolveError::StackOverflow),
            "MaxSubtaskLevelExceed" => Some(SolveError::MaxSubtaskLevelExceed),
            "NoConditions" => Some(SolveError::NoConditions),
            "NoSolutionsFound" => Some(SolveError::NoSolutionsFound),
            "ExecutionDeadline" => Some(SolveError::ExecutionDeadline),
            "Canceled" => Some(SolveError::Canceled),
            _ => None,
        }
    }
}

impl TraceTerm {
    fn from_props(tp: &TermProps, sinks: &mut Vec<SolutionTrace>) -> Self {
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
                    .map(|sub| {
                        let idx = sinks.len();
                        sinks.push(SolutionTrace::from(sub.as_ref()));
                        idx
                    })
                    .collect();
                TraceInference::Rule {
                    parent:       *parent,
                    rule_ref:     RuleRef::from(rule.as_ref()),
                    params:       TraceParams::from(params),
                    requirements: req_indices,
                }
            }
            TermInference::Transform { parent, solution } => {
                let idx = sinks.len();
                sinks.push(SolutionTrace::from(solution.as_ref()));
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
                .map(|(k, v)| (k.as_ref().as_str().to_owned(), v.clone()))
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
