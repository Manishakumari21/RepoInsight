import numpy as np
from sklearn.calibration import CalibratedClassifierCV
from sklearn.frozen import FrozenEstimator

from repoinsight_ml.calibration import evaluate_calibration
from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.dataset import split_chronologically
from repoinsight_ml.features import rows_to_features


def _row(sha, ts, path, label):
    blob = {
        "file_size_bytes": 100,
        "lines_of_code": 10,
        "function_count": 1,
        "cyclomatic_complexity": 2,
        "max_nesting_depth": 1,
        "incoming_dependencies": 0,
        "outgoing_dependencies": 0,
        "coupling": 0,
        "num_dependents": 0,
    }
    hist = {
        "previous_change_count": 0,
        "historical_churn": 0,
        "contributor_count": 1,
        "time_since_last_change_secs": None,
        "recent_change_frequency": 0.0,
        "historical_cochange_frequency": 0,
    }
    temp = {
        "recent_change_sequence_count": 0,
        "previous_files_changed_count": 0,
        "change_window_count": 0,
        "avg_change_delay_seconds": None,
        "change_order_count": 0,
        "propagation_frequency": 0,
        "followup_frequency": 0,
        "historical_rework_frequency": 0,
    }
    return {
        "commit_sha": sha,
        "timestamp": ts,
        "file_path": path,
        "structural": dict(blob),
        "historical": dict(hist),
        "temporal": dict(temp),
        "label": label,
    }


def _rows(n_commits=8, per_commit=6):
    rows = []
    for c in range(n_commits):
        for r in range(per_commit):
            rows.append(_row(f"c{c}", 1000 + c * 100, f"f{r}.py", (r + c) % 2))
    return rows


def test_all_candidates_can_be_calibrated():
    results = evaluate_calibration(_rows())
    assert {r["model"] for r in results} == {
        "logistic_regression",
        "random_forest",
        "hist_gradient_boosting",
    }
    for r in results:
        assert set(r) >= {
            "model",
            "roc_auc_uncalibrated",
            "roc_auc_calibrated",
            "pr_auc_uncalibrated",
            "pr_auc_calibrated",
            "brier_uncalibrated",
            "brier_calibrated",
        }
        assert 0.0 <= r["brier_uncalibrated"] <= 1.0
        assert 0.0 <= r["brier_calibrated"] <= 1.0


def test_calibration_does_not_use_test_data_for_fitting():
    rows = _rows()
    split = split_chronologically(rows)
    train_ids = {r["commit_sha"] for r in split.train}
    val_ids = {r["commit_sha"] for r in split.validation}
    test_ids = {r["commit_sha"] for r in split.test}
    assert not (train_ids & val_ids)
    assert not (train_ids & test_ids)
    assert not (val_ids & test_ids)
    assert max(r["timestamp"] for r in split.train) <= min(
        r["timestamp"] for r in split.validation
    )
    assert max(r["timestamp"] for r in split.validation) <= min(
        r["timestamp"] for r in split.test
    )

    # Single-class test period still calibrates: test labels are not
    # needed to fit the calibrator.
    single_class = [dict(r) for r in rows]
    test_ids = {r["commit_sha"] for r in split.test}
    for r in single_class:
        if r["commit_sha"] in test_ids:
            r["label"] = 1
    results = evaluate_calibration(single_class)
    assert len(results) == 3
    for r in results:
        assert r["roc_auc_uncalibrated"] is None
        assert r["roc_auc_calibrated"] is None


def test_calibrated_probabilities_within_bounds():
    rows = _rows()
    split = split_chronologically(rows)
    X_train, y_train = rows_to_features(split.train)
    X_val, y_val = rows_to_features(split.validation)
    X_test, _ = rows_to_features(split.test)
    for _, base in build_candidate_models().items():
        base.fit(X_train, y_train)
        cal = CalibratedClassifierCV(
            estimator=FrozenEstimator(base), method="sigmoid", cv=2
        )
        cal.fit(X_val, y_val)
        probs = cal.predict_proba(X_test)[:, 1]
        assert np.all(probs >= 0.0) and np.all(probs <= 1.0)


def test_calibration_is_deterministic():
    import math

    rows = _rows()
    first = evaluate_calibration(rows)
    second = evaluate_calibration(rows)
    assert [r["model"] for r in first] == [r["model"] for r in second]
    for a, b in zip(first, second):
        for key in (
            "roc_auc_uncalibrated",
            "roc_auc_calibrated",
            "pr_auc_uncalibrated",
            "pr_auc_calibrated",
            "brier_uncalibrated",
            "brier_calibrated",
        ):
            if a[key] is None or b[key] is None:
                assert a[key] is None and b[key] is None
            else:
                assert math.isclose(a[key], b[key], rel_tol=1e-9, abs_tol=1e-12)
