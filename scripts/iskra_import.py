#!/usr/bin/env python3
"""Импорт задач из задачника «Искры» (reshatel-api) в формат .pbl.

Источник — baseline/zadachnik_block2.json (14 790 задач с целями,
посылками, условиями и ответами в скобочной записи термов).

Использование:
    python scripts/iskra_import.py --section "3 2" --stats
    python scripts/iskra_import.py --section "3 2" --out tasks/iskra/eq_32.pbl
"""

import argparse
import collections
import json
import sys
from pathlib import Path

DEFAULT_ISKRA = Path(__file__).resolve().parents[1] / ".." / ".." / "reshatel-api"

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
    ap.add_argument("--out", type=Path, help="целевой .pbl")
    ap.add_argument("--limit", type=int, default=0, help="не больше N задач")
    return ap.parse_args()


def main():
    args = parse_args()
    sys.path.insert(0, str(args.iskra / "scripts"))

    baseline = args.iskra / "baseline" / "zadachnik_block2.json"
    tasks = load_section(baseline, args.section.split())

    converted, skipped = [], collections.Counter()
    for t in tasks:
        try:
            converted.append(convert_task(t))
        except Unsupported as e:
            skipped[str(e)] += 1

    print(f"раздел {args.section}: всего {len(tasks)}, портируемо {len(converted)}")
    if args.stats or not args.out:
        for reason, n in skipped.most_common(30):
            print(f"  {n:5}  {reason}")
        return

    if args.limit:
        converted = converted[: args.limit]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(render_task(c) for c in converted), encoding="utf-8")
    print(f"записано {len(converted)} задач -> {args.out}")


def load_section(baseline, want):
    data = json.loads(baseline.read_text(encoding="utf-8"))
    out = []
    for t in data["tasks"]:
        p = (t.get("toc_path") or [""])[0]
        toks = p[p.find("(") + 1 : p.rfind(")")].split() if p else []
        if toks[: len(want)] == want:
            out.append(t)
    return out


def convert_task(t):
    """Задача baseline -> dict {id, goal, answer, givens}. Unsupported => исключение."""
    from losterm import LosTerm

    goals = t["task"].get("цели", [])
    premises = [LosTerm.parse(s) for s in t["task"].get("посылка", [])]
    conditions = [LosTerm.parse(s) for s in t["task"].get("условие", [])]

    unknowns = []
    for g in goals:
        gt = LosTerm.parse(g)
        if gt.head == "неизвестные":
            unknowns = [a.head for a in gt.args]

    names = VarNames(unknowns)
    task_id = f"iskra:{t['symbol']}:{t['node']}"

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
    else:
        raise Unsupported(f"цели: {' '.join(goals) or '<пусто>'}")

    return {"id": task_id, "goal": goal, "answer": answer, "givens": givens}


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
        h = head.replace("х", "x")
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
