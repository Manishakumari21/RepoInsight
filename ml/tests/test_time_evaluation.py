import json

from repoinsight_ml.time_evaluation import (
    build_time_windows,
    evaluate_time_windows,
)


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


def _rows(n_commits=6, per_commit=4, single_class_window=False):
    rows = []
    for c in range(n_commits):
        for r in range(per_commit):
            if single_class_window and c == n_commits - 1:
                label = 1  # last test window has one class only
            else:
                label = (r + c) % 2
            rows.append(_row(f"c{c}", 1000 + c * 100, f"f{r}.py", label))
    return rows


def test_windows_preserve_commit_boundaries_and_order():
    rows = _rows(n_commits=6, per_commit=3)
    windows = build_time_windows(rows, n_windows=2)
    assert len(windows) == 2
    for train_commits, test_commits in windows:
        assert not set(train_commits) & set(test_commits)
    # Progressively newer: later test commits come after earlier ones.
    first_test = windows[0][1]
    second_test = windows[1][1]
    ts = {r["commit_sha"]: r["timestamp"] for r in rows}
    assert max(ts[s] for s in first_test) <= min(ts[s] for s in second_test)
    # Expanding train: first train is subset of second train.
    assert set(windows[0][0]) <= set(windows[1][0])


def test_results_have_required_fields_and_no_overlap():
    rows = _rows(n_commits=6, per_commit=4)
    results = evaluate_time_windows(rows, n_windows=2)
    assert len(results) == 2 * 3  # windows * models
    for entry in results:
        assert set(entry) >= {
            "model",
            "train_start",
            "train_end",
            "test_start",
            "test_end",
            "train_rows",
            "test_rows",
            "precision",
            "recall",
            "f1",
            "roc_auc",
            "pr_auc",
        }
        assert entry["train_end"] <= entry["test_start"]
        assert entry["train_rows"] > 0 and entry["test_rows"] > 0
        assert 0.0 <= entry["precision"] <= 1.0
        assert 0.0 <= entry["recall"] <= 1.0
        assert 0.0 <= entry["f1"] <= 1.0


def test_results_are_deterministic():
    rows = _rows(n_commits=6, per_commit=4)
    first = evaluate_time_windows(rows, n_windows=2)
    second = evaluate_time_windows(rows, n_windows=2)
    assert first == second


def test_single_class_window_does_not_crash():
    rows = _rows(n_commits=4, per_commit=4, single_class_window=True)
    results = evaluate_time_windows(rows, n_windows=2)
    assert results
    for entry in results:
        assert entry["precision"] is not None
        # Safe AUC behavior: None when test has one class.
        if entry["window"] == 1:
            assert entry["roc_auc"] is None
            assert entry["pr_auc"] is None
