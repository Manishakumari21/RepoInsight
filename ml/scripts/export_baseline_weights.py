"""Export the Phase 6 logistic-regression baseline as a JSON weights file.

The Rust backend runs the identical math (median imputation with zero
fallback, standard scaling, sigmoid of the linear score) so the
dashboard can serve real ML predictions without a Python runtime.
Re-run this script if the baseline hyperparameters or FEATURE_COLUMNS
change, and keep the Rust parity test in sync.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ML_SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(ML_SRC))

from repoinsight_ml.compare import build_candidate_models  # noqa: E402
from repoinsight_ml.dataset import load_jsonl, split_chronologically  # noqa: E402
from repoinsight_ml.features import FEATURE_COLUMNS, rows_to_features  # noqa: E402

DATASET = Path(__file__).resolve().parents[1] / "data" / "dataset.jsonl"
OUT = (
    Path(__file__).resolve().parents[2]
    / "backend"
    / "src"
    / "prediction"
    / "weights.json"
)


def main() -> None:
    rows = load_jsonl(DATASET)
    split = split_chronologically(rows)
    X_train, y_train = rows_to_features(split.train)

    model = build_candidate_models()["logistic_regression"]
    model.fit(X_train, y_train)

    imputer = model.named_steps["imputer"]
    median = imputer.named_steps["median"]
    scaler = model.named_steps["scaler"]
    classifier = model.named_steps["classifier"]

    payload = {
        "model_name": "logistic_regression",
        "feature_order": list(FEATURE_COLUMNS),
        "median": [None if v != v else float(v) for v in median.statistics_],
        "scaler_mean": [float(v) for v in scaler.mean_],
        "scaler_scale": [float(v) for v in scaler.scale_],
        "coefficients": [float(v) for v in classifier.coef_[0]],
        "intercept": float(classifier.intercept_[0]),
        "threshold": 0.5,
        "calibrated": False,
        "provenance": {
            "dataset": "ml/data/dataset.jsonl",
            "train_rows": len(split.train),
            "train_positives": int(y_train.sum()),
            "generator": "ml/scripts/export_baseline_weights.py",
        },
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT} ({len(split.train)} train rows)")


if __name__ == "__main__":
    main()
