"""Phase 9.7 model comparison summary (no ranking).

Reads a benchmark output file (bare list or ``--envelope`` dict) and
writes one compact record per repository/model pair. This is a research
comparison, not a leaderboard: repositories have different class
distributions, so metrics must be interpreted in context — see the
``interpretation`` note embedded in the output.

Usage:
    python ml/scripts/run_model_comparison.py \\
        --in ml/multi_repo_benchmark.json \\
        --out ml/model_comparison.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

COMPARISON_KEYS = (
    "repository",
    "model",
    "precision",
    "recall",
    "f1",
    "roc_auc",
    "pr_auc",
    "prediction_time_seconds",
)

INTERPRETATION = (
    "Repositories are evaluated independently on chronological test "
    "splits with different class distributions (see test_positive_rate "
    "in the source benchmark). Precision/recall/F1 and especially "
    "PR-AUC must be interpreted in that context; no overall winner is "
    "declared. Accuracy is not reported: it is misleading under class "
    "imbalance."
)


def load_records(path: Path) -> tuple[list[dict], dict]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(payload, dict):
        return payload["records"], payload.get("metadata", {})
    return payload, {}


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--in", dest="source", required=True)
    parser.add_argument("--out", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    records, metadata = load_records(Path(args.source))
    comparison = [
        {key: record.get(key) for key in COMPARISON_KEYS} for record in records
    ]
    payload = {
        "metadata": metadata,
        "interpretation": INTERPRETATION,
        "records": comparison,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(comparison)} model-comparison records to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
