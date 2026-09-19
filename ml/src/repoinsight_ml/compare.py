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
    """Build Phase 6.2 CPU-friendly candidates plus the Phase 6.1 baseline.

    Tree models use imputation only (no scaling). Hyperparameters are
    untuned defaults kept small for an 8 GB laptop. Random states are
    fixed for determinism.
    """
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
    """Fit each candidate on the same chronological train split.

    Returns one dict per model with model name and comparable metrics.
    No ranking is applied here.
    """
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
