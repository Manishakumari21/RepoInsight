import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

import benchmark
from repoinsight_ml.ablation import (
    ABLATON_CONFIGS,
    FEATURE_GROUPS,
    columns_for,
    run_ablation,
)
from repoinsight_ml.dataset import split_chronologically
from repoinsight_ml.evaluate import evaluate_model
from repoinsight_ml.features import FEATURE_COLUMNS, rows_to_features


def _synthetic_rows(n_commits=5, rows_per_commit=6):
    rows = []
    for c in range(n_commits):
        for r in range(rows_per_commit):
            rows.append(
                {
                    "commit_sha": f"c{c}",
                    "timestamp": 1000 + c * 100,
                    "file_path": f"f{r}.py",
                    "structural": {
                        "file_size_bytes": 100 + r * 10 + c,
                        "lines_of_code": 10 + r,
                        "function_count": 1 + (r % 3),
                        "cyclomatic_complexity": 2 + (c % 2),
                        "max_nesting_depth": 1,
                        "incoming_dependencies": r % 2,
                        "outgoing_dependencies": c % 2,
                        "coupling": r,
                        "num_dependents": 0,
                    },
                    "historical": {
                        "previous_change_count": c,
                        "historical_churn": c * 10 + r,
                        "contributor_count": 1,
                        "time_since_last_change_secs": None,
                        "recent_change_frequency": 0.1 * c,
                        "historical_cochange_frequency": r % 2,
                    },
                    "temporal": {
                        "recent_change_sequence_count": c % 3,
                        "previous_files_changed_count": 1,
                        "change_window_count": c,
                        "avg_change_delay_seconds": None,
                        "change_order_count": 0,
                        "propagation_frequency": r % 2,
                        "followup_frequency": 0,
                        "historical_rework_frequency": c % 2,
                    },
                    "label": (r + 2 * c) % 3 == 0,
                }
            )
            rows[-1]["label"] = int(rows[-1]["label"])
    return rows


def test_feature_groups_partition_canonical_columns():
    assert [len(FEATURE_GROUPS[g]) for g in ("structural", "historical", "temporal")] == [9, 6, 8]
    seen = FEATURE_GROUPS["structural"] + FEATURE_GROUPS["historical"] + FEATURE_GROUPS["temporal"]
    assert sorted(seen) == sorted(FEATURE_COLUMNS)
    assert len(set(seen)) == 23


def test_columns_for_unknown_group_raises():
    try:
        columns_for(["nope"])
    except KeyError:
        return
    raise AssertionError("expected KeyError")


def test_ablation_covers_all_configurations_with_same_test_size():
    split = split_chronologically(_synthetic_rows())
    results = run_ablation(split)
    assert [r["feature_groups"] for r in results] == [c[0] for c in ABLATON_CONFIGS]
    assert len(results) == 7
    sizes = {r["tp"] + r["fp"] + r["tn"] + r["fn"] for r in results}
    assert sizes == {len(split.test)}, "every config sees the same test rows"
    counts = {"all": 23, "structural-only": 9, "historical-only": 6, "temporal-only": 8,
              "structural+historical": 15, "structural+temporal": 17, "historical+temporal": 14}
    for r in results:
        assert r["feature_count"] == counts[r["feature_groups"]]


def test_split_has_no_future_rows_in_train():
    split = split_chronologically(_synthetic_rows())
    assert max(int(r["timestamp"]) for r in split.train) < min(
        int(r["timestamp"]) for r in split.test
    )


def test_undefined_metrics_stay_undefined():
    rows = _synthetic_rows()
    # Keep both classes in train, but leave the final (test) commit all-negative.
    last = max(r["commit_sha"] for r in rows)
    for row in rows:
        if row["commit_sha"] == last:
            row["label"] = 0
    split = split_chronologically(rows)
    assert len(split.test) > 0
    assert {r["label"] for r in split.test} == {0}
    results = run_ablation(split)
    for r in results:
        assert r["roc_auc"] is None
        assert r["pr_auc"] is None
        assert r["tp"] + r["fp"] + r["tn"] + r["fn"] == len(split.test)


def test_confusion_counts_are_internally_consistent():
    split = split_chronologically(_synthetic_rows())
    X_test, y_test = rows_to_features(split.test)
    positives = int((y_test == 1).sum())
    for r in run_ablation(split):
        assert r["tp"] + r["fn"] == positives
        assert r["tp"] + r["fp"] + r["tn"] + r["fn"] == len(split.test)


def test_evaluate_model_never_emits_recall_none():
    split = split_chronologically(_synthetic_rows())
    X_test, y_test = rows_to_features(split.test)

    class _Majority:
        def predict(self, X):
            import numpy as np
            return np.zeros(len(X), dtype=int)

        def predict_proba(self, X):
            import numpy as np
            return np.column_stack([np.ones(len(X)), np.zeros(len(X))])

    metrics = evaluate_model(_Majority(), X_test, y_test)
    assert metrics["recall"] == 0.0
