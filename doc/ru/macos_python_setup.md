# Настройка Python для сборки на macOS

Парсер (`src/parser`) встраивает интерпретатор Python через `pyo3`, чтобы
делегировать часть символьных преобразований библиотеке **sympy** (см.
[делегирование в sympy](sympy_delegation_plan.md)). Поэтому для сборки и запуска
нужна установка Python, в которой:

- стоит пакет `sympy`;
- есть **динамическая** библиотека `libpythonX.Y.dylib` (а не только `.a`).

На macOS при этом часто всплывают две проблемы рантайма — обе из-за того, что
дистрибутивы вроде Anaconda/conda не прописывают пути в бинарник. Ниже —
как настроить один раз и забыть.

> Все примеры — для Anaconda с Python 3.7. Если у вас другой Python (Homebrew,
> python.org, pyenv), пути и версия будут другими — как их найти, описано в
> шаге 1.

## Симптомы

**1. Не загружается библиотека Python:**

```
dyld[...]: Library not loaded: @rpath/libpython3.7m.dylib
  Reason: tried: '.../target/release/...' (no such file), ...
Abort trap: 6
```

`dyld` не нашёл `libpython*.dylib`, потому что путь к каталогу `lib` Python не
попал в `rpath` бинарника.

**2. Интерпретатор не находит свою стандартную библиотеку:**

```
Could not find platform independent libraries <prefix>
Consider setting $PYTHONHOME to <prefix>[:<exec_prefix>]
Fatal Python error: initfsencoding: unable to load the file system codec
ModuleNotFoundError: No module named 'encodings'
Abort trap: 6
```

Библиотека загрузилась, но интерпретатору не задан `PYTHONHOME`, и он не может
найти модули стандартной библиотеки (`encodings` и т.д.).

## Шаг 1. Найдите параметры своего Python

Возьмите тот Python, в котором установлен (или будет установлен) `sympy`, и
выполните:

```bash
# Проверить, что sympy на месте
python -c "import sympy; print('sympy', sympy.__version__)"

# PYTHONHOME (prefix) и каталог с libpython*.dylib (LIBDIR)
python -c "import sysconfig; print('PREFIX =', sysconfig.get_config_var('prefix')); print('LIBDIR =', sysconfig.get_config_var('LIBDIR'))"

# Имя самой динамической библиотеки
python -c "import sysconfig; print(sysconfig.get_config_var('LDLIBRARY'))"
```

Убедитесь, что файл `LIBDIR/<LDLIBRARY>` существует и это `.dylib`:

```bash
ls -l "$(python -c 'import sysconfig;print(sysconfig.get_config_var("LIBDIR"))')"/libpython*.dylib
```

Если `.dylib` нет (есть только `.a`) — у вас сборка Python без разделяемой
библиотеки, `pyo3` с ней не слинкуется. Поставьте Python с динамической
библиотекой (Anaconda, python.org, либо Homebrew `python@3.x`) и установите в
него `sympy` (`pip install sympy`).

## Шаг 2. Определите целевую триплу Rust

Строка зависит от архитектуры процессора:

```bash
rustc -vV | grep host
```

- Intel: `x86_64-apple-darwin`
- Apple Silicon (M1/M2/…): `aarch64-apple-darwin`

## Шаг 3. Создайте `.cargo/config.toml`

В корне проекта создайте файл `.cargo/config.toml`, подставив **свои** значения
из шагов 1–2 (`<TARGET>` — трипла, `<LIBDIR>` и `<PREFIX>` — пути Python):

```toml
# pyo3 линкуется с libpython, но дистрибутив Python не прописывает rpath
# в бинарник — добавляем его вручную, иначе dyld не находит libpython*.dylib.
[target.<TARGET>]
rustflags = ["-C", "link-args=-Wl,-rpath,<LIBDIR>"]

# Встроенный интерпретатор не находит свою стандартную библиотеку (модуль
# encodings) — задаём PYTHONHOME. cargo прокидывает переменную во все
# запускаемые процессы (cargo run / cargo test).
[env]
PYTHONHOME = { value = "<PREFIX>", force = true }
```

Пример для Anaconda (Python 3.7, Intel):

```toml
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-args=-Wl,-rpath,/Applications/anaconda3/lib"]

[env]
PYTHONHOME = { value = "/Applications/anaconda3", force = true }
```

## Шаг 4. Пересоберите и запустите

Изменение `rustflags` инвалидирует кеш — будет один полный перелинк:

```bash
cargo run --release --bin cli -- -c config/cli.yaml
```

Если задачи из каталога `tasks/test/sympy_solve` решаются — Python и sympy
поднялись корректно.

## Проверка результата

```bash
# rpath прописан в бинарник?
otool -l target/release/cli | grep -A2 LC_RPATH

# на какую libpython слинкован бинарник?
otool -L target/release/cli | grep -i python
```

## Замечания

- **Запуск напрямую.** `PYTHONHOME` из секции `[env]` действует только при
  запуске **через cargo** (`cargo run`, `cargo test`). Если запускаете бинарник
  напрямую, задайте переменную сами:
  `PYTHONHOME=<PREFIX> ./target/release/cli -c config/cli.yaml`.
- **Файл локальный.** `.cargo/config.toml` содержит пути конкретной машины. Если
  работаете в команде с разными окружениями, имеет смысл не коммитить его
  (добавить в `.gitignore`), а держать у каждого свой.
- **Версия Python и sympy.** `pyo3` на сборке выбирает Python из `PYO3_PYTHON`
  (если задан) либо первый подходящий `python3` в `PATH`. Чтобы явно
  зафиксировать интерпретатор: `PYO3_PYTHON=/path/to/python cargo build`.
