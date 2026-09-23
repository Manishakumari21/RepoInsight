from __future__ import annotations

import warnings
from typing import Any

import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingClassifier, RandomForestClassifier
from sklearn.linear_model import LogisticRegression

from .features import FEATURE_COLUMNS

SUPPORTED_MODELS = (
    "logistic_regression",
    "random_forest",
    "hist_gradient_boosting",
)

DIRECTION_SUPPORTS = "supports"
DIRECTION_CONTRADICTS = "contradicts"
DIRECTION_UNKNOWN = "unknown"

EXPECTED_CLASSIFIERS = {
    "logistic_regression": LogisticRegression,
    "random_forest": RandomForestClassifier,
    "hist_gradient_boosting": HistGradientBoostingClassifier,
}

FEATURE_DESCRIPTIONS: dict[str, tuple[str, str]] = {
    "file_size_bytes": ("structural", "Size of the source file in bytes."),
    "lines_of_code": ("structural", "Lines of code in the source file."),
    "function_count": ("structural", "Number of functions defined in the file."),
    "cyclomatic_complexity": (
        "structural",
        "Cyclomatic complexity of the file.",
    ),
    "max_nesting_depth": (
        "structural",
        "Maximum nesting depth in the file.",
    ),
    "incoming_dependencies": (
        "structural",
        "Number of incoming dependencies on the file.",
    ),
    "outgoing_dependencies": (
        "structural",
        "Number of outgoing dependencies from the file.",
    ),
    "coupling": ("structural", "Coupling of the file to other files."),
    "num_dependents": (
        "structural",
        "Number of files depending on this file.",
    ),
    "previous_change_count": (
        "historical",
        "How often the file changed before the target commit.",
    ),
    "historical_churn": (
        "historical",
        "Total added plus deleted lines for the file in prior history.",
    ),
    "contributor_count": (
        "historical",
        "Number of distinct contributors touching the file before.",
    ),
    "time_since_last_change_secs": (
        "historical",
        "Seconds since the file last changed before the target commit.",
    ),
    "recent_change_frequency": (
        "historical",
        "How frequently the file changed in the recent window.",
    ),
    "historical_cochange_frequency": (
        "historical",
        "How often the file changed together with other files.",
    ),
    "recent_change_sequence_count": (
        "temporal",
        "Recent ordered change sequences involving the file.",
    ),
    "previous_files_changed_count": (
        "temporal",
        "Files changed in commits preceding the target commit.",
    ),
    "change_window_count": (
        "temporal",
        "Changes to the file inside the recent change window.",
    ),
    "avg_change_delay_seconds": (
        "temporal",
        "Average delay between consecutive changes to the file.",
    ),
    "change_order_count": (
        "temporal",
        "Ordered change events involving the file.",
    ),
    "propagation_frequency": (
        "temporal",
        "How often changes propagated through this file.",
    ),
    "followup_frequency": (
        "temporal",
        "How often changes to the file were followed by further changes.",
    ),
    "historical_rework_frequency": (
        "temporal",
        "How often changes to the file were followed by rework.",
    ),
}

PERMUTATION_REPEATS = 5
PERMUTATION_RANDOM_STATE = 42


def _as_frame(
    feature_vector: Any, feature_names: list[str]
) -> pd.DataFrame:
    """Coerce one feature vector into a single-row DataFrame."""
    if isinstance(feature_vector, pd.DataFrame):
        frame = feature_vector.copy()
    elif isinstance(feature_vector, pd.Series):
        frame = feature_vector.to_frame().T
    else:
        values = np.asarray(feature_vector, dtype=float).ravel()
        if values.shape[0] != len(feature_names):
            raise ValueError(
                f"Expected {len(feature_names)} features, "
                f"got {values.shape[0]}"
            )
        frame = pd.DataFrame([values], columns=feature_names)
    missing = [col for col in feature_names if col not in frame.columns]
    if missing:
        raise ValueError(f"Feature vector is missing columns: {missing}")
    return frame[feature_names]


def _classifier(model: Any) -> Any:
    """Return the fitted classifier inside a pipeline or bare estimator."""
    named = getattr(model, "named_steps", None)
    if named is not None and "classifier" in named:
        return named["classifier"]
    return model


def _transformed_values(model: Any, frame: pd.DataFrame) -> np.ndarray:
    """Apply pre-classifier pipeline steps to one row, deterministically."""
    named = getattr(model, "named_steps", None)
    out: np.ndarray = frame.to_numpy(dtype=float)
    if named is None:
        return out
    columns = list(frame.columns)
    for name, step in named.items():
        if name == "classifier":
            break
        transform = getattr(step, "transform", None)
        if transform is None:
            raise ValueError(
                f"Pipeline step {name!r} has no transform; "
                "cannot compute transformed values"
            )
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            intermediate = pd.DataFrame(out, columns=columns)
            out = np.asarray(transform(intermediate), dtype=float)
    return out


