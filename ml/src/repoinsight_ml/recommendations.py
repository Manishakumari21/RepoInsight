from __future__ import annotations

from typing import Any


def _raw(evidence: list[dict[str, Any]], feature: str) -> float | None:
    for entry in evidence:
        if entry.get("feature") == feature:
            value = entry.get("raw_value")
            return None if value is None else float(value)
    return None


def _supports(evidence: list[dict[str, Any]], feature: str) -> bool:
    return any(
        entry.get("feature") == feature and entry.get("direction") == "supports"
        for entry in evidence
    )


def build_recommendations(
    *,
    file_path: str,
    evidence: list[dict[str, Any]],
    historical_examples: list[dict[str, Any]],
    top_n: int = 3,
) -> list[str]:
    if top_n < 1:
        raise ValueError("top_n must be >= 1")
    if not file_path or not evidence:
        return []

    suggestions: list[str] = []
    rework = _raw(evidence, "historical_rework_frequency")
    if rework is not None and rework > 0 and _supports(
        evidence, "historical_rework_frequency"
    ):
        suggestions.append(
            "Historical changes suggest reviewing past rework in "
            f"{file_path} before modifying it."
        )

    cochange = next(
        (
            item
            for item in historical_examples
            if item.get("event_type") == "co_change"
            and item.get("related_files")
        ),
        None,
    )
    if cochange is not None:
        related = str(cochange["related_files"][0])
        suggestions.append(
            "Consider reviewing related changes in "
            f"{related} before modifying {file_path}."
        )

    followup = _raw(evidence, "followup_frequency")
    if followup is not None and followup > 0 and _supports(
        evidence, "followup_frequency"
    ):
        suggestions.append(
            f"Check whether recent follow-up changes affect {file_path}."
        )

    propagation = _raw(evidence, "propagation_frequency")
    if propagation is not None and propagation > 0 and _supports(
        evidence, "propagation_frequency"
    ):
        suggestions.append(
            "Historical changes suggest checking downstream files that "
            f"previously changed after {file_path}."
        )

    coupling = _raw(evidence, "coupling")
    if coupling is not None and coupling > 0 and _supports(
        evidence, "coupling"
    ):
        suggestions.append(
            f"Consider reviewing coupled files of {file_path}; "
            "its coupling is associated with rework."
        )

    return suggestions[:top_n]
