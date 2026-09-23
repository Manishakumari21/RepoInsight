"""Evaluate the Change Impact Simulator on a real git repository.

Builds leakage-safe dataset rows (one per commit/file) plus commit
messages straight from git history, then compares impact baselines:

* keyword-only (filename matching),
* co-change-only (historical coupling),
* combined (semantic + historical + path-kind).

Chronological discipline: every test target (last fifth of commits) is
scored from rows strictly before its timestamp; the commit message stands
in for the planned-change description.

Usage:
    .venv/bin/python ml/scripts/run_impact_evaluation.py \
        --git-repo /path/to/repo --k 5 --out ml/impact_results.json

Research question: can repository structure and history estimate the
impact of a planned change before implementation?
"""

from __future__ import annotations

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

ML_SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(ML_SRC))

from repoinsight_ml.impact import compare_impact_models


def load_git_history(repo: Path, max_commits: int) -> tuple[list[dict], dict[str, str]]:
    """Extract (commit_sha, timestamp, message, files) from git log.

    With ``-z``, both records and filenames are NUL-separated, so parsing
    is stateful: a chunk starting with a 40-hex SHA opens a new commit
    (its first line is the header, an optional remainder is the first
    filename); every other non-empty chunk is a filename.
    """
    log = subprocess.run(
        ["git", "log", f"--max-count={max_commits}",
         "--pretty=format:%H%x1f%ct%x1f%s", "--name-only", "-z"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
    )
    rows: list[dict] = []
    messages: dict[str, str] = {}
    current_sha: str | None = None
    current_stamp: int | None = None

    def emit(path: str) -> None:
        cleaned = path.strip().strip('"')
        if not cleaned or current_sha is None or current_stamp is None:
            return
        rows.append(
            {
                "commit_sha": current_sha,
                "timestamp": current_stamp,
                "file_path": cleaned,
                "structural": {},
                "historical": {},
                "temporal": {},
                "label": 0,
            }
        )

    for chunk in log.stdout.split("\0"):
        if not chunk.strip():
            continue
        head, sep, rest = chunk.partition("\n")
        parts = head.split("\x1f")
        if len(parts) == 3 and re.fullmatch(r"[0-9a-f]{40}", parts[0]):
            current_sha, current_stamp = parts[0], int(parts[1])
            messages[current_sha] = parts[2]
            if sep and rest.strip():
                emit(rest)
        else:
            emit(chunk)
    return rows, messages


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--git-repo", required=True)
    parser.add_argument("--max-commits", type=int, default=1000)
    parser.add_argument("--k", type=int, default=5)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)

    repo = Path(args.git_repo)
    rows, messages = load_git_history(repo, args.max_commits)
    results = compare_impact_models(rows, messages, k=args.k)
    payload = {
        "metadata": {
            "generated_utc": datetime.datetime.now(
                datetime.timezone.utc
            ).isoformat(),
            "repository": str(repo),
            "commits": len(messages),
            "rows": len(rows),
            "k": args.k,
            "note": (
                "Commit messages stand in for planned-change "
                "descriptions; every target is scored from history "
                "strictly before its timestamp. No ranking is applied "
                "between baselines."
            ),
        },
        "results": results,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(
        f"Wrote {len(results)} impact records "
        f"({len(messages)} commits, {len(rows)} rows) to {out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
