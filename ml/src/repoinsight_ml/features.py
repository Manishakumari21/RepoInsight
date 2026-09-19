from __future__ import annotations

from typing import Any

import numpy as np
import pandas as pd


FEATURE_COLUMNS = [
    # Structural
    "file_size_bytes",
    "lines_of_code",
    "function_count",
    "cyclomatic_complexity",
    "max_nesting_depth",
    "incoming_dependencies",
    "outgoing_dependencies",
    "coupling",
    "num_dependents",

    # Historical
    "previous_change_count",
    "historical_churn",
    "contributor_count",
    "time_since_last_change_secs",
    "recent_change_frequency",
    "historical_cochange_frequency",

    # Temporal
    "recent_change_sequence_count",
    "previous_files_changed_count",
    "change_window_count",
    "avg_change_delay_seconds",
    "change_order_count",
    "propagation_frequency",
    "followup_frequency",
    "historical_rework_frequency",
]


def rows_to_features(
    rows: list[dict[str, Any]],
) -> tuple[pd.DataFrame, pd.Series]:
    """Convert Phase 5 dataset rows into the ML feature matrix and target."""

    features: list[dict[str, float]] = []
    labels: list[int] = []

    for row in rows:
        structural = row["structural"]
        historical = row["historical"]
        temporal = row["temporal"]

        features.append(
            {
                # Structural
                "file_size_bytes": float(structural["file_size_bytes"]),
                "lines_of_code": float(structural["lines_of_code"]),
                "function_count": float(structural["function_count"]),
                "cyclomatic_complexity": float(
                    structural["cyclomatic_complexity"]
                ),
                "max_nesting_depth": float(
                    structural["max_nesting_depth"]
                ),
                "incoming_dependencies": float(
                    structural["incoming_dependencies"]
                ),
                "outgoing_dependencies": float(
                    structural["outgoing_dependencies"]
                ),
                "coupling": float(structural["coupling"]),
                "num_dependents": float(structural["num_dependents"]),

                # Historical
                "previous_change_count": float(
                    historical["previous_change_count"]
                ),
                "historical_churn": float(
                    historical["historical_churn"]
                ),
                "contributor_count": float(
                    historical["contributor_count"]
                ),
                "time_since_last_change_secs": _optional_float(
                    historical["time_since_last_change_secs"]
                ),
                "recent_change_frequency": float(
                    historical["recent_change_frequency"]
                ),
                "historical_cochange_frequency": float(
                    historical["historical_cochange_frequency"]
                ),

                # Temporal
                "recent_change_sequence_count": float(
                    temporal["recent_change_sequence_count"]
                ),
                "previous_files_changed_count": float(
                    temporal["previous_files_changed_count"]
                ),
                "change_window_count": float(
                    temporal["change_window_count"]
                ),
                "avg_change_delay_seconds": _optional_float(
                    temporal["avg_change_delay_seconds"]
                ),
                "change_order_count": float(
                    temporal["change_order_count"]
                ),
                "propagation_frequency": float(
                    temporal["propagation_frequency"]
                ),
                "followup_frequency": float(
                    temporal["followup_frequency"]
                ),
                "historical_rework_frequency": float(
                    temporal["historical_rework_frequency"]
                ),
            }
        )

        labels.append(int(row["label"]))

    frame = pd.DataFrame(features, columns=FEATURE_COLUMNS)
    target = pd.Series(labels, name="label", dtype="int64")

    return frame, target


def _optional_float(value: Any) -> float:
    """Represent Rust Option numeric values as NaN for sklearn imputation."""

    if value is None:
        return np.nan

    return float(value)