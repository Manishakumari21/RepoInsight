"""Phase 9.8 feature-group ablation (existing features only).

Uses the canonical ``FEATURE_COLUMNS`` order from ``features.py`` and the
project's chronological split. The same train/test row sets back every
configuration, so differences reflect the feature subset — never a
different data split, and never future information (column selection
cannot introduce rows outside the already leakage-safe split).
"""

from __future__ import annotations

from typing import Any

from sklearn.metrics import confusion_matrix

from .dataset import DatasetSplit
from .evaluate import evaluate_model
from .features import FEATURE_COLUMNS, rows_to_features
from .train import build_baseline_model


FEATURE_GROUPS: dict[str, list[str]] = {
    "structural": [c for c in FEATURE_COLUMNS[:9]],
    "historical": [c for c in FEATURE_COLUMNS[9:15]],
    "temporal": [c for c in FEATURE_COLUMNS[15:]],
}

for _group, _columns in FEATURE_GROUPS.items():
    missing = [c for c in _columns if c not in FEATURE_COLUMNS]
    if missing:
        raise ValueError(f"group {_group} has unknown columns: {missing}")

ABLATON_CONFIGS: list[tuple[str, list[str]]] = [
    ("all", ["structural", "historical", "temporal"]),
    ("structural-only", ["structural"]),
    ("historical-only", ["historical"]),
    ("temporal-only", ["temporal"]),
    ("structural+historical", ["structural", "historical"]),
    ("structural+temporal", ["structural", "temporal"]),
    ("historical+temporal", ["historical", "temporal"]),
]


def columns_for(groups: list[str]) -> list[str]:
    """Canonical columns for feature groups, in canonical order."""
    wanted: set[str] = set()
    for group in groups:
        if group not in FEATURE_GROUPS:
            raise KeyError(f"unknown feature group: {group}")
        wanted.update(FEATURE_GROUPS[group])
    return [c for c in FEATURE_COLUMNS if c in wanted]


def run_ablation(
    split: DatasetSplit,
    model_name: str = "logistic_regression",
) -> list[dict[str, Any]]:
    """Evaluate every ablation configuration on one fixed split.

    Only the Phase 6.1 logistic-regression baseline is supported: the
    goal is to show how performance changes when feature groups are
    removed, not to compare algorithms (see ``compare.py``).
    """
    if model_name != "logistic_regression":
        raise ValueError(f"unsupported ablation model: {model_name}")

    results: list[dict[str, Any]] = []
    for config_name, groups in ABLATON_CONFIGS:
        columns = columns_for(groups)
        X_train, y_train = rows_to_features(split.train)
        X_test, y_test = rows_to_features(split.test)
        X_train, X_test = X_train[columns], X_test[columns]

        model = build_baseline_model()
        model.fit(X_train, y_train)
        metrics = evaluate_model(model, X_test, y_test)
        predictions = model.predict(X_test)
        tn, fp, fn, tp = (
            int(v)
            for v in confusion_matrix(y_test, predictions, labels=[0, 1]).ravel()
        )
        results.append(
            {
                "feature_groups": config_name,
                "feature_count": len(columns),
                "features": columns,
                "precision": metrics["precision"],
                "recall": metrics["recall"],
                "f1": metrics["f1"],
                "roc_auc": metrics["roc_auc"],
                "pr_auc": metrics["pr_auc"],
                "tp": tp,
                "fp": fp,
                "tn": tn,
                "fn": fn,
            }
        )
    return results
