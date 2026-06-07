# Paper draft — состояние и план (session handoff)

Документ для подхвата работы над статьёй в новой чистой сессии.
Содержит framing-решения, терминологическую политику, состояние
черновика и кода, незакрытые задачи, cross-refs.

## Контекст

Статья — **продолжение Solver-paper** («Архитектура упрощённой
системы автоматического вывода на основе подхода А. С. Подколзина»,
файл `../../Solver-paper/main.tex` в соседнем worktree), целевой
venue — журнал «Интеллектуальные системы. Теория и приложения»
(LaTeX-класс `intsys`).

**Фокус статьи**: расширение логического решателя задач
детерминированными алгоритмами через два механизма — вызов
подзадачи из правила и вызов внешнего алгоритма (процедурный символ).
CAS-делегирование (SymPy) — основной пример расширения,
демонстрируется на каталоге из 12 параметрических задач, на которых
SymPy 1.14.0 систематически теряет решения.

## Версии черновика

Активные параллельные версии в `doc/ru/paper/`:

| Версия | Файлы | Объём | Назначение |
|---|---|---|---|
| Markdown long | `01_introduction.md` .. `09_conclusion.md` | ~5500 слов | 9 секций, развёрнутая для основной статьи |
| LaTeX compact | `paper_compact.tex` | ~3500 слов | 2 раздела, без преамбулы, под краткий тезисный формат |

Outline статьи — в `doc/ru/parametric_catalog.md`, раздел «Outline
статьи (черновик)» (около строки 631). Sympy-outline (broader, 31
карточка по R×M) — в `doc/ru/sympy_catalog.md`, придерживается как
запас для расширенной версии.

## Структура

### Compact (LaTeX, `paper_compact.tex`)

```
\section{Архитектура механизма делегирования}
  \subsection{Вызов подзадачи из правила}        % find-block, B3
  \subsection{Вызов внешнего алгоритма из правила} % процедурный символ, 4 блока
  \subsection{Пример}                              % rational_roots
\section{Классы параметрических задач, вызывающих трудности у sympy}
  \subsection{Решение задач с параметрами}        % 3 стадии × 9 приёмов
  \subsection{Примеры}                            % C1, C29, C21, C2
  \subsection{Результаты}                         % сетка + карта приёмов + пределы
```

### Markdown long (9 файлов)

```
§1 Введение                      (01_introduction.md)
§2 Связанные работы              (02_related_work.md, 4 направления, минимальный обзор)
§3 Архитектура и механизм делегирования (03_architecture.md)
  §3.1 Концептуальная схема             — 3-этапный цикл + гранулярность
  §3.2 Механизм делегирования           — 4 блока + rational_roots
  §3.3 Вызов подзадачи из правила       — find-block, B3 (NEW после rebase)
§4 Семейство логических приёмов  (04_techniques.md, 3 стадии × 9 приёмов)
§5 Каталог примеров               (05_catalog.md)
  §5.1 Представительные карточки  — C1, C29, C21, C2
  §5.2 Остальные карточки         — таблица 8 карточек
  §5.3 Сетка покрытия             — структура × что искать, 5/30 ячеек
§6 Покрытие, пробелы и обобщение  (06_coverage.md)
§7 Реализация в описываемой системе (07_implementation.md, C1 walkthrough)
§8 Ограничения                    (08_limitations.md)
§9 Заключение                     (09_conclusion.md)
```

## Ключевые framing-решения

- **Имя «laffie» не упоминается в тексте** — только в URL репозитория
  (как в Solver-paper). В прозе: «описываемая система», «логическое
  ядро», «логический слой».
- **Технические детали минимизированы** для фундаментального
  журнала: Rust, Python, PyO3, sympy.Poly, cargo — не упоминаются
  явно. Конкретные SymPy-функции (`solve`, `solveset`, `rref`)
  оставлены — это содержание статьи.
- **Гранулярность делегации** (fine-grained vs coarse-grained)
  заменила прежний «in-tree vs external» framing. Старая дихотомия
  была неточной: rational_roots использует SymPy внутри 3 из 5
  калькуляторов (`is_polynomial`, `free_term`, `poly_div` — обёртки
  над `sympy.Poly`); только `divisors` и `substitute` чисто
  внутренние.
- **Параметр правила vs параметр задачи** — явный квалификатор там,
  где соседствуют. В §3.2 параметры рулевые (F, d, u, Q); в §7
  параметр C1-правила `a` совпадает по имени с параметром задачи `a`
  через унификацию, дисамбигуация в §7 явная.
- **Два параллельных механизма делегирования** — процедурный символ
  (§1.2/§3.2) и вызов подзадачи из правила (§1.1/§3.3). C2 —
  единственный пример, использующий оба в одном правиле; rational_roots
  использует только §1.2/§3.2.
- **Не убираем rational_roots из §1.3 / §3.2** — пользователь явно
  попросил оставить (более раннее предложение заменить на C2
  отклонено).

