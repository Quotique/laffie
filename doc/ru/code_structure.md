# Структура кода

Документ описывает раскладку исходного кода и привязывает основные
понятия из [terminology.md](terminology.md) к конкретным файлам и
типам.

## Воркспейс

Проект — Cargo-воркспейс из нескольких крейтов:

| Крейт | Назначение |
|-------|------------|
| `solver` | Решающее ядро: представление термов, правила, алгоритм решения. |
| `parser` | Грамматика `.sym` и `.pbl` файлов, обходчик каталогов. |
| `view` | Рендеринг решения (Console, HTML, TUI) через трейт `Renderer`. |
| `database` | Хранение задач и истории прогонов (`redb`, JSON+zstd). |
| `utils` | Логирование (`slog`), вспомогательные структуры. |
| `cli`, `tui` | Приложения-бинарники в `src/bin/`. |

`src/bin/tgbot/` находится в дереве, но исключён из воркспейса
(`Cargo.toml: exclude`). Возрождение требует обновления библиотеки
`telegram-bot` и переписывания под новый `Db` API.

Каждому крейту соответствует подкаталог `src/<имя>/`. Дополнительно
существуют:

- `symbols/` — описания символов и правил (`*.sym`), сгруппированные
  по символам.
- `tasks/` — задачи в формате `*.pbl`, сгруппированные по категориям
  (`test`, `elementary_algebra`, …); используются как набор
  интеграционных тестов.
- `config/` — конфигурации бинарников (`*.yaml`, `*.json`).
- `claude_docs/` — детальная навигация по крейтам (списки файлов и
  типов с номерами строк); используется в работе ассистентов и
  поддерживается актуальной.

## Привязка концептов к коду

| Концепт из [terminology.md](terminology.md) | Тип / файл |
|--------------|----------|
| Терм | `solver::term::TermBuf` (`src/solver/term/buffer.rs`), `SharedTerm = Arc<TermBuf>`, `TermRef`/`TermMut` |
| Атомы (символ, параметр, переменная, число, ArgList) | `solver::term::atom::Atom` (`src/solver/term/atom.rs`) |
| Функциональный символ | `solver::term::symbol::SymbolProgram` (`src/solver/term/symbol/program.rs`) |
| Реестр символов | `solver::term::symbol::container` (`src/solver/term/symbol/container.rs`) |
| Базовые символы (+, ==, in, …) | `src/solver/term/symbol/base/*.rs` |
| Подстановка параметров | `solver::term::ParamSubstitution` (`src/solver/term/substitution.rs`) |
| Сопоставление с образцом | `solver::term::TermRef::try_match` (`src/solver/term/refer.rs`) |
| Нормализация | `solver::term::TermMut::normalize` (`src/solver/term/refer_mut.rs`); уровень — `solver::NormalizationLevel` |
| Правило | `solver::rule::Rule` (`src/solver/rule/rule.rs`) |
| Атрибуты правила | `solver::rule::RuleAttr` (`src/solver/rule/rule_attribute.rs`) |
| База правил | `solver::rule::RulesEngine` (`src/solver/rule/rules_engine.rs`) |
| Гипотеза, конкретизация | `solver::rule::Hypothesis`, `GroundedHypothesis` (`src/solver/rule/hypothesis.rs`) |
| Кэш символов терма + флаги + фильтры | `solver::rule::TermFilters`, `TermFlags` (`src/solver/rule/term_filters.rs`) |
| Метаданные терма | `solver::task::TermProps` (`src/solver/task/props.rs`) |
| Источник вывода | `solver::task::TermInference` (там же) |
| Задача | `solver::task::Task` (`src/solver/task/mod.rs`) |
| Целевая установка | `solver::task::Goal` (`src/solver/task/goal.rs`) |
| Решение, статус | `solver::task::Solution`, `SolutionStatus`, `SolveError` (`src/solver/task/solution.rs`) |
| Решающий алгоритм | `solver::task::Solver` (`src/solver/task/solver.rs`) |
| Лимит вложенности подзадач | `MAX_SUBTASK_LEVEL` (там же) |
| Лимит итераций | `EXECUTION_DEADLINE_DEFAULT` (там же) |
| Парсер `.sym` / `.pbl` | `src/parser/` |
| Рендереры | `src/view/` |

