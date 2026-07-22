#!/usr/bin/env python3
"""Карта вердиктов прогона CLI по импортированному корпусу.

Сопоставляет записи лога CLI (в порядке обработки) с задачами .pbl
(сортированный обход каталога + порядок в файле — тот же порядок)
и печатает раскладку по вердиктам с иходными id Искры.

Использование:
    python scripts/run_report.py <run.log> <tasks_dir> [--json out.json]
"""

import argparse
import json
import re
import sys
from pathlib import Path


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("log", type=Path)
    ap.add_argument("tasks_dir", type=Path)
    ap.add_argument("--json", type=Path, help="полный отчёт в JSON")
    return ap.parse_args()


def main():
    args = parse_args()
    ids = pbl_ids(args.tasks_dir)
    records = log_records(args.log.read_text(encoding="utf-8"))
    if len(ids) != len(records):
        sys.exit(f"рассинхрон: {len(ids)} задач в .pbl, {len(records)} записей в логе")

    rows = [{"id": i, **r} for i, r in zip(ids, records)]
    by_verdict = {}
    for r in rows:
        by_verdict.setdefault(r["verdict"], []).append(r)

    for verdict in ("solved", "wrong_answer", "not_solved"):
        group = by_verdict.get(verdict, [])
        print(f"== {verdict}: {len(group)}")
        for r in group if verdict == "solved" else group[:15]:
            extra = f"  получено: {r['obtained']}" if r.get("obtained") else ""
            print(f"   {r['id']}  [{r['cycles']} cycles]{extra}")
        if verdict != "solved" and len(group) > 15:
            print(f"   ... ещё {len(group) - 15}")

    if args.json:
        args.json.write_text(
            json.dumps(rows, ensure_ascii=False, indent=1), encoding="utf-8"
        )
        print(f"JSON -> {args.json}")


def pbl_ids(tasks_dir):
    ids = []
    for f in sorted(tasks_dir.glob("*.pbl")):
        for m in re.finditer(r'^\s*id "([^"]+)";', f.read_text(encoding="utf-8"), re.M):
            ids.append(m.group(1))
    return ids


def log_records(text):
    """Лог CLI -> [{verdict, cycles, obtained}] в порядке обработки."""
    chunks = re.split(r"^Task ", text, flags=re.M)[1:]
    records = []
    for c in chunks:
        cycles = None
        m = re.search(r"\[(\d+) cycles\]", c)
        if m:
            cycles = int(m.group(1))
        if "Answer changed:" in c:
            om = re.search(r"^Obtained: (.*)$", c, re.M)
            records.append(
                {
                    "verdict": "wrong_answer",
                    "cycles": cycles,
                    "obtained": om.group(1) if om else None,
                }
            )
        elif "NOT SOLVED!" in c or "NoSolutionsFound" in c or "not done" in c:
            records.append({"verdict": "not_solved", "cycles": cycles})
        else:
            records.append({"verdict": "solved", "cycles": cycles})
    return records


if __name__ == "__main__":
    main()
