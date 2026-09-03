# Solver Crate — Navigation Map

Terms, rules, and the search that solves a task.

**No line numbers here, on purpose.** Measured 2026-09-03: 5 of 66 were still
correct, most of them stale in files nobody had touched. They rot on every edit
and point you at the wrong place with a straight face. Grep the name instead.

## Layering

```
term ──┬── task ──┐
       └── rule ──┴── engine
```

- **`term`** — the tree and its symbols. Depends on nothing.
- **`task`** — what is asked: `Task`, `Goal`, `Answer`. Depends only on `term`.
- **`rule`** — how terms rewrite: `Rule`, `Hypothesis`. Depends only on `term`.
- **`task` and `rule` do not know about each other** (no `crate::rule` under
  `task/`, no `crate::task` under `rule/`). They are siblings, not a chain.
- **`engine`** — the search. Depends on all three; nothing below depends on it.

A type that needs `Solution` or `TermProps` belongs in `engine` — those are the
vocabulary of the term store, and pushing them down inverts the layering.

## File Map

```
src/solver/
├── lib.rs                  # pub mod engine/rule/task/term; NormLevel, Rational, CompactString
├── rational.rs             # decimal literal → exact ratio; rendering (decimal if 2·5-smooth, else p/q)
├── term/                   # the foundation
│   ├── mod.rs              # re-exports; `answer` symbol module; test helpers term_with_params/vars
│   ├── atom.rs             # Atom (Symbol|Param|Variable|Number|ArgList), Param, Variable, ArgList
│   ├── buffer.rs           # TermBuf (owned tree), SharedTerm = Arc<TermBuf>, TermPath
│   ├── refer.rs            # Term trait, TermRef, match_term! macro, commutative matcher + MATCH_BUDGET
│   ├── refer_mut.rs        # TermMut: mutation, evaluate, normalize (fixpoint cap)
│   ├── substitution.rs     # ParamSubstitution, VariableSubstitution, Substitute trait
│   └── symbol/
│       ├── mod.rs          # Symbol, sym(), try_sym(), symbol_names()
│       ├── program.rs      # SymbolProgram + its hooks: Calculator, TruthChecker, Comparator; Truth, TruthCtx, SymbolAttr
│       ├── container.rs    # global registry (RwLock<HashMap>), all_func_symbols()
│       └── base/           # 20 built-in symbols, one file each
│           ├── answer.rs   # the `answer(x)` marker: NAME, mark(), marked() — re-exported as term::answer
│           ├── equal.rs inequal.rs less.rs less_or_equal.rs more.rs more_or_equal.rs
│           ├── plus.rs mul.rs divide.rs power.rs sqrt.rs
│           ├── logic_and.rs logic_or.rs op_not.rs op_true.rs op_in.rs
│           └── is.rs substitute.rs symbolic_eq.rs
├── task/                   # what is asked
│   ├── mod.rs              # Task, content_id(); test helper parse_task
│   ├── goal.rs             # Goal (private GoalBody), GoalKind, GoalError
│   ├── answer.rs           # Answer (parts, one per unknown), Recognized
│   └── builder.rs          # TaskBuilder — infallible once it holds a Goal
├── rule/                   # how terms rewrite
│   ├── mod.rs              # re-exports; test helper parse_rule
│   ├── rule.rs             # Rule, RuleId, Level, SharedRule, ApplyRule, RuleDeclineReason
│   ├── builder.rs          # RuleBuilder, RuleBuilderError
│   ├── hypothesis.rs       # Hypothesis → GroundedHypothesis, HypothesisIterator, grounding pipeline
│   ├── rule_attribute.rs   # RuleAttr, RuleAttrValue
│   ├── rules_engine.rs     # RulesEngine: rules indexed by Level, suggested by TermFilters
│   └── term_filters.rs     # TermFilters + TermFlags bits (REPLACED, SIMPLIFIED, GOAL)
└── engine/                 # the search
    ├── mod.rs              # the crate's public engine surface
    ├── solver.rs           # Solver: the cycle, term inference, subtasks, answer checks
    ├── solution.rs         # Solution, SolutionStatus, SolveError, SharedSolution, TermIdx
    ├── run.rs              # Run (per-solve state), Limits, CancelToken, subtask cache
    ├── props.rs            # TermProps, TermInference, TermAsRule
    ├── bounds.rs           # bound_implies: `x > 2` proves `x > 0`
    ├── steps.rs            # Steps/Visit/StepsSource — walks a solution for rendering
    └── tracing/            # Tracer trait, TracerHub aggregator, FileDumpTracer
```

