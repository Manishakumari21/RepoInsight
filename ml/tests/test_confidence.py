import pytest

from repoinsight_ml.confidence import describe_confidence


def test_documented_thresholds():
    assert describe_confidence(0.55)["confidence_level"] == "low"
    assert describe_confidence(0.70)["confidence_level"] == "medium"
    assert describe_confidence(0.90)["confidence_level"] == "high"
    assert describe_confidence(0.10)["confidence_level"] == "high"
    assert describe_confidence(0.50)["confidence_level"] == "low"


def test_probability_preserved_and_calibrated_flag():
    out = describe_confidence(0.82, calibrated=True)
    assert out == {
        "probability": 0.82,
        "confidence_level": "high",
        "calibrated": True,
    }
    assert describe_confidence(0.82)["calibrated"] is False


def test_invalid_probability_fails():
    with pytest.raises(ValueError, match="outside"):
        describe_confidence(1.2)
    with pytest.raises(ValueError, match="outside"):
        describe_confidence(-0.1)
