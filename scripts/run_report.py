#!/usr/bin/env python3
"""Карта вердиктов прогона CLI по импортированному корпусу.

Сопоставляет записи лога CLI (в порядке обработки) с задачами .pbl
(сортированный обход каталога + порядок в файле — тот же порядок)
и печатает раскладку по вердиктам с исходными id Искры.

Использование:
    python scripts/run_report.py <run.log> <.pbl или каталог>... [--json out.json]
"""

import argparse
import json
import re
import sys
from pathlib import Path


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("log", type=Path)
    ap.add_argument("tasks", type=Path, nargs="+", help=".pbl-файлы или каталоги")
    ap.add_argument("--json", type=Path, help="полный отчёт в JSON")
    return ap.parse_args()


def main():
    args = parse_args()
    ids = pbl_ids(pbl_files(args.tasks))
    records = log_records(args.log.read_text(encoding="utf-8"))
    report(ids, records, args.json)


def report(ids, records, json_path=None):
    """Раскладка по вердиктам; json_path — полный отчёт в JSON."""
    if len(ids) != len(records):
        sys.exit(f"рассинхрон: {len(ids)} задач в .pbl, {len(records)} записей в логе")

    rows = [{"id": i, **r} for i, r in zip(ids, records)]
    by_verdict = {}
    for r in rows:
        by_verdict.setdefault(r["verdict"], []).append(r)

    for verdict in ("solved", "wrong_answer", "not_solved", "no_verdict"):
        group = by_verdict.get(verdict, [])
        print(f"== {verdict}: {len(group)}")
        for r in group if verdict == "solved" else group[:15]:
            cycles = f"  [{r['cycles']} cycles]" if r["cycles"] is not None else ""
            extra = f"  получено: {r['obtained']}" if r.get("obtained") else ""
            print(f"   {r['id']}{cycles}{extra}")
        if verdict != "solved" and len(group) > 15:
            print(f"   ... ещё {len(group) - 15}")

    if json_path:
        json_path.write_text(json.dumps(rows, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"JSON -> {json_path}")


def pbl_files(paths):
    """Каталог — сортированный обход; этот же порядок у run_corpus.py."""
    files = []
    for p in paths:
        files += sorted(p.glob("*.pbl")) if p.is_dir() else [p]
    return files


TASK = re.compile(r"^task\b(?:[^\n]*\}\n|.*?^\}\n)", re.M | re.S)
TASK_ID = re.compile(r'^\s*id "([^"]+)";', re.M)


def pbl_tasks(files):
    """[(id, блок task {…})] в порядке прогона; без `id` — `каталог/файл#номер`."""
    out = []
    for f in files:
        for n, block in enumerate(TASK.findall(f.read_text(encoding="utf-8"))):
            m = TASK_ID.search(block)
            out.append((m.group(1) if m else f"{f.parent.name}/{f.name}#{n}", block))
    return out


def pbl_ids(files):
    return [i for i, _ in pbl_tasks(files)]


ANSI = re.compile(r"\x1b\[[0-9;]*m")

# Вердикт — последняя такая строка чанка: подзадачи печатают свои раньше.
VERDICT = re.compile(r"^(PROVEN!|NOT SOLVED!|Answer: .*?) \[(\d+) cycles\]$", re.M)


def log_records(text):
    """Лог CLI -> [{verdict, cycles, obtained}] в порядке обработки."""
    chunks = re.split(r"^Task ", ANSI.sub("", text), flags=re.M)[1:]
    records = []
    for c in chunks:
        verdicts = VERDICT.findall(c)
        if not verdicts:
            # Оборванный чанк тоже даёт запись: main сверяет длину со списком .pbl-id.
            records.append({"verdict": "no_verdict", "cycles": None})
            continue
        last, cycles = verdicts[-1]
        cycles = int(cycles)
        if "Answer changed:" in c:
            om = re.search(r"^Obtained: (.*)$", c, re.M)
            records.append(
                {
                    "verdict": "wrong_answer",
                    "cycles": cycles,
                    "obtained": om.group(1) if om else None,
                }
            )
        elif last == "NOT SOLVED!":
            records.append({"verdict": "not_solved", "cycles": cycles})
        else:
            records.append({"verdict": "solved", "cycles": cycles})
    return records


if __name__ == "__main__":
    main()
