from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np
from sklearn.calibration import CalibratedClassifierCV
from sklearn.frozen import FrozenEstimator
from sklearn.metrics import (
    average_precision_score,
    brier_score_loss,
    roc_auc_score,
)

from .compare import build_candidate_models
from .dataset import load_jsonl, split_chronologically
from .features import rows_to_features


def _safe_score(metric: Any, y: Any, probabilities: Any) -> float | None:
    """Return None when the evaluation set contains only one class."""
    if np.unique(y).size < 2:
        return None
    return float(metric(y, probabilities))


def evaluate_calibration(
    rows: list[dict[str, Any]],
    method: str = "sigmoid",
) -> list[dict[str, Any]]:
    split = split_chronologically(rows)

    if not split.train:
        raise ValueError("Training split is empty")
    if not split.validation:
        raise ValueError("Validation split is empty")
    if not split.test:
        raise ValueError("Test split is empty")

    X_train, y_train = rows_to_features(split.train)
    X_val, y_val = rows_to_features(split.validation)
    X_test, y_test = rows_to_features(split.test)

    if np.unique(y_val).size < 2:
        raise ValueError("Validation split needs both classes for calibration")
    if len(y_val) < 2:
        raise ValueError("Validation split is too small for calibration")

    y_test_arr = np.asarray(y_test)

    results: list[dict[str, Any]] = []
    for name, base in build_candidate_models().items():
        base.fit(X_train, y_train)

        calibrator = CalibratedClassifierCV(
            estimator=FrozenEstimator(base),
            method=method,
            cv=2,
        )
        calibrator.fit(X_val, y_val)

        prob_uncal = np.asarray(base.predict_proba(X_test)[:, 1])
        prob_cal = np.asarray(calibrator.predict_proba(X_test)[:, 1])

        results.append(
            {
                "model": name,
                "roc_auc_uncalibrated": _safe_score(
                    roc_auc_score, y_test_arr, prob_uncal
                ),
                "roc_auc_calibrated": _safe_score(
                    roc_auc_score, y_test_arr, prob_cal
                ),
                "pr_auc_uncalibrated": _safe_score(
                    average_precision_score, y_test_arr, prob_uncal
                ),
                "pr_auc_calibrated": _safe_score(
                    average_precision_score, y_test_arr, prob_cal
                ),
                "brier_uncalibrated": float(
                    brier_score_loss(y_test_arr, prob_uncal)
                ),
                "brier_calibrated": float(
                    brier_score_loss(y_test_arr, prob_cal)
                ),
            }
        )

    return results


def evaluate_calibration_from_path(
    dataset_path: str | Path,
    method: str = "sigmoid",
) -> list[dict[str, Any]]:
    return evaluate_calibration(load_jsonl(dataset_path), method=method)