## Терминологическая политика (Solver-paper register)

| Англ. | Русский (используем) |
|---|---|
| case-split | разбор случаев |
| generator-binding | связывание по генератору |
| calculator post-filter | пост-фильтр калькулятора |
| grounding | конкретизация |
| `Hypothesis::ground` (в прозе) | конкретизация параметров правила |
| rule-based слой/ядро/решатель | логический слой / логическое ядро / логический решатель |
| rule-based (прил.) | правиловый / основанный на правилах |
| pivot | ведущий элемент |
| proof-term | терм-доказательство |
| Gröbner | Гребнер |
| PyO3-bridge | мост PyO3 / обёртка над SymPy (контекстно) |
| end-to-end | сквозной |
| showcase | демонстрация / пример |
| pruning | отсечение |
| warm-cache | переиспользование между вызовами |

**Сохранены английскими** только в code-spans и proper nouns:
- API SymPy: `solve`, `solveset`, `rref`, `factor`, `limit`, `linsolve`
- Системы: Sledgehammer, Isabelle, Coq, Lean, SymPy, Mathematica,
  Maple, Maxima, REDLOG, Theorema, ACL2
- Тактики: `nlinarith`, `polyrith`, `omega`, `linarith`, `ring`
- Code-names системы: `sympy_solve`, `divisors`, `is_polynomial`,
  `free_term`, `poly_div`, `substitute`, `Hypothesis::ground`,
  `cargo test`
- Концепты в код-листингах: `find(x)`, `prove(s)`, `transform(s)`,
  `goal`, `attr`, `answer(...)`

**Синонимы калькулятора**: основной — «калькулятор»; «оракул» — где
подчёркивается граница доверия (только в §2.1 motivation в md);
«алгоритм» — когда речь о теле процедурного символа (внутри обёртки).
«Вычислительное ядро» больше не используется (путалось с «логическим
ядром»).

## Состояние реализации (`paper` после rebase на `simpy_troubles`)

### Phase 1 cards (end-to-end)
- **C1** (`a·x = a`) — case-split + `sympy_solve`. Канонический ответ
  за ~637 циклов.
- **C24** (`(a²−4)·x = a−2`) — факторизация + case-split. Использует
  то же универсальное правило `x*a + b == 0` из `symbols/equal.sym`.
- **C2** (`(a²−1)·x² + 2(a−1)·x + 1 = 0`) — двухуровневый разбор
  случаев. Использует ОБА механизма: find-block subtask в ветви
  `a == 0` + `sympy_solve` в ветви `a != 0`. Канонический за ~2724
  циклов.

### B3: «вызов подзадачи из правила»
- **Синтаксис**: `find(vars) { eqs; ... }` в требованиях правила.
- **AST**: преобразуется в `solve(find(vars), eqs...)` с отдельным
  головным символом `solve`, отделяющим суб-вычисление от обычной
  целевой установки `find`.
- **Resolution**: `resolve_solve_in_hypothesis` в
  `src/solver/task/solver.rs` запускает суб-решатель на матче
  `solve(...) == Param` и splice-ит ответ обратно (без `answer(...)`
  обёртки).
- **Recursion guard**: cache-key по полному выражению `solve(...)`.
  При повторном вхождении той же формы возвращается empty marker
  (рекурсивная ветвь обрывается). Другие формы (меньшая степень и
  т. п.) получают свежие ключи и решаются нормально.
- **Inheritance**: parent facts наследуются в суб-solve с фильтром
  по `unknown_terms` подзадачи.

### Ключевые файлы кода
- `symbols/sympy_solve.sym` — обёртка над `sympy.solve`.
- `symbols/equal.sym` — правила case-split для линейного и
  квадратного уравнения (level 2), правило канонизации (level 5).
- `symbols/{divisors,is_polynomial,free_term,poly_div}.sym` —
  процедурные символы для rational_roots.
- `symbols/power.sym` — правило поиска рациональных корней (level 5).
- `src/solver/term/symbol/base/substitute.rs` — Rust core.
- `src/solver/task/solver.rs` — `resolve_solve_in_hypothesis`,
  `check_answer_term`, cache-recursion guard.
- `src/parser/grammar.rs` — синтаксис `find(vars) { eqs; ... }`.
- `tasks/sympy_comparison/parametric/{c1,c2,c24}.pbl` — задачи.
- `doc/ru/parametric_implementation_plan.md` — план Phase 1 + B3
  (на ветке).

### Caveat
- `tasks/sympy_comparison/parametric/c2.pbl` всё ещё содержит две
  формы ответа (каноническая + литеральная), хотя каноническая теперь
  достигается. Литеральная форма устарела. Чистка — отдельный коммит,
  не относится к paper-черновику.
- SymPy-репродьюсеры — в `doc/sympy_audit/`, независимая
  регрессионная suite.

## Незакрытые задачи

