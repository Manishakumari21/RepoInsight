from repoinsight_ml.compare import build_candidate_models, compare_models


def _synthetic_rows(n_commits: int = 4, rows_per_commit: int = 6):
    rows = []
    for c in range(n_commits):
        for r in range(rows_per_commit):
            rows.append(
                {
                    "commit_sha": f"c{c}",
                    "timestamp": 1000 + c * 100,
                    "file_path": f"f{r}.py",
                    "structural": {
                        "file_size_bytes": 100 + r,
                        "lines_of_code": 10 + r,
                        "function_count": 1,
                        "cyclomatic_complexity": 2,
                        "max_nesting_depth": 1,
                        "incoming_dependencies": 0,
                        "outgoing_dependencies": 0,
                        "coupling": 0,
                        "num_dependents": 0,
                    },
                    "historical": {
                        "previous_change_count": c,
                        "historical_churn": c * 10,
                        "contributor_count": 1,
                        "time_since_last_change_secs": None,
                        "recent_change_frequency": 0.1 * c,
                        "historical_cochange_frequency": 0,
                    },
                    "temporal": {
                        "recent_change_sequence_count": 0,
                        "previous_files_changed_count": 1,
                        "change_window_count": 0,
                        "avg_change_delay_seconds": None,
                        "change_order_count": 0,
                        "propagation_frequency": 0,
                        "followup_frequency": 0,
                        "historical_rework_frequency": 0,
                    },
                    "label": (r + c) % 2,
                }
            )
    # Ensure test split has both classes (chronological last commit).
    return rows


def test_build_candidate_models_has_expected_models():
    models = build_candidate_models()
    assert set(models) == {
        "logistic_regression",
        "random_forest",
        "hist_gradient_boosting",
    }


def test_compare_models_returns_comparable_metrics(tmp_path):
    import json

    rows = _synthetic_rows()
    path = tmp_path / "dataset.jsonl"
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")

    results = compare_models(path)

    assert len(results) == 3
    for entry in results:
        assert set(entry) == {
            "model",
            "precision",
            "recall",
            "f1",
            "roc_auc",
            "pr_auc",
        }
        assert 0.0 <= entry["precision"] <= 1.0
        assert 0.0 <= entry["recall"] <= 1.0
        assert 0.0 <= entry["f1"] <= 1.0


def test_compare_models_uses_only_sklearn_estimators():
    from sklearn.pipeline import Pipeline

    for model in build_candidate_models().values():
        assert isinstance(model, Pipeline)
        assert "imputer" in model.named_steps
        assert "classifier" in model.named_steps
