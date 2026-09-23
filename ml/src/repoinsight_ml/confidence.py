from __future__ import annotations

from typing import Any

LOW_THRESHOLD = 0.6
HIGH_THRESHOLD = 0.8


def describe_confidence(
    probability: float, *, calibrated: bool = False
) -> dict[str, Any]:
    probability = float(probability)
    if not 0.0 <= probability <= 1.0:
        raise ValueError(f"Probability {probability!r} is outside [0, 1]")
    strength = max(probability, 1.0 - probability)
    if strength >= HIGH_THRESHOLD:
        level = "high"
    elif strength >= LOW_THRESHOLD:
        level = "medium"
    else:
        level = "low"
    return {
        "probability": probability,
        "confidence_level": level,
        "calibrated": bool(calibrated),
    }