## Key Types

### task/goal.rs
| Type | What it is |
|------|------------|
| `Goal` | What a task asks. Wraps a **private** `GoalBody` enum, because a `pub enum`'s variant payloads are constructible from anywhere; the wrapper is what keeps `parse` the only door. |
| `GoalBody` | `Find`/`Prove`/`Transform`, each holding the goal **as written** (`find(x, y)`, not `x`). |
| `Goal::parse` | The only fallible constructor, so every `Goal` that exists is well-formed and the search never re-checks. |
| `Goal::subject` | First argument of the written form; borrows, allocates nothing. |
| `Goal::answer` | A fresh `Answer` over the unknowns; `None` for anything but a `find`. |
| `GoalKind` | `Copy` tag for matching without touching the body. |

### task/answer.rs
| Type | What it is |
|------|------------|
| `Answer` | One `Part` per unknown, in the order asked. Self-contained: parts own their terms, so `term()` asks nothing of a `Solution`. |
| `Part` | `asked` (the unknown) + `got: Option<(SharedTerm, usize)>`. The index only justifies the answer in a trace; nothing else reads it. |
| `Answer::recognize` | Is this term an answer? One unknown → whole/no; several → which part it binds. Known-ness arrives as an injected predicate, so the check needs no engine. |
| `Answer::term` | The parts joined with `&&`, or `None` while any is unbound. Exits on the first gap without allocating. |
| `Recognized` | `No` / `Whole` / `Binding(usize)` — the part's position, not its term. |

### task/mod.rs
| Type | What it is |
|------|------------|
| `Task` | id, name, text, group, givens, subtask_level, possible_answers, and a **private** goal read via `goal()`. Built only through `TaskBuilder` / `from_goal`. |
| `content_id` | `u64` from givens (order-independent) + goal. Formatting- and location-independent; in-memory dedup only. |

### rule/hypothesis.rs
| Type | What it is |
|------|------------|
| `Hypothesis` | Rule application with possibly free params: rule, resolution, free_params, params, requirements, blocked_rules, `pos` (match position in the parent term). |
| `GroundedHypothesis` | All params bound; ready for requirement proving. |
| `ground()` | Binds `==` requirements → cartesian product over generators → re-binds per combination. |
| `resolve_parents()` | Rewrites the `parents` marker into the `set(...)` of the match position's ancestor head symbols. Needs only the hypothesis, which is why it lives here and not in the engine. |
| `HypothesisIterator` | Iterates the hypotheses `Rule::apply` produced. |

