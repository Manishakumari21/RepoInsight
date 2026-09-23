from __future__ import annotations

from pathlib import Path
from typing import Any

from sklearn.linear_model import LogisticRegression
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler

from .dataset import load_jsonl, split_chronologically
from .evaluate import evaluate_model
from .features import rows_to_features
from .preprocessing import build_imputer


def build_baseline_model() -> Pipeline:
    """Build the Phase 6.1 Logistic Regression baseline."""

    return Pipeline(
        [
            ("imputer", build_imputer()),
            ("scaler", StandardScaler()),
            (
                "classifier",
                LogisticRegression(
                    max_iter=1000,
                    class_weight="balanced",
                    random_state=42,
                ),
            ),
        ]
    )


def train_baseline(dataset_path: str | Path) -> dict[str, Any]:

    rows = load_jsonl(dataset_path)
    split = split_chronologically(rows)

    if not split.train:
        raise ValueError("Training split is empty")

    if not split.test:
        raise ValueError("Test split is empty")

    X_train, y_train = rows_to_features(split.train)
    X_test, y_test = rows_to_features(split.test)

    model = build_baseline_model()
    model.fit(X_train, y_train)

    metrics = evaluate_model(model, X_test, y_test)

    return {
        "model": model,
        "train_rows": len(split.train),
        "validation_rows": len(split.validation),
        "test_rows": len(split.test),
        "metrics": metrics,
    }