def _clean(value: float) -> float | None:
    """Map NaN to None so evidence stays JSON serializable."""
    number = float(value)
    if np.isnan(number):
        return None
    return number


def _check_model(model: Any, model_name: str) -> Any:
    """Fail clearly on unsupported models or mismatched estimators."""
    if model_name not in SUPPORTED_MODELS:
        raise ValueError(
            f"Unsupported model {model_name!r}; "
            f"expected one of {list(SUPPORTED_MODELS)}"
        )
    classifier = _classifier(model)
    expected = EXPECTED_CLASSIFIERS[model_name]
    if not isinstance(classifier, expected):
        raise ValueError(
            f"Model {model_name!r} requires a "
            f"{expected.__name__} classifier, got "
            f"{type(classifier).__name__}; refusing to apply "
            "another model's explanation logic"
        )
    return classifier


def _logistic_contributions(
    classifier: LogisticRegression, transformed: np.ndarray
) -> tuple[np.ndarray, str]:
    coefs = np.asarray(classifier.coef_[0], dtype=float)
    return coefs * transformed, "coefficient"


def _forest_contributions(
    classifier: RandomForestClassifier,
) -> tuple[np.ndarray, str]:
    importances = np.asarray(
        classifier.feature_importances_, dtype=float
    )
    return importances, "impurity"


def _boosting_contributions(
    model: Any,
    frame: pd.DataFrame,
    background: tuple[Any, Any] | None,
) -> tuple[np.ndarray, str]:
    from sklearn.inspection import permutation_importance

    if background is None:
        return np.zeros(frame.shape[1], dtype=float), "unavailable"
    X_bg, y_bg = background
    result = permutation_importance(
        model,
        X_bg,
        y_bg,
        n_repeats=PERMUTATION_REPEATS,
        random_state=PERMUTATION_RANDOM_STATE,
    )
    return np.asarray(result.importances_mean, dtype=float), "permutation"


def extract_evidence(
    model: Any,
    feature_vector: Any,
    *,
    model_name: str,
    probability: float | None = None,
    predicted_label: int | None = None,
    feature_names: list[str] | None = None,
    background: tuple[Any, Any] | None = None,
) -> list[dict[str, Any]]:
    names = list(feature_names) if feature_names else list(FEATURE_COLUMNS)
    if len(names) != len(FEATURE_COLUMNS):
        raise ValueError(
            f"Expected {len(FEATURE_COLUMNS)} feature names, got {len(names)}"
        )
    if probability is not None and not 0.0 <= float(probability) <= 1.0:
        raise ValueError(f"Probability {probability!r} is outside [0, 1]")
    if predicted_label is not None and int(predicted_label) not in (0, 1):
        raise ValueError(f"Predicted label {predicted_label!r} must be 0 or 1")

    classifier = _check_model(model, model_name)
    frame = _as_frame(feature_vector, names)
    raw = frame.to_numpy(dtype=float)[0]
    transformed = _transformed_values(model, frame)[0]

    if model_name == "logistic_regression":
        contributions, method = _logistic_contributions(
            classifier, transformed
        )
        signed = True
    elif model_name == "random_forest":
        contributions, method = _forest_contributions(classifier)
        signed = False
    else:
        contributions, method = _boosting_contributions(
            model, frame, background
        )
        signed = False

    evidence: list[dict[str, Any]] = []
    for index, feature in enumerate(names):
        contribution = float(contributions[index])
        if signed:
            if contribution > 0:
                direction = DIRECTION_SUPPORTS
            elif contribution < 0:
                direction = DIRECTION_CONTRADICTS
            else:
                direction = DIRECTION_UNKNOWN
        else:
            direction = DIRECTION_UNKNOWN
        group, description = FEATURE_DESCRIPTIONS.get(
            feature, ("unknown", "No description available.")
        )
        evidence.append(
            {
                "feature": feature,
                "group": group,
                "description": description,
                "raw_value": _clean(raw[index]),
                "transformed_value": _clean(transformed[index]),
                "contribution": contribution,
                "direction": direction,
                "method": method,
            }
        )
    return evidence


def top_evidence(
    evidence: list[dict[str, Any]], n: int = 5
) -> list[dict[str, Any]]:
    if n < 1:
        raise ValueError("n must be >= 1")
    ordered = sorted(
        evidence,
        key=lambda entry: (
            -abs(float(entry["contribution"])),
            str(entry["feature"]),
        ),
    )
    return ordered[:n]