### engine/solver.rs
| Type | What it is |
|------|------------|
| `Solver` | Holds only the rule set; `solve(&self, …)` carries no state between tasks. |
| `solve_impl` | The cycle: pick a term → simplify → infer → check for an answer. |
| `try_infer_new_terms` | Chooses the goal pattern **once per focused term** (`prove(...)` wrapper only on a prove goal's own term), then walks the suggested rules. |
| `produce` | Grounds one rule's hypotheses, polls the limits every `DEADLINE_CHECK_INTERVAL` groundings. |
| `AnswerCheck` | `No` / `Found(TermIdx)` / `Derived(Box<TermProps>)`. The checks only answer; the cycle is the single place that writes `SolutionStatus::Answer`. |
| `LocalRules` | Rules derived from a frame's own terms. `IndexMap`, because the order rules are offered in is part of the search's identity. |
| `MAX_SUBTASK_LEVEL` | 10 — deeper nesting aborts with `MaxSubtaskLevelExceed`. |

### engine/run.rs
| Type | What it is |
|------|------------|
| `Run` | Per-`solve` state shared by every frame: limits, cycle counter, subtask cache, tracer. A copy per subtask would change the cycle count and the cache hits. |
| `Limits` | What stops a run: cycle budget, wall clock, cancellation. `Limits::init` also hands back the `CancelToken`. |
| `CancelToken` | Every clone shares one flag, so any thread can stop the run. |
| `CacheKey` | `Goal(Goal)` or `SolveBlock(TermBuf)` — a `solve(...)` call must not collide with the goal inside it. |
| `CacheSlot` | `#[must_use]`, not `Clone`: reserving a key obliges you to fill it. A depth failure releases the key, any other failure stays cached. |

### engine/solution.rs
| Type | What it is |
|------|------------|
| `Solution` | task, status, cycles, `terms` arena, two indexes, `find_answer`, `known_vars`, agenda. |
| `goal_index` | `IndexMap`, not `HashMap`: it is iterated to pick the answer, and a per-process order made the reported answer vary between runs. |
| `main_index` | Stays a `HashMap` — nothing iterates it, and its `contains_key` is on the hypothesis path. |
| `agenda` | Min-heap keyed `(level, id)`; stale entries are discarded lazily on peek. |
| `known_vars` | Names declared `v is known`, kept out of the term so term identity stays independent of known-ness. |
| `Solution::subtask` | Inherits proven, non-goal, non-`answer` terms as conditions, with level reset and inference dropped. |
| `SolutionStatus` | `NotDone` / `Answer(TermIdx)` / `Err(SolveError)`. |

### engine/props.rs
| Type | What it is |
|------|------------|
| `TermProps` | id, term, inference, filters, and a cached `proven` valid only after the term is added. |
| `TermInference` | `Rule{parent,params,rule,requirements}` / `Transform{parent,solution}` / `Condition` — how the term was reached, and the spine a trace walks back. |

### term/refer.rs, refer_mut.rs, buffer.rs
| Type | What it is |
|------|------------|
| `Term` trait | parent, first/last_arg, args_iter, data, degree, truth. Implemented for `TermRef` only; `TermMut` has its own inherent methods plus `as_ref()`. |
| `TermRef<'a>` | Read-only view; `Copy`. Equality is structural, not pointer-based (`same()` is the pointer test). |
| `TermMut<'a>` | Mutation, `evaluate`, `normalize` (repeats a pass to a fixpoint under a cap). |
| `TermBuf` | Owned tree over `trees::Tree<Atom>`. Factories: `symbol`, `number`, `variable`, `param`, `arg`. |
| `match_term!` | Structural match: `match_term!(term, "=="(lhs, rhs))`. |

## Gotchas worth knowing before you edit

- **The dev-dependency cycle.** `solver`'s tests use `parser`, which depends on
  `solver`, so a second instance of this crate is linked and `parser`'s
  `TermBuf`/`Rule`/`Task` are nominally distinct types. `term_with_vars` bridges
  by serde roundtrip; `parse_rule`/`parse_task` cannot and use a documented
  `transmute`. Read the `SAFETY` notes before touching them.
- **Cycle-for-cycle identity is the refactor invariant.** Any change to term
  ordering, rule order, or level bumps moves the search, and the corpus notices.
  The check that catches it: run `tasks/regress` with `--explain`, keep the
  per-task cycle counts, and compare them plus the rendered output byte for
  byte against the run before the change.
- **The `answer` marker has one home.** `term::answer::{mark, marked}`; `marked`
  requires arity 1, so a malformed `answer` is not a marker rather than a panic.
- **`transform` on a goal term leaves `added` false on purpose** in
  `try_infer_new_terms`; the tail's second level bump is what the corpus's
  search order rests on.
