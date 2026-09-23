from __future__ import annotations

from typing import Any

import numpy as np
from sklearn.metrics import (
    average_precision_score,
    f1_score,
    precision_score,
    recall_score,
    roc_auc_score,
)


def evaluate_model(
    model: Any,
    X: Any,
    y: Any,
) -> dict[str, float | None]:

    predictions = model.predict(X)
    probabilities = model.predict_proba(X)[:, 1]

    metrics: dict[str, float | None] = {
        "precision": float(
            precision_score(y, predictions, zero_division=0)
        ),
        "recall": float(
            recall_score(y, predictions, zero_division=0)
        ),
        "f1": float(
            f1_score(y, predictions, zero_division=0)
        ),
        "roc_auc": _safe_auc(roc_auc_score, y, probabilities),
        "pr_auc": _safe_auc(average_precision_score, y, probabilities),
    }

    return metrics


def _safe_auc(metric: Any, y: Any, probabilities: Any) -> float | None:
    """Return None when the evaluation set contains only one class."""
    if np.unique(y).size < 2:
        return None

    return float(metric(y, probabilities))