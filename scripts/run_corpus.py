#!/usr/bin/env python3
"""Прогон корпуса .pbl по одной задаче, каждая под внешним таймаутом.

Общий прогон виснет на одной задаче: лимит времени решателя проверяется
раз в цикл (беклог §4). Здесь каждая задача — отдельный запуск cli.
cli запускается последовательно: параллельные делят logs/cli.log (беклог §5).

Пишет в --out: split/NNN/t.pbl, run.log (для run_report.py),
times.tsv (задача, секунды, код выхода), slow/NNN.out (вывод убитых).

Использование:
    python scripts/run_corpus.py tasks/iskra/eq_32.pbl --out /tmp/eq [--cli /tmp/cli]
"""

import argparse
import re
import subprocess
import sys
import time
from pathlib import Path

from run_report import log_records, pbl_files, pbl_tasks, report

REPO = Path(__file__).resolve().parents[1]


def parse_args():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("pbl", type=Path, nargs="+", help=".pbl-файлы или каталоги")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--timeout", type=int, default=40, help="внешний таймаут, с")
    ap.add_argument("--limit", type=int, default=30, help="лимит решателя -l, с")
    ap.add_argument("--json", type=Path, help="полный отчёт в JSON")
    ap.add_argument(
        "--cli",
        type=Path,
        default=REPO / "target" / "release" / "cli",
        help="бинарник cli; копия не даёт пересборке испортить долгий прогон",
    )
    return ap.parse_args()


def main():
    args = parse_args()
    if not args.cli.exists():
        hint = ": cargo build --release" if args.cli == REPO / "target" / "release" / "cli" else ""
        sys.exit(f"нет {args.cli}{hint}")

    tasks = pbl_tasks(pbl_files(args.pbl))
    dirs = split(tasks, args.out / "split")
    (args.out / "slow").mkdir(parents=True, exist_ok=True)

    log, times = [], ["task\tsecs\texit"]
    for i, d in enumerate(dirs, 1):
        secs, code, out = run_one(args.cli, d, args.timeout, args.limit)
        times.append(f"{d.name}\t{secs:.1f}\t{code}")
        if code != 0:
            (args.out / "slow" / f"{d.name}.out").write_text(out, encoding="utf-8")
        # убитая задача могла не успеть напечатать заголовок — запись нужна всё равно
        if not re.search(r"^Task ", out, re.M):
            out = f"Task {d.name}\n{out}"
        log.append(out)
        print(f"\r{i}/{len(dirs)}", end="", file=sys.stderr, flush=True)
    print(file=sys.stderr)

    (args.out / "run.log").write_text("\n".join(log), encoding="utf-8")
    (args.out / "times.tsv").write_text("\n".join(times) + "\n", encoding="utf-8")
    report([i for i, _ in tasks], log_records("\n".join(log)), args.json)


def split(tasks, root):
    dirs = []
    for n, (_, block) in enumerate(tasks):
        d = root / f"{n:03d}"
        d.mkdir(parents=True, exist_ok=True)
        (d / "t.pbl").write_text(block, encoding="utf-8")
        dirs.append(d)
    return dirs


def run_one(cli, task_dir, timeout, limit):
    cmd = [str(cli), "-c", "config/cli.yaml", "-p", str(task_dir), "-l", str(limit)]
    start = time.monotonic()
    try:
        p = subprocess.run(
            cmd, cwd=REPO, capture_output=True, text=True, timeout=timeout
        )
        code, out = p.returncode, p.stdout + p.stderr
    except subprocess.TimeoutExpired as e:
        code, out = -9, decode(e.stdout) + decode(e.stderr)
    return time.monotonic() - start, code, out


def decode(b):
    if b is None:
        return ""
    return b if isinstance(b, str) else b.decode("utf-8", "replace")


if __name__ == "__main__":
    main()
