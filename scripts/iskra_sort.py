#!/usr/bin/env python3
"""Переносит решённые задачи Искры из tasks/unsolved/iskra по прогону run_corpus.py.

Решённая за <= 3 с уходит в tasks/regress/iskra, дольше — в tasks/slow/iskra
(порог наборов — CLAUDE.md), в файл подраздела с тем же именем.

Использование:
    python scripts/iskra_sort.py /tmp/run_prove /tmp/run_eq [--dry-run]
"""

import argparse
import re
from pathlib import Path

from run_report import TASK, TASK_ID, log_records

REPO = Path(__file__).resolve().parents[1]
UNSOLVED = REPO / "tasks" / "unsolved" / "iskra"
FAST_SECS = 3.0


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("runs", type=Path, nargs="+", help="каталоги --out прогона run_corpus.py")
    ap.add_argument("--dry-run", action="store_true", help="только показать")
    return ap.parse_args()


def main():
    args = parse_args()
    solved = {}
    for run in args.runs:
        solved.update(solved_in(run))

    moved = 0
    for src in sorted(UNSOLVED.glob("*.pbl")):
        text = src.read_text(encoding="utf-8")
        keep = []
        for block in TASK.findall(text):
            task_id = TASK_ID.search(block).group(1)
            if task_id not in solved:
                keep.append(block)
                continue
            dest = REPO / "tasks" / ("regress" if solved[task_id] <= FAST_SECS else "slow") / "iskra"
            print(f"{task_id} -> {dest.name}/{src.name} ({solved[task_id]:.1f} s)")
            moved += 1
            if not args.dry_run:
                append(dest / src.name, header(text), block)
        if not args.dry_run and len(keep) < len(TASK.findall(text)):
            if keep:
                src.write_text(header(text) + "\n".join(keep) + "\n", encoding="utf-8")
            else:
                src.unlink()
    print(f"перенесено {moved}")


def solved_in(run):
    """{id: секунды} решённых задач прогона: порядок split/NNN — порядок run.log."""
    dirs = sorted((run / "split").iterdir())
    records = log_records((run / "run.log").read_text(encoding="utf-8"))
    secs = {}
    for line in (run / "times.tsv").read_text(encoding="utf-8").splitlines()[1:]:
        name, s, _ = line.split("\t")
        secs[name] = float(s)
    out = {}
    for d, r in zip(dirs, records, strict=True):
        if r["verdict"] == "solved":
            out[TASK_ID.search((d / "t.pbl").read_text(encoding="utf-8")).group(1)] = secs[d.name]
    return out


def header(text):
    """Комментарий-заголовок файла подраздела (название раздела)."""
    m = re.match(r"(//[^\n]*\n)+\n?", text)
    return m.group(0) if m else ""


def append(path, head, block):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        path.write_text(path.read_text(encoding="utf-8").rstrip("\n") + "\n\n" + block, encoding="utf-8")
    else:
        path.write_text(head + block, encoding="utf-8")


if __name__ == "__main__":
    main()
