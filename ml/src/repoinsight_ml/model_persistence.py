from __future__ import annotations

import datetime
import platform
import sys
from importlib import metadata as importlib_metadata
from pathlib import Path
from typing import Any

import joblib
import sklearn

from .features import FEATURE_COLUMNS


def _ml_version() -> str:
    try:
        return importlib_metadata.version("repoinsight-ml")
    except importlib_metadata.PackageNotFoundError:
        return "0.1.0"


def build_metadata(
    model_name: str,
    model: Any,
    train_rows: int,
) -> dict[str, Any]:
    classifier = None
    try:
        classifier = model.named_steps["classifier"]
    except (AttributeError, KeyError):
        classifier = None
    return {
        "model_name": model_name,
        "ml_version": _ml_version(),
        "feature_list": list(FEATURE_COLUMNS),
        "feature_count": len(FEATURE_COLUMNS),
        "sklearn_version": sklearn.__version__,
        "python_version": platform.python_version(),
        "python_implementation": platform.python_implementation(),
        "train_rows": int(train_rows),
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "model_type": type(classifier).__name__ if classifier is not None else type(model).__name__,
        "python_version_info": list(sys.version_info),
    }


def _check_feature_compatibility(metadata: dict[str, Any]) -> None:
    expected = metadata.get("feature_list", None)
    if expected is None:
        raise ValueError("Artifact metadata is missing feature_list")
    if list(expected) != list(FEATURE_COLUMNS):
        raise ValueError(
            f"Feature schema mismatch: artifact expects {len(list(expected))} "
            f"features, current FEATURE_COLUMNS has {len(FEATURE_COLUMNS)}. "
            "Refusing to load rather than silently mispredicting."
        )


def save_model(
    model: Any,
    path: str | Path,
    metadata: dict[str, Any] | None = None,
) -> Path:
    """Persist a complete preprocessing + classifier pipeline with joblib."""
    out = Path(path)
    if out.parent != Path("") and str(out.parent) not in ("", "."):
        out.parent.mkdir(parents=True, exist_ok=True)
    payload = {"model": model, "metadata": dict(metadata) if metadata else {}}
    joblib.dump(payload, out)
    return out


def load_model(path: str | Path) -> Any:
    """Load a persisted pipeline, validating feature compatibility."""
    src = Path(path)
    if not src.is_file():
        raise FileNotFoundError(f"Model artifact not found: {src}")
    try:
        payload = joblib.load(src)
    except Exception as exc:
        raise ValueError(f"Invalid model artifact {src}: {exc}") from exc
    if not isinstance(payload, dict) or "model" not in payload:
        raise ValueError(f"Invalid model artifact {src}: missing 'model' key")
    metadata = payload.get("metadata", {})
    if isinstance(metadata, dict) and metadata:
        _check_feature_compatibility(metadata)
    return payload["model"]


def load_model_with_metadata(path: str | Path) -> tuple[Any, dict[str, Any]]:
    src = Path(path)
    if not src.is_file():
        raise FileNotFoundError(f"Model artifact not found: {src}")
    try:
        payload = joblib.load(src)
    except Exception as exc:
        raise ValueError(f"Invalid model artifact {src}: {exc}") from exc
    if not isinstance(payload, dict) or "model" not in payload:
        raise ValueError(f"Invalid model artifact {src}: missing 'model' key")
    metadata = payload.get("metadata", {})
    if not isinstance(metadata, dict):
        raise ValueError(f"Invalid model artifact {src}: bad metadata")
    if metadata:
        _check_feature_compatibility(metadata)
    return payload["model"], metadata


def default_artifact_paths(artifact_dir: str | Path) -> dict[str, Path]:
    base = Path(artifact_dir)
    return {
        "logistic_regression": base / "logistic_regression.joblib",
        "random_forest": base / "random_forest.joblib",
        "hist_gradient_boosting": base / "hist_gradient_boosting.joblib",
    }


def persist_candidate_models(
    models_with_rows: dict[str, tuple[Any, int]],
    artifact_dir: str | Path,
) -> dict[str, Path]:
    import json

    base = Path(artifact_dir)
    base.mkdir(parents=True, exist_ok=True)
    paths = default_artifact_paths(base)
    combined: dict[str, Any] = {}
    for name, (model, train_rows) in models_with_rows.items():
        if name not in paths:
            raise ValueError(f"Unknown candidate model: {name}")
        meta = build_metadata(name, model, train_rows)
        save_model(model, paths[name], meta)
        combined[name] = meta
    with (base / "metadata.json").open("w", encoding="utf-8") as handle:
        json.dump(combined, handle, indent=2)
    return paths
