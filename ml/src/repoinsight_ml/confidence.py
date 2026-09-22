from __future__ import annotations

from typing import Any

# Documented thresholds on prediction strength max(p, 1 - p):
# strength < 0.6 -> low, 0.6 <= strength < 0.8 -> medium, >= 0.8 -> high.
LOW_THRESHOLD = 0.6
HIGH_THRESHOLD = 0.8


def describe_confidence(
    probability: float, *, calibrated: bool = False
) -> dict[str, Any]:
    """Represent model confidence without overstating it.

    The probability is the model's predicted probability for the
    positive class — reported as-is, never relabeled "certainty".
    "high" confidence describes a strong probability, not a guarantee.
    """
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
