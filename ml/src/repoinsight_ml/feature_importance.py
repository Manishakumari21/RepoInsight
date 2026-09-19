from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np
from sklearn.inspection import permutation_importance

from .compare import build_candidate_models
from .dataset import load_jsonl, split_chronologically
from .features import FEATURE_COLUMNS, rows_to_features


PERMUTATION_SCORING = "accuracy"
PERMUTATION_REPEATS = 5
PERMUTATION_RANDOM_STATE = 42


def evaluate_feature_importance(
    rows: list[dict[str, Any]],
    n_repeats: int = PERMUTATION_REPEATS,
    random_state: int = PERMUTATION_RANDOM_STATE,
) -> list[dict[str, Any]]:
    """Compute feature importance on validation data.

    Models fit on chronological train only; importance is measured on
    validation (test untouched, no refit). Native importances operate on
    pipeline-transformed features, which preserve FEATURE_COLUMNS order
    (imputation keeps 23 columns, scaler preserves order).
    """
    split = split_chronologically(rows)

    if not split.train:
        raise ValueError("Training split is empty")
    if not split.validation:
        raise ValueError("Validation split is empty")

    X_train, y_train = rows_to_features(split.train)
    X_val, y_val = rows_to_features(split.validation)

    results: list[dict[str, Any]] = []
    for name, model in build_candidate_models().items():
        model.fit(X_train, y_train)

        if name == "logistic_regression":
            coefs = np.asarray(
                model.named_steps["classifier"].coef_[0], dtype=float
            )
            for feature, value in zip(FEATURE_COLUMNS, coefs):
                results.append(
                    {
                        "model": name,
                        "feature": feature,
                        "importance": float(abs(value)),
                        "method": "coefficient_abs",
                    }
                )
        elif name == "random_forest":
            importances = np.asarray(
                model.named_steps["classifier"].feature_importances_,
                dtype=float,
            )
            for feature, value in zip(FEATURE_COLUMNS, importances):
                results.append(
                    {
                        "model": name,
                        "feature": feature,
                        "importance": float(value),
                        "method": "impurity",
                    }
                )

        perm = permutation_importance(
            model,
            X_val,
            y_val,
            n_repeats=n_repeats,
            random_state=random_state,
            scoring=PERMUTATION_SCORING,
        )
        for feature, value in zip(
            FEATURE_COLUMNS, np.asarray(perm.importances_mean, dtype=float)
        ):
            results.append(
                {
                    "model": name,
                    "feature": feature,
                    "importance": float(value),
                    "method": "permutation",
                }
            )

    return results


def top_features(
    results: list[dict[str, Any]],
    model: str,
    n: int = 10,
    method: str | None = None,
) -> list[dict[str, Any]]:
    """Return the top-N features for a model, sorted by importance."""
    filtered = [r for r in results if r["model"] == model]
    if method is not None:
        filtered = [r for r in filtered if r["method"] == method]
    return sorted(filtered, key=lambda r: r["importance"], reverse=True)[:n]


def evaluate_feature_importance_from_path(
    dataset_path: str | Path,
    n_repeats: int = PERMUTATION_REPEATS,
    random_state: int = PERMUTATION_RANDOM_STATE,
) -> list[dict[str, Any]]:
    """Load JSONL then compute feature importance."""
    return evaluate_feature_importance(
        load_jsonl(dataset_path),
        n_repeats=n_repeats,
        random_state=random_state,
    )
