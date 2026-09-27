#!/usr/bin/env python3
"""Импорт задач из задачника «Искры» (reshatel-api) в формат .pbl.

Источник — baseline/zadachnik_block2.json (14 790 задач с целями,
посылками, условиями и ответами в скобочной записи термов).

Использование:
    python scripts/iskra_import.py --section "3 2" --stats
    python scripts/iskra_import.py --section "3 2" --out-dir tasks/unsolved/iskra
"""

import argparse
import collections
import json
import sys
from pathlib import Path

DEFAULT_ISKRA = Path(__file__).resolve().parents[1] / ".." / ".." / "iskra" / "reshatel-api"

LosTerm = None  # ставит load_losterm() на старте

# Подразделы оглавления задачника: путь -> (имя файла, название)
SECTIONS = {
    ("3", "2", "1"): ("rational_1", "Рациональные уравнения - 1."),
    ("3", "2", "2"): ("rational_2", "Рациональные уравнения - 2."),
    ("3", "2", "3"): ("irrational_1", "Иррациональные уравнения - 1."),
    ("3", "2", "4"): ("irrational_2", "Иррациональные уравнения - 2."),
    ("3", "2", "5"): ("systems_1", "Системы алгебраических уравнений - 1."),
    ("3", "2", "6"): ("systems_2", "Системы алгебраических уравнений - 2."),
    ("3", "2", "7"): ("systems_3", "Системы алгебраических уравнений - 3."),
    ("3", "2", "8"): ("systems_4", "Системы алгебраических уравнений - 4."),
    ("3", "2", "10"): ("exponential", "Показательные уравнения."),
    ("3", "4", "1"): ("identities", "Доказательство тождеств."),
    ("3", "4", "2"): ("inequalities_1", "Доказательство неравенств - 1."),
    ("3", "4", "3"): ("inequalities_2", "Доказательство неравенств - 2."),
    ("3", "4", "4"): ("inequalities_3", "Доказательство неравенств - 3."),
}

# Метки оглавления после 9 — словом
NUMERALS = {"десять": 10, "одиннадцать": 11, "двенадцать": 12, "тринадцать": 13,
            "четырнадцать": 14, "пятнадцать": 15}

# Бинарные/n-арные головы: терм Искры -> (шаблон, приоритет)
CMP = {
    "равно": "==",
    "меньше": "<",
    "меньшеилиравно": "<=",
    "больше": ">",
    "большеилиравно": ">=",
}

ATOMS = {
    "истина": "true",
    "ложь": "false",
}

# Головы, которые конвертер умеет. Всё прочее => задача не портируется.
SUPPORTED = (
    set(CMP)
    | set(ATOMS)
    | {
        "плюс",
        "минус",
        "умножение",
        "дробь",
        "степень",
        "величина",
        "модуль",
        "и",
        "или",
        "не",
        "число",
        "принадлежит",
        "перечень",
    }
)

# приоритеты выражений (больше = сильнее связывает)
P_OR, P_AND, P_NOT, P_CMP, P_SUM, P_MUL, P_NEG, P_POW, P_ATOM = range(1, 10)


class Unsupported(Exception):
    """Терм содержит голову вне словаря конвертера."""


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--iskra", type=Path, default=DEFAULT_ISKRA, help="корень reshatel-api")
    ap.add_argument("--section", required=True, help='путь раздела, напр. "3 2"')
    ap.add_argument("--stats", action="store_true", help="только статистика покрытия")
    ap.add_argument("--out-dir", type=Path, help="каталог: по файлу на подраздел")
    ap.add_argument("--limit", type=int, default=0, help="не больше N задач")
    return ap.parse_args()


def main():
    args = parse_args()
    load_losterm(args.iskra)

    baseline = args.iskra / "baseline" / "zadachnik_block2.json"
    tasks = load_section(baseline, args.section.split())

    converted, skipped = [], collections.Counter()
    for t in tasks:
        try:
            converted.append(convert_task(t))
        except Unsupported as e:
            skipped[str(e)] += 1

    print(f"раздел {args.section}: всего {len(tasks)}, портируемо {len(converted)}")
    if args.stats or not args.out_dir:
        for reason, n in skipped.most_common(30):
            print(f"  {n:5}  {reason}")
        return

    if args.limit:
        converted = converted[: args.limit]
    by_section = collections.defaultdict(list)
    for c in converted:
        by_section[c["section"]].append(c)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    for section, group in sorted(by_section.items()):
        path = args.out_dir / section_file(section)
        head = f"// {'.'.join(section)}. {SECTIONS.get(section, ('', ''))[1]}\n\n"
        path.write_text(head + "".join(render_task(c) for c in group), encoding="utf-8")
        print(f"записано {len(group)} задач -> {path}")


