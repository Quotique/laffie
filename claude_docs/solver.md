# Solver Crate — Navigation Map

Core reasoning engine: term representation, rule system, task-solving algorithm.

## File Map

```
src/solver/
├── lib.rs                    # Exports: rule, task, term. Types: NormalizationLevel, Decimal, CompactString
├── rule/
│   ├── mod.rs                # Re-exports all rule types
│   ├── rule.rs               # Rule, RuleId, Level, SharedRule, ApplyRule trait
│   ├── builder.rs            # RuleBuilder — fluent API for constructing rules
│   ├── hypothesis.rs         # Hypothesis, GroundedHypothesis, HypothesisIterator — grounding pipeline
│   ├── rule_attribute.rs     # RuleAttr enum, RuleAttrValue enum
│   ├── rules_engine.rs       # RulesEngine — central rule registry and dispatcher
│   └── term_filters.rs       # TermFilters, TermFlags (bitflags: REPLACED, SIMPLIFIED, GOAL)
├── task/
│   ├── mod.rs                # Task struct, re-exports
│   ├── goal.rs               # FindGoal, Goal enum: Find(FindGoal), Prove, Transform
│   ├── limits.rs             # CalculationLimits — cycle budgeting with Arc<RwLock>
│   ├── props.rs              # TermProps, TermInference enum, TermAsRule
│   ├── solution.rs           # Solution, SolutionStatus, SolveError, SharedSolution
│   ├── solver.rs             # Solver — main bidirectional search algorithm
│   ├── steps.rs              # Steps iterator, Visit enum, StepsSource trait
│   ├── builder.rs            # TaskBuilder — fluent API for constructing tasks
│   └── tracing/
│       ├── mod.rs            # Re-exports
│       ├── tracer.rs         # Tracer trait (callbacks), TracerHub (aggregator)
│       └── file.rs           # FileDumpTracer — writes trace to file
└── term/
    ├── mod.rs                # Re-exports, test helpers: term_with_params(), term_with_vars()
    ├── atom.rs               # Atom enum (Symbol|Param|Variable|Number|ArgList), Param, Variable, ArgList
    ├── buffer.rs             # TermBuf (owned tree), SharedTerm (Arc), TermPath
    ├── codec.rs              # bincode Encode/Decode for Atom, TermBuf
    ├── refer.rs              # Term trait, TermRef — read-only view, pattern matching, match_term! macro
    ├── refer_mut.rs          # TermMut — mutable view, normalization, evaluate
    ├── substitution.rs       # ParamSubstitution, VariableSubstitution
    └── symbol/
        ├── mod.rs            # Symbol struct, sym(), try_sym()
        ├── program.rs        # SymbolProgram, Truth enum, SymbolAttr, SymbolAttrValue
        ├── container.rs      # Global symbol registry (RwLock<HashMap>), all_func_symbols()
        └── base/             # Built-in symbol implementations (16 modules)
            ├── mod.rs         # Re-exports all base symbols
            ├── equal.rs       # == (truth checker + calculator)
            ├── inequal.rs     # !=
            ├── plus.rs        # + (associative, commutative)
            ├── mul.rs         # * (associative, commutative)
            ├── divide.rs      # /
            ├── power.rs       # ^
            ├── sqrt.rs        # sqrt
            ├── less.rs        # <
            ├── more.rs        # >
            ├── less_or_equal.rs  # <=
            ├── more_or_equal.rs  # >=
            ├── is.rs          # is (property assertion)
            ├── op_true.rs     # true literal
            ├── op_not.rs      # ! (logical not)
            ├── op_in.rs       # in (set membership, truth checker + generator)
            ├── substitute.rs  # substitute(x == val, expr) — variable substitution
            ├── replace.rs     # replace operation
            └── symbolic_eq.rs # symbolic equality
```

## Key Types

### rule/rule.rs
| Line | Type | Description |
|------|------|-------------|
| 15 | `RuleId(u64)` | Unique rule identifier |
| 22 | `Level(u64)` | Rule application level (priority) |
| 30 | `RuleDeclineReason` | Enum: LevelMissmatch, GoalMissmatch, AlreadyApplied, Blocked, ParamSubstitutionErr |
| 48 | `SharedRule` | `Arc<Rule>` |
| 50 | `Rule` | Fields: id, level, symbol, attrs, block, term, pattern, replace, requirements, pattern_symbols |
| 231 | `ApplyRule` trait | `fn apply(&self, term, filters, goal) -> Result<Vec<Hypothesis>, RuleDeclineReason>` |

### task/mod.rs
| Line | Type | Description |
|------|------|-------------|
| 20 | `Task` | Fields: id, text, group, goal, givens, subtask_level, possible_answers |

### rule/hypothesis.rs
| Line | Type | Description |
|------|------|-------------|
| 13 | `Hypothesis` | Non-ground: rule, resolution, free_params, params, requirements, blocked_rules |
| 26 | `GroundedHypothesis` | Ground instance: all params bound, ready for requirement proving |
| 35 | `HypothesisIterator` | Iterator over hypotheses from `Rule::apply()` |
| 48 | `ground()` | Grounds hypothesis: bind `==` → cartesian over generators → re-bind `==` per combo |
| 116 | `bind_equality_params()` | Iteratively binds `param == term` requirements |
| 160 | `extract_generators()` | Extracts `<param> in set(...)` requirements as generators |
| 177 | `Substitute for Hypothesis` | Propagates `ParamSubstitution` to resolution, requirements, free_params |

