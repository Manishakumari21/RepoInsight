import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import benchmark
import error_analysis
from run_model_comparison import COMPARISON_KEYS, load_records

REQUIRED_BENCHMARK_KEYS = {
    "repository", "language", "dataset", "files", "commits",
    "dataset_rows", "positive_labels", "negative_labels", "positive_rate",
    "training_rows", "validation_rows", "test_rows",
    "test_positives", "test_negatives", "test_positive_rate",
    "model", "tp", "fp", "tn", "fn",
    "precision", "recall", "f1", "roc_auc", "pr_auc",
    "analysis_time_seconds", "prediction_time_seconds",
}


def _synthetic_rows(n_commits=5, rows_per_commit=8):
    rows = []
    for c in range(n_commits):
        for r in range(rows_per_commit):
            rows.append(
                {
                    "commit_sha": f"c{c}",
                    "timestamp": 1000 + c * 100,
                    "file_path": f"f{r}.py",
                    "structural": {
                        "file_size_bytes": 100 + r * 13 + c * 7,
                        "lines_of_code": 10 + r,
                        "function_count": 1 + (r % 4),
                        "cyclomatic_complexity": 2 + (c % 3),
                        "max_nesting_depth": 1 + (r % 2),
                        "incoming_dependencies": r % 2,
                        "outgoing_dependencies": c % 2,
                        "coupling": r,
                        "num_dependents": c % 2,
                    },
                    "historical": {
                        "previous_change_count": c,
                        "historical_churn": c * 10 + r,
                        "contributor_count": 1 + (c % 2),
                        "time_since_last_change_secs": None,
                        "recent_change_frequency": 0.1 * c,
                        "historical_cochange_frequency": r % 3,
                    },
                    "temporal": {
                        "recent_change_sequence_count": c % 3,
                        "previous_files_changed_count": 1,
                        "change_window_count": c,
                        "avg_change_delay_seconds": None,
                        "change_order_count": r % 2,
                        "propagation_frequency": r % 2,
                        "followup_frequency": c % 2,
                        "historical_rework_frequency": (r + c) % 2,
                    },
                    "label": int((r * 3 + c) % 4 == 0),
                }
            )
    return rows


def _write_dataset(tmp_path, rows):
    path = tmp_path / "dataset.jsonl"
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")
    return path


def test_benchmark_record_schema_is_stable(tmp_path):
    path = _write_dataset(tmp_path, _synthetic_rows())
    records = benchmark.benchmark_repo("synth", path, "python")
    assert len(records) == 3
    for record in records:
        assert REQUIRED_BENCHMARK_KEYS <= set(record), set(record)
        total = record["positive_labels"] + record["negative_labels"]
        assert record["positive_rate"] == record["positive_labels"] / total
        assert record["test_positive_rate"] == record["test_positives"] / record["test_rows"]
        assert record["tp"] + record["fp"] + record["tn"] + record["fn"] == record["test_rows"]
        assert record["tp"] + record["fn"] == record["test_positives"]


def test_model_comparison_keys_and_envelope_shapes(tmp_path):
    bare = [{"repository": "r", "model": "m"}]
    bare_path = tmp_path / "bare.json"
    bare_path.write_text(json.dumps(bare), encoding="utf-8")
    recs, meta = load_records(bare_path)
    assert recs == bare and meta == {}
    envelope = {"metadata": {"a": 1}, "records": [{"repository": "r"}]}
    env_path = tmp_path / "env.json"
    env_path.write_text(json.dumps(envelope), encoding="utf-8")
    recs, meta = load_records(env_path)
    assert recs == [{"repository": "r"}] and meta == {"a": 1}
    assert tuple(COMPARISON_KEYS) == (
        "repository", "model", "precision", "recall", "f1",
        "roc_auc", "pr_auc", "prediction_time_seconds",
    )


def test_error_analysis_counts_match_predictions(tmp_path):
    path = _write_dataset(tmp_path, _synthetic_rows())
    out = tmp_path / "err.json"
    assert error_analysis.main(
        ["--repo", "synth", str(path), "--out", str(out),
         "--models", "logistic_regression"]
    ) == 0
    payload = json.loads(out.read_text(encoding="utf-8"))
    assert payload["records"], "expected at least one record"
    record = payload["records"][0]
    assert record["false_positive_count"] + record["false_negative_count"] >= 0
    assert len(record["false_positives"]) <= 25
    assert len(record["false_negatives"]) <= 25
    assert len(record["false_positives"]) == min(25, record["false_positive_count"])
    assert len(record["false_negatives"]) == min(25, record["false_negative_count"])
    for example in record["false_positives"] + record["false_negatives"]:
        assert {"commit_sha", "timestamp", "file_path", "predicted_probability",
                "actual_label", "predicted_label", "features"} <= set(example)
        assert len(example["features"]) == 23
    neg = record["test_negatives"]
    pos = record["test_positives"]
    assert record["false_positive_count"] <= neg
    assert record["false_negative_count"] <= pos
    denom_fp = record["false_positive_count"] + (neg - record["false_positive_count"])
    if denom_fp:
        assert abs(record["false_positive_rate"] - record["false_positive_count"] / denom_fp) < 1e-9
