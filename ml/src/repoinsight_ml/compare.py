from __future__ import annotations

from pathlib import Path
from typing import Any

from sklearn.ensemble import HistGradientBoostingClassifier, RandomForestClassifier
from sklearn.pipeline import Pipeline

from .dataset import load_jsonl, split_chronologically
from .evaluate import evaluate_model
from .features import rows_to_features
from .preprocessing import build_imputer
from .train import build_baseline_model


def build_candidate_models() -> dict[str, Pipeline]:
    return {
        "logistic_regression": build_baseline_model(),
        "random_forest": Pipeline(
            [
                ("imputer", build_imputer()),
                (
                    "classifier",
                    RandomForestClassifier(
                        n_estimators=200,
                        class_weight="balanced",
                        random_state=42,
                        n_jobs=-1,
                    ),
                ),
            ]
        ),
        "hist_gradient_boosting": Pipeline(
            [
                ("imputer", build_imputer()),
                (
                    "classifier",
                    HistGradientBoostingClassifier(
                        max_iter=200,
                        class_weight="balanced",
                        random_state=42,
                    ),
                ),
            ]
        ),
    }


def compare_models(dataset_path: str | Path) -> list[dict[str, Any]]:
    rows = load_jsonl(dataset_path)
    split = split_chronologically(rows)

    if not split.train:
        raise ValueError("Training split is empty")

    if not split.test:
        raise ValueError("Test split is empty")

    X_train, y_train = rows_to_features(split.train)
    X_test, y_test = rows_to_features(split.test)

    results: list[dict[str, Any]] = []
    for name, model in build_candidate_models().items():
        model.fit(X_train, y_train)
        metrics = evaluate_model(model, X_test, y_test)
        results.append(
            {
                "model": name,
                "precision": metrics["precision"],
                "recall": metrics["recall"],
                "f1": metrics["f1"],
                "roc_auc": metrics["roc_auc"],
                "pr_auc": metrics["pr_auc"],
            }
        )

    return results
