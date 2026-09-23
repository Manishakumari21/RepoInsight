"""Tests for Change Ripple Forecasting ranking (ML side)."""

from repoinsight_ml.ripple import (
    build_ripple_scores,
    compare_ripple_models,
    evaluate_ripple_ranking,
)


def _row(sha, timestamp, path):
    return {
        "commit_sha": sha,
        "timestamp": timestamp,
        "file_path": path,
        "structural": {},
        "historical": {},
        "temporal": {},
        "label": 0,
    }


def _history():
    return [
        _row("c1", 100, "a.rs"),
        _row("c1", 100, "b.rs"),
        _row("c2", 200, "a.rs"),
        _row("c3", 300, "b.rs"),
        _row("c4", 400, "a.rs"),
        _row("c4", 400, "b.rs"),
        _row("c5", 500, "a.rs"),
        _row("c5", 500, "c.rs"),
    ]


def test_cochange_probability_is_directional():
    scores = build_ripple_scores(_history(), "a.rs", use_temporal=False)
    by_file = {entry["file"]: entry for entry in scores}

    assert by_file["b.rs"]["cochange_probability"] == 0.5


def test_temporal_follow_excludes_same_commit():
    rows = [_row("c1", 100, "a.rs"), _row("c1", 100, "b.rs")]
    scores = build_ripple_scores(rows, "a.rs", use_temporal=True)
    by_file = {entry["file"]: entry for entry in scores}
    assert by_file["b.rs"]["follow_probability"] == 0.0


def test_no_history_for_unknown_source():
    assert build_ripple_scores(_history(), "missing.rs") == []


def test_chronological_evaluation_respects_time():
    report = evaluate_ripple_ranking(_history(), k=2)
    assert report["evaluated_targets"] >= 1
    assert report["precision_at_k"] is not None
    assert report["k"] == 2


def test_too_few_commits_yield_no_evaluation():
    rows = [_row("c1", 100, "a.rs"), _row("c1", 100, "b.rs")]
    report = evaluate_ripple_ranking(rows)
    assert report["evaluated_targets"] == 0
    assert report["precision_at_k"] is None


def test_baseline_vs_full_comparison_reports_both():
    reports = compare_ripple_models(_history(), k=2)
    assert [report["model"] for report in reports] == [
        "ripple_cochange_only",
        "ripple_full",
    ]
