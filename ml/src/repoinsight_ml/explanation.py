from __future__ import annotations

from typing import Any

from .confidence import describe_confidence
from .evidence import top_evidence
from .historical_examples import find_historical_examples
from .recommendations import build_recommendations


def build_explanation(
    *,
    model_name: str,
    probability: float,
    predicted_label: int,
    evidence: list[dict[str, Any]],
    top_n: int = 5,
) -> dict[str, Any]:
    """Build a structured explanation derived from actual evidence.

    top_evidence, supporting_evidence and contradicting_evidence are
    slices of the supplied evidence — never invented. The summary names
    the real top contributors.
    """
    probability = float(probability)
    if not 0.0 <= probability <= 1.0:
        raise ValueError(f"Probability {probability!r} is outside [0, 1]")
    predicted_label = int(predicted_label)
    if predicted_label not in (0, 1):
        raise ValueError(f"Predicted label {predicted_label!r} must be 0 or 1")

    top = top_evidence(evidence, n=top_n) if evidence else []
    supporting = sorted(
        (entry for entry in evidence if entry["direction"] == "supports"),
        key=lambda entry: (-float(entry["contribution"]), str(entry["feature"])),
    )
    contradicting = sorted(
        (entry for entry in evidence if entry["direction"] == "contradicts"),
        key=lambda entry: (float(entry["contribution"]), str(entry["feature"])),
    )

    outcome = "likely to require rework" if predicted_label == 1 else (
        "unlikely to require rework"
    )
    if top:
        names = ", ".join(entry["feature"] for entry in top)
        summary = (
            f"{model_name} predicts the file is {outcome} "
            f"(probability {probability:.2f}). Evidence associated with "
            f"this prediction comes from: {names}."
        )
    else:
        summary = (
            f"{model_name} predicts the file is {outcome} "
            f"(probability {probability:.2f}). No feature evidence was "
            "available for this prediction."
        )

    return {
        "model_name": model_name,
        "predicted_probability": probability,
        "predicted_label": predicted_label,
        "summary": summary,
        "top_evidence": top,
        "supporting_evidence": supporting,
        "contradicting_evidence": contradicting,
    }


def explain_prediction(
    *,
    model: Any,
    model_name: str,
    feature_vector: Any,
    feature_names: list[str] | None = None,
    historical_rows: list[dict[str, Any]] | None = None,
    target_ref: dict[str, Any] | None = None,
    calibrated: bool = False,
    top_n: int = 5,
    background: tuple[Any, Any] | None = None,
) -> dict[str, Any]:
    """End-to-end explanation: predict, then explain with real data.

    Historical examples only use rows strictly before the target
    timestamp, so no future information leaks into the explanation.
    """
    from .evidence import extract_evidence

    probability = float(model.predict_proba(feature_vector)[0, 1])
    predicted_label = int(model.predict(feature_vector)[0])

    evidence = extract_evidence(
        model,
        feature_vector,
        model_name=model_name,
        probability=probability,
        predicted_label=predicted_label,
        feature_names=feature_names,
        background=background,
    )
    explanation = build_explanation(
        model_name=model_name,
        probability=probability,
        predicted_label=predicted_label,
        evidence=evidence,
        top_n=top_n,
    )
    examples: list[dict[str, Any]] = []
    file_path = None
    if historical_rows is not None and target_ref is not None:
        file_path = target_ref.get("file_path")
        examples = find_historical_examples(
            historical_rows,
            commit_sha=str(target_ref.get("commit_sha", "")),
            timestamp=int(target_ref.get("timestamp", 0)),
            file_path=str(file_path or ""),
        )
    confidence = describe_confidence(probability, calibrated=calibrated)
    recommendations = build_recommendations(
        file_path=str(file_path or ""),
        evidence=evidence,
        historical_examples=examples,
    )
    return {
        "prediction": {
            "probability": explanation["predicted_probability"],
            "label": explanation["predicted_label"],
            "calibrated": bool(calibrated),
        },
        "evidence": {
            "top_evidence": explanation["top_evidence"],
            "supporting_evidence": explanation["supporting_evidence"],
            "contradicting_evidence": explanation["contradicting_evidence"],
        },
        "historical_examples": examples,
        "confidence": confidence,
        "recommendations": recommendations,
        "model_name": model_name,
        "summary": explanation["summary"],
    }
