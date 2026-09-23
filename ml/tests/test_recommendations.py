import pytest

from repoinsight_ml.features import FEATURE_COLUMNS
from repoinsight_ml.recommendations import build_recommendations


def _evidence(**overrides):
    entries = []
    for feature in FEATURE_COLUMNS:
        raw, direction = overrides.get(feature, (0.0, "unknown"))
        entries.append(
            {"feature": feature, "raw_value": raw, "direction": direction}
        )
    return entries


def test_empty_without_evidence():
    assert build_recommendations(file_path="a.py", evidence=[],
                                 historical_examples=[]) == []
    assert build_recommendations(file_path="", evidence=_evidence(),
                                 historical_examples=[]) == []


def test_rework_evidence_drives_recommendation():
    recs = build_recommendations(
        file_path="a.py",
        evidence=_evidence(historical_rework_frequency=(3.0, "supports")),
        historical_examples=[],
    )
    assert len(recs) == 1
    assert "a.py" in recs[0]
    assert "suggest" in recs[0]


def test_rework_without_support_direction_stays_silent():
    recs = build_recommendations(
        file_path="a.py",
        evidence=_evidence(historical_rework_frequency=(3.0, "unknown")),
        historical_examples=[],
    )
    assert recs == []


def test_cochange_uses_real_related_file():
    recs = build_recommendations(
        file_path="a.py",
        evidence=_evidence(),
        historical_examples=[{
            "commit_sha": "abc",
            "timestamp": 1,
            "file_path": "a.py",
            "event_type": "co_change",
            "related_files": ["api.py"],
        }],
    )
    assert recs == [
        "Consider reviewing related changes in api.py "
        "before modifying a.py."
    ]


def test_hedged_language_and_determinism():
    recs = build_recommendations(
        file_path="a.py",
        evidence=_evidence(
            historical_rework_frequency=(2.0, "supports"),
            followup_frequency=(1.0, "supports"),
            propagation_frequency=(4.0, "supports"),
            coupling=(6.0, "supports"),
        ),
        historical_examples=[],
    )
    assert len(recs) == 3
    joined = " ".join(recs)
    assert "definitely" not in joined
    assert "will break" not in joined
    assert "guarantee" not in joined
    again = build_recommendations(
        file_path="a.py",
        evidence=_evidence(
            historical_rework_frequency=(2.0, "supports"),
            followup_frequency=(1.0, "supports"),
            propagation_frequency=(4.0, "supports"),
            coupling=(6.0, "supports"),
        ),
        historical_examples=[],
    )
    assert recs == again
    with pytest.raises(ValueError, match="top_n"):
        build_recommendations(file_path="a.py", evidence=_evidence(),
                              historical_examples=[], top_n=0)