## Структура `solver`

`src/solver/` сгруппирован по основным понятиям:

```
src/solver/
├── lib.rs              # NormalizationLevel, Decimal, CompactString
├── term/               # представление термов
│   ├── atom.rs         # Atom: Symbol, Param, Variable, Number, ArgList
│   ├── buffer.rs       # TermBuf, SharedTerm, TermPath
│   ├── refer.rs        # TermRef, try_match, find_matching_subterms
│   ├── refer_mut.rs    # TermMut, normalize, evaluate
│   ├── substitution.rs # ParamSubstitution, Substitute trait
│   ├── codec.rs        # bincode для термов
│   └── symbol/         # реестр символов
│       ├── program.rs  # SymbolProgram, Truth
│       ├── container.rs# глобальный реестр
│       └── base/       # +, *, ==, in, sqrt, … (16 символов)
├── rule/
│   ├── rule.rs         # Rule, RuleId, Level, ApplyRule
│   ├── builder.rs      # RuleBuilder
│   ├── hypothesis.rs   # Hypothesis, ground()
│   ├── rule_attribute.rs   # RuleAttr, RuleAttrValue
│   ├── rules_engine.rs # RulesEngine — двухуровневая база
│   └── term_filters.rs # TermFilters, TermFlags
└── task/
    ├── mod.rs          # Task
    ├── goal.rs         # Goal: Find / Prove / Transform
    ├── props.rs        # TermProps, TermInference
    ├── solution.rs     # Solution, SolutionStatus
    ├── solver.rs       # Solver — основной цикл
    ├── steps.rs        # обход решения для рендеринга
    ├── builder.rs      # TaskBuilder
    └── tracing/        # Tracer, TracerHub
```

Зависимости внутри `solver`:

```
term  ← rule  ← task
```

Подробная навигация с номерами строк ключевых типов — в
[`claude_docs/solver.md`](../../claude_docs/solver.md).

## Структура `parser`

`src/parser/` содержит PEG-грамматику и постобработку:

| Файл | Что |
|------|-----|
| `grammar.rs` | PEG-грамматика (`peg`-крейт). |
| `lang.rs` | Точки входа: `task`, `rule`, `symbol`, `terms`. |
| `term.rs`, `task.rs`, `rule.rs`, `symbol.rs` | Постобработка деревьев в типы из `solver`. |
| `dir_loader.rs` | Обход каталогов с `.sym`/`.pbl` файлами. |
| `error.rs` | Тип ошибок парсера и форматирование сообщений. |

Подробнее — [`claude_docs/parser.md`](../../claude_docs/parser.md).

## Бинарники

`src/bin/cli/` и `src/bin/tui/` — приложения, использующие `solver`,
`parser`, `view`, `database`. Конфигурации лежат в `config/`.
Подробности — [`claude_docs/binaries.md`](../../claude_docs/binaries.md).

`src/bin/tgbot/` оставлен в дереве, но не входит в воркспейс и не
собирается обычным `cargo build` — это архивная версия, которую
нельзя считать частью поддерживаемого кода.

## Где искать что

- **Изменить поведение нормализации** — `src/solver/term/refer_mut.rs`
  и калькуляторы в `src/solver/term/symbol/base/*.rs`.
- **Добавить атрибут правила** — `src/solver/rule/rule_attribute.rs`,
  затем поддержка в `RuleBuilder` и `Solver::suggest_rules`.
- **Изменить порядок выбора термов** — `Solution::pick_next` в
  `src/solver/task/solution.rs`.
- **Изменить логику ответа** — `Solver::check_if_answer` и связанные
  методы в `src/solver/task/solver.rs`.
- **Добавить новый базовый символ** — модуль в
  `src/solver/term/symbol/base/`, регистрация в `container.rs`.
