import json
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.dataset import load_jsonl
from repoinsight_ml.evidence import extract_evidence
from repoinsight_ml.explanation import build_explanation, explain_prediction
from repoinsight_ml.features import FEATURE_COLUMNS, rows_to_features

DATASET = Path(__file__).resolve().parents[1] / "data" / "dataset.jsonl"


def _trained_pair(seed=11):
    rng = np.random.default_rng(seed)
    X = pd.DataFrame(
        {c: rng.normal(0, 1, 60) for c in FEATURE_COLUMNS},
        columns=FEATURE_COLUMNS,
    )
    y = pd.Series((X["coupling"] + X["previous_change_count"] > 0).astype(int))
    model = build_candidate_models()["logistic_regression"]
    model.fit(X, y)
    return model, X.iloc[[3]]


def test_probability_and_label_preserved():
    model, row = _trained_pair()
    proba = float(model.predict_proba(row)[0, 1])
    label = int(model.predict(row)[0])
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    out = build_explanation(
        model_name="logistic_regression",
        probability=proba,
        predicted_label=label,
        evidence=evidence,
    )
    assert out["predicted_probability"] == proba
    assert out["predicted_label"] == label
    assert out["model_name"] == "logistic_regression"


def test_summary_names_real_top_features():
    model, row = _trained_pair()
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    out = build_explanation(
        model_name="logistic_regression",
        probability=0.8,
        predicted_label=1,
        evidence=evidence,
        top_n=3,
    )
    top_names = [e["feature"] for e in out["top_evidence"]]
    assert len(top_names) == 3
    for name in top_names:
        assert name in out["summary"]
    assert "Many factors contributed" not in out["summary"]


def test_supporting_contradicting_split():
    model, row = _trained_pair()
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    out = build_explanation(
        model_name="logistic_regression",
        probability=0.4,
        predicted_label=0,
        evidence=evidence,
    )
    assert all(e["direction"] == "supports" for e in out["supporting_evidence"])
    assert all(
        e["direction"] == "contradicts"
        for e in out["contradicting_evidence"]
    )
    assert len(out["supporting_evidence"]) + len(
        out["contradicting_evidence"]
    ) <= len(evidence)


def test_empty_evidence_fallback_without_generic_claim():
    out = build_explanation(
        model_name="random_forest",
        probability=0.5,
        predicted_label=0,
        evidence=[],
    )
    assert out["top_evidence"] == []
    assert "No feature evidence was available" in out["summary"]


def test_invalid_inputs_fail():
    with pytest.raises(ValueError, match="outside"):
        build_explanation(
            model_name="x", probability=1.5, predicted_label=1, evidence=[]
        )
    with pytest.raises(ValueError, match="must be 0 or 1"):
        build_explanation(
            model_name="x", probability=0.5, predicted_label=2, evidence=[]
        )


def test_json_serializable_and_real_row():
    rows = load_jsonl(DATASET)
    split_at = int(len(rows) * 0.7)
    X_train, y_train = rows_to_features(rows[:split_at])
    model = build_candidate_models()["logistic_regression"]
    model.fit(X_train, y_train)
    target = rows[split_at]
    X_one, _ = rows_to_features([target])
    full = explain_prediction(
        model=model,
        model_name="logistic_regression",
        feature_vector=X_one,
        historical_rows=rows[:split_at],
        target_ref=target,
    )
    json.dumps(full)
    assert full["prediction"]["label"] in (0, 1)
    assert full["model_name"] == "logistic_regression"


def test_orchestrator_preserves_prediction():
    model, row = _trained_pair()
    full = explain_prediction(
        model=model, model_name="logistic_regression", feature_vector=row
    )
    assert full["prediction"]["probability"] == pytest.approx(
        float(model.predict_proba(row)[0, 1])
    )
    assert full["prediction"]["label"] == int(model.predict(row)[0])