### Контент черновика
1. **§7 md-версии (Реализация в описываемой системе)** — не обновлён
   под C2/find-block. Сейчас walkthrough только C1. Логично либо
   обновить под C2 (теперь find-block доступен на ветке), либо
   добавить параллельный C2-walkthrough после §7.2 (C1).
2. **§1 md-версии (Введение)** — упоминает только «два уровня
   гранулярности» процедурного символа; стоит мягко упомянуть второй
   механизм (subtask call from rule) для согласования с §3.3.
3. **Cross-refs внутри текста после §3.3** — `04_techniques.md`,
   `05_catalog.md`, `06_coverage.md`, `07_implementation.md` ссылались
   на §3.1 и §3.2; убедиться, что переходы корректны при наличии
   §3.3.
4. **Цитаты** — около 12 TODO-плейсхолдеров в обеих версиях:
   `Solver-paper`, `SymPy 1.14`, `Baader-Nipkow`, `Подколзин 2008`,
   `Sledgehammer`, `nlinarith/polyrith`, `Mathematica Reduce / Collins
   CAD`, `Weispfenning`, `Sit`, `Theorema`, `Dolzmann-Sturm REDLOG`,
   `ACL2`, `SO #59995637` для C23.

### Сборка LaTeX (compact-версия)
1. **Преамбула** — `\documentclass{intsys}` + `\usepackage{listings}`,
   `\usepackage{amsbib}` (взять `intsys.cls`, `amsbib.sty` из
   `../../Solver-paper/`).
2. **Title / Abstract / Keywords** — пока нет. Рабочий заголовок:
   «Гибридное решение параметрических задач: координация логического
   вывода и детерминированного решателя».
3. **Author block** — Анненков А. П., аспирант МГУ, мех-мат, кафедра
   математической теории интеллектуальных систем (по образцу
   Solver-paper).
4. **Bibliography** — `\begin{thebibliographyRU}` + `\begin{thebibliographyEN}`
   по образцу Solver-paper.
5. **Введение / Связанные работы** — в compact.tex отсутствуют.
   Решение по этим разделам зависит от целевого формата.

## Cross-refs (в paper-ветке)

### Свои файлы paper-черновика
- `doc/ru/paper/01_introduction.md` — §1, Введение.
- `doc/ru/paper/02_related_work.md` — §2, Связанные работы.
- `doc/ru/paper/03_architecture.md` — §3 + §3.1/§3.2/§3.3.
- `doc/ru/paper/04_techniques.md` — §4, Семейство приёмов.
- `doc/ru/paper/05_catalog.md` — §5 + §5.1/§5.2/§5.3.
- `doc/ru/paper/06_coverage.md` — §6, Покрытие и пробелы.
- `doc/ru/paper/07_implementation.md` — §7, Реализация.
- `doc/ru/paper/08_limitations.md` — §8, Ограничения.
- `doc/ru/paper/09_conclusion.md` — §9, Заключение.
- `doc/ru/paper/paper_compact.tex` — compact-версия (две секции).

### Опорные документы в репо
- `../../Solver-paper/main.tex` — статья-предшественник (вне
  paper-worktree, в соседней папке `Solver-paper/`).
- `doc/ru/parametric_catalog.md` — outline статьи + 12 карточек +
  сетка покрытия + сводная карта приёмов.
- `doc/ru/parametric_implementation_plan.md` — план Phase 1 + B3
  (актуально после rebase на simpy_troubles).
- `doc/ru/sympy_catalog.md` — broader каталог (31 карточка по R×M).
- `doc/ru/rational_roots.md` — описание правила rational_roots.
- `doc/ru/hypothesis.md` — описание конкретизации параметров.
- `doc/sympy_audit/` — регрессионная SymPy-suite.

### Memory entries (cross-session)
В `~/.config/claude/fft/projects/-home-std-documents-projects-soft-rust-laffie-master/memory/`:
- `project_paper_active_outline.md` — parametric outline активен
- `project_paper_predecessor.md` — Solver-paper, intsys-class
- `project_sympy_delegation.md` — общая направленность
- `project_sympy_publication_priority.md` — Phase 1 implementation-first
- `feedback_no_quotique.md`, `feedback_bilingual_readme.md`,
  `feedback_commit_language.md`, `feedback_concise_commits.md`,
  `feedback_no_bibliographies.md` — преференции пользователя
- `feedback_descriptive_symbol_names.md`,
  `feedback_param_is_known.md`, `feedback_new_symbol_known_chain.md`,
  `feedback_legacy_todo_correctness.md` — code-side фидбэк

## Конвенции работы

- **Коммиты в English**, заголовок + 3-5 строк тела максимум, с
  Co-Authored-By в конце (по `feedback_concise_commits.md` и стилю
  recent commits на simpy_troubles).
- **Чат в русском**, терсе.
- **Confirm before committing** — не коммитить автоматически после
  multi-step работы; запрашивать подтверждение.
- **Defer follow-ups until usage signal** — после shipping малой
  версии не строить сразу heavy follow-up.
