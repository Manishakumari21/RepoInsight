import math
from pathlib import Path

from repoinsight_ml.feature_importance import (
    evaluate_feature_importance,
    top_features,
)
from repoinsight_ml.features import FEATURE_COLUMNS


def _row(sha, ts, path, label):
    blob = {
        "file_size_bytes": 100 + hash(path) % 50,
        "lines_of_code": 10,
        "function_count": 1 + hash(sha) % 3,
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


def _grouped(results):
    groups: dict[tuple[str, str], list] = {}
    for r in results:
        groups.setdefault((r["model"], r["method"]), []).append(r)
    return groups


def test_exactly_23_features_matching_columns():
    for _, group in _grouped(
        evaluate_feature_importance(_rows(), n_repeats=2)
    ).items():
        assert len(group) == 23
        assert {r["feature"] for r in group} == set(FEATURE_COLUMNS)


def test_all_models_produce_finite_importance():
    results = evaluate_feature_importance(_rows(), n_repeats=2)
    assert {r["model"] for r in results} == {
        "logistic_regression",
        "random_forest",
        "hist_gradient_boosting",
    }
    methods = {r["method"] for r in results}
    assert {"coefficient_abs", "impurity", "permutation"} <= methods
    for r in results:
        assert isinstance(r["importance"], float)
        assert math.isfinite(r["importance"])


def test_deterministic_output():
    first = evaluate_feature_importance(_rows(), n_repeats=2)
    second = evaluate_feature_importance(_rows(), n_repeats=2)
    assert first == second


def test_test_data_not_used_for_fitting():
    from repoinsight_ml.dataset import split_chronologically

    rows = _rows()
    results = evaluate_feature_importance(rows, n_repeats=2)
    split = split_chronologically(rows)
    test_ids = {r["commit_sha"] for r in split.test}
    perturbed = [
        dict(r, label=1 - r["label"]) if r["commit_sha"] in test_ids else dict(r)
        for r in rows
    ]
    assert results == evaluate_feature_importance(perturbed, n_repeats=2)
    assert top_features(results, "random_forest", n=5)


def test_real_dataset_smoke():
    from repoinsight_ml.feature_importance import (
        evaluate_feature_importance_from_path,
    )

    path = Path(__file__).resolve().parents[1] / "data" / "dataset.jsonl"
    results = evaluate_feature_importance_from_path(path, n_repeats=2)
    assert {r["model"] for r in results} == {
        "logistic_regression",
        "random_forest",
        "hist_gradient_boosting",
    }
    for _, group in _grouped(results).items():
        assert len(group) == 23
        assert {r["feature"] for r in group} == set(FEATURE_COLUMNS)
        assert all(math.isfinite(r["importance"]) for r in group)
