import warnings

import numpy as np
import pandas as pd
import pytest

from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.evidence import (
    FEATURE_COLUMNS,
    extract_evidence,
    top_evidence,
)
from repoinsight_ml.features import FEATURE_COLUMNS as COLS


def _frame(n=40, seed=7):
    rng = np.random.default_rng(seed)
    data = {col: rng.normal(0, 1, n) for col in COLS}
    return pd.DataFrame(data, columns=COLS)


def _labels(X):
    return pd.Series(
        (X["historical_rework_frequency"] + X["coupling"] > 0).astype(int)
    )


def _fitted(name="logistic_regression"):
    X = _frame()
    y = _labels(X)
    model = build_candidate_models()[name]
    model.fit(X, y)
    return model, X.iloc[[0]]


def test_all_23_features_represented():
    model, row = _fitted()
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    assert len(evidence) == 23 == len(FEATURE_COLUMNS)
    assert [e["feature"] for e in evidence] == FEATURE_COLUMNS
    for entry in evidence:
        assert set(entry) == {
            "feature",
            "group",
            "description",
            "raw_value",
            "transformed_value",
            "contribution",
            "direction",
            "method",
        }
        assert entry["direction"] in ("supports", "contradicts", "unknown")


def test_logistic_contribution_matches_coefficient_times_value():
    model, row = _fitted()
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    coef = model.named_steps["classifier"].coef_[0]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", UserWarning)
        imputed = pd.DataFrame(
            model.named_steps["imputer"].transform(row), columns=row.columns
        )
        transformed = model.named_steps["scaler"].transform(imputed)[0]
    for entry, c, t in zip(evidence, coef, transformed):
        assert entry["contribution"] == pytest.approx(c * t)
        assert entry["method"] == "coefficient"
        if c * t > 0:
            assert entry["direction"] == "supports"
        elif c * t < 0:
            assert entry["direction"] == "contradicts"
        else:
            assert entry["direction"] == "unknown"


def test_random_forest_has_no_fabricated_direction():
    model, row = _fitted("random_forest")
    evidence = extract_evidence(model, row, model_name="random_forest")
    assert len(evidence) == 23
    assert all(e["direction"] == "unknown" for e in evidence)
    assert all(e["method"] == "impurity" for e in evidence)
    total = sum(e["contribution"] for e in evidence)
    assert total == pytest.approx(1.0)


def test_boosting_without_background_is_unknown():
    model, row = _fitted("hist_gradient_boosting")
    evidence = extract_evidence(
        model, row, model_name="hist_gradient_boosting"
    )
    assert len(evidence) == 23
    assert all(e["direction"] == "unknown" for e in evidence)
    assert all(e["method"] == "unavailable" for e in evidence)


def test_boosting_with_background_uses_permutation():
    X = _frame()
    y = _labels(X)
    model = build_candidate_models()["hist_gradient_boosting"]
    model.fit(X, y)
    evidence = extract_evidence(
        model,
        X.iloc[[0]],
        model_name="hist_gradient_boosting",
        background=(X, y),
    )
    assert all(e["method"] == "permutation" for e in evidence)
    assert all(e["direction"] == "unknown" for e in evidence)


def test_top_n_and_determinism():
    model, row = _fitted()
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    assert [e["feature"] for e in top_evidence(evidence, n=5)] == [
        e["feature"]
        for e in top_evidence(
            extract_evidence(model, row, model_name="logistic_regression"),
            n=5,
        )
    ]
    assert len(top_evidence(evidence, n=3)) == 3
    magnitudes = [abs(e["contribution"]) for e in top_evidence(evidence, n=23)]
    assert magnitudes == sorted(magnitudes, reverse=True)


def test_unsupported_model_fails_clearly():
    model, row = _fitted()
    with pytest.raises(ValueError, match="Unsupported model"):
        extract_evidence(model, row, model_name="svm")


def test_mismatched_estimator_fails_clearly():
    model, row = _fitted("random_forest")
    with pytest.raises(ValueError, match="requires a LogisticRegression"):
        extract_evidence(model, row, model_name="logistic_regression")


def test_missing_values_handled_safely():
    model, _ = _fitted()
    row = _frame(n=1, seed=1)
    row.loc[0, "avg_change_delay_seconds"] = np.nan
    evidence = extract_evidence(model, row, model_name="logistic_regression")
    assert len(evidence) == 23
    by_name = {e["feature"]: e for e in evidence}
    assert by_name["avg_change_delay_seconds"]["raw_value"] is None
    assert by_name["avg_change_delay_seconds"]["transformed_value"] is not None


def test_wrong_length_vector_fails():
    model, _ = _fitted()
    with pytest.raises(ValueError, match="Expected 23 features"):
        extract_evidence(model, [1.0, 2.0], model_name="logistic_regression")