def load_section(baseline, want):
    data = json.loads(baseline.read_text(encoding="utf-8"))
    return [t for t in data["tasks"] if toc(t)[: len(want)] == want]


def toc(t):
    """Путь задачи в оглавлении задачника: ['3', '2', '1', ...]; числительные — цифрами."""
    p = (t.get("toc_path") or [""])[0]
    toks = p[p.find("(") + 1 : p.rfind(")")].split() if p else []
    return [str(NUMERALS.get(x, x)) for x in toks]


def section_file(section):
    """3_2_01_rational_1.pbl: номер подраздела в два знака держит порядок оглавления."""
    *head, last = section
    slug = SECTIONS.get(section, ("",))[0]
    return "_".join([*head, last.zfill(2)] + ([slug] if slug else [])) + ".pbl"


def load_losterm(iskra):
    """Парсер термов Искры лежит в соседнем чекауте (--iskra)."""
    global LosTerm
    sys.path.insert(0, str(iskra / "scripts"))
    try:
        from losterm import LosTerm
    except ImportError:
        sys.exit(f"не найден losterm.py в {iskra / 'scripts'}; путь задаётся --iskra")


def convert_task(t):
    """Задача baseline -> dict {id, goal, answer, givens}. Unsupported => исключение."""
    goals = t["task"].get("цели", [])
    premises = [LosTerm.parse(s) for s in t["task"].get("посылка", [])]
    conditions = [LosTerm.parse(s) for s in t["task"].get("условие", [])]

    unknowns, inputs = [], None
    for g in goals:
        gt = LosTerm.parse(g)
        if gt.head == "неизвестные":
            unknowns = [a.head for a in gt.args]
        elif gt.head == "известно":
            # исходные данные: только они вправе войти в ответ
            inputs = {VarNames._norm(a.head) for a in gt.args or []}
    if "пример" in goals:
        raise Unsupported("цель пример")

    names = VarNames(unknowns)
    task_id = f"iskra:{t['symbol']}:{t['node']}"
    section = tuple(toc(t)[:3])

    if "доказать" in goals:
        stmt = " && ".join(render(c, P_AND, names) for c in conditions)
        goal = f"prove({stmt})"
        givens = [render(p, P_OR, names) for p in premises]
        answer = []
    elif unknowns:
        goal = f"find({', '.join(names.get(u) for u in unknowns)})"
        unknown_keys = {VarNames._norm(u) for u in unknowns}

        # число(неизвестная) не эмитим: given «x in Reals» лаффи принимает
        # за готовый ответ find(x) и обрывает поиск
        def is_reality(t):
            return (
                t.head == "число"
                and len(t.args or []) == 1
                and VarNames._norm(t.args[0].head) in unknown_keys
            )

        givens = [render(c, P_OR, names) for c in conditions if not is_reality(c)]
        givens += [render(p, P_OR, names) for p in premises if not is_reality(p)]
        answer = [render(LosTerm.parse(a), P_OR, names) for a in t.get("answer") or []]
        if not answer:
            raise Unsupported("нет ответа-эталона")
        if answer == ["false"]:
            if len(unknowns) > 1:
                raise Unsupported("нет решений при нескольких неизвестных")
            answer = [f"{names.get(unknowns[0])} in empty_set"]
        # без `is known` параметр — ещё одна неизвестная
        givens += [f"{p} is known" for p in names.params_used(inputs)]
    else:
        raise Unsupported(f"цели: {' '.join(goals) or '<пусто>'}")

    return {"id": task_id, "section": section, "goal": goal, "answer": answer, "givens": givens}


def render_task(c):
    lines = ["task {\n", f"    goal {c['goal']};\n", f"    id \"{c['id']}\";\n"]
    if c["answer"]:
        lines.append(f"    answer {', '.join(c['answer'])};\n")
    lines.append("\n")
    for g in c["givens"]:
        if g != "true":  # вырожденное условие «истина» — шум
            lines.append(f"    {g};\n")
    lines.append("}\n\n")
    return "".join(lines)


class VarNames:
    """х23/x23... -> x, y, z; параметры -> a, b, c, ..."""

    UNKNOWN_LETTERS = "xyzuvw"
    PARAM_LETTERS = "abcdefghklmnpqrst"

    def __init__(self, unknowns):
        self.map = {}
        for i, u in enumerate(unknowns):
            if i >= len(self.UNKNOWN_LETTERS):
                raise Unsupported("слишком много неизвестных")
            self.map[self._norm(u)] = self.UNKNOWN_LETTERS[i]
        self.params = iter(self.PARAM_LETTERS)

    def params_used(self, only=None):
        """Буквы параметров; only — ключи исходных данных цели `известно`."""
        return [
            v
            for k, v in self.map.items()
            if v not in self.UNKNOWN_LETTERS and (only is None or k in only)
        ]

    def get(self, head):
        key = self._norm(head)
        if key not in self.map:
            try:
                self.map[key] = next(self.params)
            except StopIteration:
                raise Unsupported("слишком много параметров") from None
        return self.map[key]

    @staticmethod
    def _norm(head):
        return head.replace("х", "x")  # кириллическая х -> латиница

    @staticmethod
    def is_var(head):
        h = VarNames._norm(head)
        return len(h) > 1 and h[0] == "x" and h[1:].isdigit()


