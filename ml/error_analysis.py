"""Phase 9.9 error analysis (real datasets only).

Trains each candidate model on the chronological train split, predicts
the test split, and reports observed false-positive / false-negative
patterns with dataset context. Language is associational ("observed
in", "associated with") — no causal claims.

Usage:
    python ml/error_analysis.py \\
        --repo repoinsight ml/data/dataset.jsonl \\
        --repo repo-ranger ml/data/repo-ranger.jsonl \\
        --out ml/error_analysis.json [--models logistic_regression]
"""

from __future__ import annotations

import argparse
import datetime
import json
import sys
from pathlib import Path

import numpy
import sklearn

ML_SRC = Path(__file__).resolve().parent / "src"
sys.path.insert(0, str(ML_SRC))
sys.path.insert(0, str(ML_SRC.parent / "scripts"))

from repoinsight_ml.ablation import FEATURE_GROUPS  # noqa: E402
from repoinsight_ml.compare import build_candidate_models  # noqa: E402
from repoinsight_ml.dataset import load_jsonl, split_chronologically  # noqa: E402
from repoinsight_ml.features import FEATURE_COLUMNS, rows_to_features  # noqa: E402

MAX_EXAMPLES_PER_CLASS = 25


def group_means(frame, columns: list[str]) -> dict[str, float]:
    return {c: round(float(frame[c].mean()), 4) for c in columns}


def analyze_repo(name: str, dataset: str, models: list[str]) -> list[dict]:
    rows = load_jsonl(dataset)
    split = split_chronologically(rows)
    if not split.train or not split.test:
        raise ValueError(f"{name}: empty train or test split")
    X_train, y_train = rows_to_features(split.train)
    X_test, y_test = rows_to_features(split.test)
    test_rows = split.test
    test_pos = int((y_test == 1).sum())
    test_neg = int((y_test == 0).sum())

    candidates = build_candidate_models()
    unknown = [m for m in models if m not in candidates]
    if unknown:
        raise ValueError(f"unknown models: {unknown}")

    results: list[dict] = []
    for model_name in models:
        model = candidates[model_name]
        model.fit(X_train, y_train)
        proba = model.predict_proba(X_test)[:, 1]
        pred = model.predict(X_test)

        fp_idx = [i for i, (a, p) in enumerate(zip(y_test, pred)) if a == 0 and p == 1]
        fn_idx = [i for i, (a, p) in enumerate(zip(y_test, pred)) if a == 1 and p == 0]
        tn = int(((y_test == 0) & (pred == 0)).sum())
        tp = int(((y_test == 1) & (pred == 1)).sum())

        def example(i: int) -> dict:
            row = test_rows[i]
            flat = {c: float(X_test.iloc[i][c]) for c in FEATURE_COLUMNS}
            return {
                "commit_sha": str(row["commit_sha"]),
                "timestamp": int(row["timestamp"]),
                "file_path": str(row["file_path"]),
                "predicted_probability": round(float(proba[i]), 4),
                "actual_label": int(y_test.iloc[i]),
                "predicted_label": int(pred[i]),
                "features": flat,
            }

        fp_frame = X_test.iloc[fp_idx] if fp_idx else None
        fn_frame = X_test.iloc[fn_idx] if fn_idx else None
        results.append(
            {
                "repository": name,
                "dataset": dataset,
                "model": model_name,
                "test_rows": len(split.test),
                "test_positives": test_pos,
                "test_negatives": test_neg,
                "test_positive_rate": test_pos / len(split.test) if split.test else 0.0,
                "false_positive_count": len(fp_idx),
                "false_negative_count": len(fn_idx),
                "false_positive_rate": len(fp_idx) / (len(fp_idx) + tn)
                if (len(fp_idx) + tn)
                else 0.0,
                "false_negative_rate": len(fn_idx) / (len(fn_idx) + tp)
                if (len(fn_idx) + tp)
                else 0.0,
                "false_positives": [example(i) for i in fp_idx[:MAX_EXAMPLES_PER_CLASS]],
                "false_negatives": [example(i) for i in fn_idx[:MAX_EXAMPLES_PER_CLASS]],
                "observed_fp_feature_means": {
                    group: group_means(fp_frame, cols)
                    for group, cols in FEATURE_GROUPS.items()
                }
                if fp_frame is not None
                else {},
                "observed_fn_feature_means": {
                    group: group_means(fn_frame, cols)
                    for group, cols in FEATURE_GROUPS.items()
                }
                if fn_frame is not None
                else {},
            }
        )
    return results


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo", nargs=2, metavar=("NAME", "DATASET_JSONL"),
        action="append", required=True,
    )
    parser.add_argument(
        "--models", nargs="+", default=None,
        help="Subset of models (default: all candidates).",
    )
    parser.add_argument("--out", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    records: list[dict] = []
    for name, dataset in args.repo:
        records.extend(
            analyze_repo(name, dataset, args.models or list(build_candidate_models()))
        )
    payload = {
        "metadata": {
            "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "python": sys.version.split()[0],
            "sklearn": sklearn.__version__,
            "numpy": numpy.__version__,
            "note": (
                "FP/FN patterns are observed associations in the "
                "chronological test split, not causal explanations."
            ),
        },
        "records": records,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(records)} error-analysis records to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