### task/goal.rs
| Line | Type | Description |
|------|------|-------------|
| 14 | `FindGoal` | Fields: targets(Vec\<TermBuf\>), term(TermProps) |
| 20 | `Goal` | Enum: Find(FindGoal), Prove(TermProps), Transform(TermProps) |

### task/solver.rs
| Line | Type | Description |
|------|------|-------------|
| 20 | `MAX_SUBTASK_LEVEL` | Const: 10 |
| 26 | `EXECUTION_DEADLINE_DEFAULT` | Const: 100_000 |
| 44 | `Solver` | Fields: rules_engine, local_rules |
| 50 | `Solver::new()` | Constructor |
| 70 | `Solver::solve()` | Entry point: `(task, tracer, deadline) -> SharedSolution` |
| 94 | `solve_impl()` | Main loop: focus term → simplify → infer → check answer |
| 248 | `try_infer_new_terms()` | Suggests rules, produces hypotheses, adds new terms |
| 290 | `produce()` | Applies rule, grounds hypotheses, proves requirements |
| 331 | `try_prove_hypothesis()` | Proves rule requirements via subtasks |
| 395 | `prove()` | Creates subtask for proving a term |
| 440 | `transform()` | Handles Transform goals |
| 486 | `solve_subtask()` | Recursive subtask solving |
| 588 | `check_find_answer()` | Incremental multi-var find: binds targets via `==`/`in`, assembles `&&` |
| 636 | `build_multi_find_answer()` | Builds conjunction answer from find_bindings |

### task/solution.rs
| Line | Type | Description |
|------|------|-------------|
| 15 | `STACK_SIZE` | Const: 2048 |
| 17 | `SharedSolution` | `Arc<Solution>` |
| 20 | `SolveError` | Enum: StackOverflow, MaxSubtaskLevelExceed, NoConditions, NoSolutionsFound, ExecutionDeadline, Canceled |
| 30 | `SolutionStatus` | Enum: NotDone, Answer(usize), Err(SolveError) |
| 39 | `Solution` | Fields: task, goal, status, start/end_cycle, main_index, goal_index, terms, find_bindings, unproven_terms_count |

### task/props.rs
| Line | Type | Description |
|------|------|-------------|
| 13 | `TermInference` | Enum: Rule{parent,params,rule,requirements}, Transform{parent,solution}, Condition |
| 37 | `TermProps` | Fields: id, term(SharedTerm), inference, filters, rule(TermAsRule) |

### term/atom.rs
| Line | Type | Description |
|------|------|-------------|
| 18 | `Param(CompactString)` | Rule parameter placeholder |
| 20 | `Variable { name, known }` | Task variable; `known` bit stamped at task load, excluded from eq/hash/serde |
| 92 | `ArgList(u64)` | Variadic argument placeholder `..` |
| 102 | `Atom` | Enum: Symbol, Param, Variable, Number(Decimal), ArgList |

### term/buffer.rs
| Line | Type | Description |
|------|------|-------------|
| 14 | `SharedTerm` | `Arc<TermBuf>` |
| 16 | `TermPath(Vec<usize>)` | Path to a node within a term tree |
| 19 | `TermBuf` | Owned term tree. Factories: symbol(), number(), variable(), param(), arg() |
| 62 | `normalize()` | Applies normalization to given level |

Param/arglist substitution: see `term/substitution.rs`.

### term/substitution.rs
| Line | Type | Description |
|------|------|-------------|
| 8 | `VariableSubstitution` | `HashMap<Variable, TermBuf>` |
| 11 | `ParamSubstitution` | `IndexMap<Param, TermBuf>` + arglists |
| 20 | `Substitute` trait | `substitute()` in-place; `substituted()`, `substitute_iter()` for free |

Implementors: `TermBuf`, `TermMut`, `Hypothesis`.

### term/refer.rs
| Line | Type | Description |
|------|------|-------------|
| 18 | `Term` trait | Generic interface: parent, first/last_arg, args_iter, data, degree, truth |
| 40 | `TermRef<'a>` | Read-only term view |
| 135 | `args_permutations()` | Returns all argument permutations for commutative symbols |
| 176 | `try_match()` | Pattern matching: `self vs pattern → Vec<ParamSubstitution>` |
| 180 | `find_matching_subterms()` | Finds all subterms matching a pattern |
| 434 | `match_term!` | Macro for structural matching: `match_term!(term, "=="(lhs, rhs))` |

### term/refer_mut.rs
| Line | Type | Description |
|------|------|-------------|
| 11 | `TermMut<'a>` | Mutable term view |
| 184 | `evaluate()` | Runs symbol-specific calculator |
| 280 | `normalize()` | Full normalization: associative nesting, commutative reorder, evaluate |

### term/symbol/program.rs
| Line | Type | Description |
|------|------|-------------|
| 13 | `Truth` | Enum: True, False, Unknown |
| 20 | `SymbolAttr` | Enum: Infix, Display, Associative, Commutative |
| 35 | `SymbolProgram` | Symbol definition: name, attrs, arg_cmp, calculator, truth_checker |
| 79 | `register()` | Registers symbol in global registry |

## Module Dependencies

```
term (foundation)
 ├── atom, buffer, refer, refer_mut, substitution
 └── symbol (global registry + base implementations)

rule (depends on term)
 ├── rule.rs: ApplyRule uses TermBuf, TermFilters, ParamSubstitution
 ├── hypothesis.rs: wraps rule application results
 └── rules_engine.rs: indexes rules by Level, suggests by TermFilters

task (depends on rule + term)
 ├── solver.rs: orchestrates rule application on terms
 ├── solution.rs: stores terms and tracks status
 └── tracing/: observability hooks
```