def render(t, ctx, names):
    """Терм Искры -> выражение лаффи; скобки, если приоритет ниже контекста."""
    prec, s = _render(t, names)
    return f"({s})" if prec < ctx else s


def _render(t, names):
    h, args = t.head, list(t.args or [])

    if not args:
        if h in ATOMS:
            return P_ATOM, ATOMS[h]
        if VarNames.is_var(h):
            return P_ATOM, names.get(h)
        if h.isdigit():
            return P_ATOM, h
        raise Unsupported(f"атом {h}")

    if h not in SUPPORTED:
        raise Unsupported(f"голова {h}")

    if h == "величина":
        return P_ATOM, velichina(args)
    if h in CMP:
        need(h, args, 2)
        lhs = render(args[0], P_SUM, names)
        rhs = render(args[1], P_SUM, names)
        return P_CMP, f"{lhs} {CMP[h]} {rhs}"
    if h == "плюс":
        parts = [render(args[0], P_SUM, names)]
        for a in args[1:]:
            if a.head == "минус" and len(a.args or []) == 1:
                parts.append(f" - {render(a.args[0], neg_ctx(a.args[0]), names)}")
            else:
                parts.append(f" + {render(a, P_MUL, names)}")
        return P_SUM, "".join(parts)
    if h == "минус":
        need(h, args, 1)
        return P_NEG, f"-{render(args[0], neg_ctx(args[0]), names)}"
    if h == "умножение":
        return P_MUL, "*".join(render(a, P_NEG, names) for a in args)
    if h == "дробь":
        need(h, args, 2)
        return P_MUL, f"{render(args[0], P_MUL, names)}/{render(args[1], P_NEG, names)}"
    if h == "степень":
        need(h, args, 2)
        e = args[1]
        if e.head == "дробь" and [a.head for a in e.args] == ["1", "2"]:
            return P_ATOM, f"sqrt({render(args[0], P_OR, names)})"
        return P_POW, f"{render(args[0], P_POW + 1, names)}^{render(e, P_POW + 1, names)}"
    if h == "модуль":
        need(h, args, 1)
        return P_ATOM, f"abs({render(args[0], P_OR, names)})"
    if h == "и":
        return P_AND, " && ".join(render(a, P_AND + 1, names) for a in args)
    if h == "или":
        return P_OR, " || ".join(render(a, P_OR + 1, names) for a in args)
    if h == "не":
        need(h, args, 1)
        inner = args[0]
        if inner.head == "равно" and len(inner.args) == 2:
            lhs = render(inner.args[0], P_SUM, names)
            rhs = render(inner.args[1], P_SUM, names)
            return P_CMP, f"{lhs} != {rhs}"
        return P_NOT, f"!({render(inner, P_OR, names)})"
    if h == "число":
        need(h, args, 1)
        return P_CMP, f"{render(args[0], P_ATOM, names)} in Reals"
    if h == "принадлежит":
        need(h, args, 2)
        return P_CMP, f"{render(args[0], P_SUM, names)} in {render(args[1], P_ATOM, names)}"
    if h == "перечень":
        return P_ATOM, f"set({', '.join(render(a, P_OR, names) for a in args)})"
    raise Unsupported(f"голова {h}")


def neg_ctx(t):
    """Контекст операнда под минусом: степень — всегда в скобках.

    PEG-грамматика лаффи разбирает «- 3» как minus_number и не
    откатывается, так что «a - 3^x» без скобок не парсится.
    """
    return P_ATOM if t.head == "степень" else P_NEG


def velichina(args):
    """величина(2 5 , 5) -> 25.5 (операнды — цифры, ',' — десятичная точка)."""
    digits = []
    for a in args:
        if a.args:
            raise Unsupported("величина с составным операндом")
        digits.append("." if a.head == "," else a.head)
    s = "".join(digits)
    if not s.replace(".", "", 1).isdigit():
        raise Unsupported(f"величина {s}")
    return s


def need(head, args, n):
    if len(args) != n:
        raise Unsupported(f"{head}/{len(args)}")


if __name__ == "__main__":
    main()
