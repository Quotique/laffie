## Сборка

Для сборки требуется компилятор Rust. Инструкция по установке на оф. сайте https://www.rust-lang.org/tools/install.

После установки собрать и запустить систему можно командой:

```bash
cargo run --release --bin cli -- -c config/cli.yaml
```

Рекомендуется использовать имено релизную сборку (флаг --release), так как время работы отличается существенно.

## [Интерфейс TUI](doc/tui_ru.md)

## [Терминология](doc/terminology_ru.md)

## [Принцип работы](doc/work_principle_ru.md)

## [Как вводить задачи](doc/task_syntax_ru.md)

## [Как вводить правила](doc/rule_syntax_ru.md)

## [Структура каталогов](doc/code_structure_ru.md)
