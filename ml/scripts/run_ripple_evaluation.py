"""Evaluate Change Ripple Forecasting: co-change baseline vs full model.

Usage:
    .venv/bin/python ml/scripts/run_ripple_evaluation.py \
        --dataset ml/data/dataset.jsonl --k 5 [--output FILE]

Chronological ranking evaluation (P@K, R@K, MRR, MAP); no training.
Research question: does temporal change history add predictive information
beyond simple historical co-change?
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ML_SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(ML_SRC))

from repoinsight_ml.dataset import load_jsonl
from repoinsight_ml.ripple import compare_ripple_models


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset", default=None)
    parser.add_argument(
        "--repo",
        nargs=2,
        metavar=("NAME", "DATASET_JSONL"),
        action="append",
        default=None,
    )
    parser.add_argument("--k", type=int, default=5)
    parser.add_argument("--output", default=None)
    parser.add_argument("--out", default=None)
    args = parser.parse_args()

    targets: list[tuple[str, str]] = []
    if args.repo:
        targets.extend(args.repo)
    if args.dataset:
        targets.append((Path(args.dataset).stem, args.dataset))
    if not targets:
        parser.error("provide --dataset and/or --repo NAME DATASET_JSONL")

    results = []
    for name, dataset in targets:
        rows = load_jsonl(dataset)
        results.append(
            {
                "repository": name,
                "dataset": dataset,
                "dataset_rows": len(rows),
                "k": args.k,
                "results": compare_ripple_models(rows, k=args.k),
            }
        )
    out = args.out or args.output
    if len(targets) == 1 and out is None:
        payload: dict = {
            "dataset": targets[0][1],
            "k": args.k,
            "results": results[0]["results"],
        }
        print(json.dumps(payload, indent=2))
        return
    if out is None:
        parser.error("multi-repo runs require --out")
    payload = {"results": results}
    Path(out).write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote ripple evaluation for {len(results)} datasets to {out}")


if __name__ == "__main__":
    main()
