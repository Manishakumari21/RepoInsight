"""Phase 9.8 ablation runner (real datasets only).

Trains the logistic-regression baseline on each feature-group
configuration (all / single groups / pairs) using the exact same
chronological split per repository, then records precision/recall/F1,
ROC-AUC/PR-AUC and confusion counts. No ranking is applied.

Usage:
    python ml/scripts/run_ablation.py \\
        --repo repoinsight ml/data/dataset.jsonl \\
        --repo repo-ranger ml/data/repo-ranger.jsonl \\
        --out ml/ablation_results.json [--envelope]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

ML_SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(ML_SRC))

from repoinsight_ml.ablation import run_ablation  # noqa: E402
from repoinsight_ml.dataset import load_jsonl, split_chronologically  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
from benchmark import label_counts, relative_dataset_ref, run_metadata  # noqa: E402


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo",
        nargs=2,
        metavar=("NAME", "DATASET_JSONL"),
        action="append",
        required=True,
    )
    parser.add_argument("--out", required=True)
    parser.add_argument("--envelope", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    records: list[dict] = []
    for name, dataset in args.repo:
        t0 = time.perf_counter()
        rows = load_jsonl(dataset)
        split = split_chronologically(rows)
        if not split.train or not split.test:
            raise ValueError(f"{name}: empty train or test split")
        test_pos, test_neg = label_counts(split.test)
        for result in run_ablation(split):
            records.append(
                {
                    "repository": name,
                    "dataset": relative_dataset_ref(Path(dataset)),
                    "dataset_rows": len(rows),
                    "test_rows": len(split.test),
                    "test_positives": test_pos,
                    "test_negatives": test_neg,
                    **result,
                    "elapsed_seconds": round(time.perf_counter() - t0, 3),
                }
            )
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    if args.envelope:
        payload: dict = {
            "metadata": run_metadata(
                [(n, d, "") for n, d in args.repo]
            ),
            "records": records,
        }
    else:
        payload = records  # type: ignore[assignment]
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(records)} ablation records to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
