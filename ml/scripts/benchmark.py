from __future__ import annotations

import argparse
import datetime
import json
import sys
import time
from collections import Counter
from pathlib import Path

import numpy
import sklearn
from sklearn.metrics import confusion_matrix

ML_SRC = Path(__file__).resolve().parents[1] / "src"
sys.path.insert(0, str(ML_SRC))

from repoinsight_ml.compare import build_candidate_models
from repoinsight_ml.dataset import load_jsonl, split_chronologically
from repoinsight_ml.evaluate import evaluate_model
from repoinsight_ml.features import rows_to_features

TRAIN_RATIO = 0.70
VALIDATION_RATIO = 0.15

MODEL_PARAM_KEYS = (
    "n_estimators",
    "max_iter",
    "class_weight",
    "random_state",
    "n_jobs",
)


def label_counts(rows: list[dict]) -> tuple[int, int]:
    counts = Counter(int(row["label"]) for row in rows)
    return counts[1], counts[0]


def positive_rate(positives: int, negatives: int) -> float:
    total = positives + negatives
    return positives / total if total else 0.0


def relative_dataset_ref(path: Path) -> str:
    """Dataset reference without machine-specific absolute paths."""
    try:
        return str(path.relative_to(Path.cwd()))
    except ValueError:
        return path.name


def model_configs() -> dict[str, dict]:
    configs: dict[str, dict] = {}
    for name, model in build_candidate_models().items():
        classifier = model.named_steps["classifier"]
        params = classifier.get_params()
        configs[name] = {
            "class": type(classifier).__name__,
            "params": {
                key: params[key] for key in MODEL_PARAM_KEYS if key in params
            },
        }
    return configs


def run_metadata(repos: list[tuple[str, str, str]]) -> dict:
    return {
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "python": sys.version.split()[0],
        "sklearn": sklearn.__version__,
        "numpy": numpy.__version__,
        "split": {
            "strategy": "chronological_by_commit",
            "train_ratio": TRAIN_RATIO,
            "validation_ratio": VALIDATION_RATIO,
        },
        "models": model_configs(),
        "datasets": [
            {"repository": name, "path": dataset, "language": language}
            for name, dataset, language in repos
        ],
    }


def benchmark_repo(name: str, path: Path, language: str) -> list[dict]:
    t0 = time.perf_counter()
    rows = load_jsonl(path)
    split = split_chronologically(rows)
    if not split.train:
        raise ValueError(f"{name}: training split is empty")
    if not split.test:
        raise ValueError(f"{name}: test split is empty")
    X_train, y_train = rows_to_features(split.train)
    X_test, y_test = rows_to_features(split.test)
    analysis_time = time.perf_counter() - t0

    train_pos, train_neg = label_counts(split.train)
    val_pos, val_neg = label_counts(split.validation)
    test_pos, test_neg = label_counts(split.test)

    records: list[dict] = []
    for model_name, model in build_candidate_models().items():
        t1 = time.perf_counter()
        model.fit(X_train, y_train)
        metrics = evaluate_model(model, X_test, y_test)
        predictions = model.predict(X_test)
        tn, fp, fn, tp = (int(v) for v in confusion_matrix(y_test, predictions, labels=[0, 1]).ravel())
        prediction_time = time.perf_counter() - t1

        total_pos, total_neg = label_counts(rows)
        records.append(
            {
                "repository": name,
                "language": language,
                "dataset": relative_dataset_ref(path),
                "files": len({str(r["file_path"]) for r in rows}),
                "commits": len({str(r["commit_sha"]) for r in rows}),
                "dataset_rows": len(rows),
                "positive_labels": total_pos,
                "negative_labels": total_neg,
                "positive_rate": positive_rate(total_pos, total_neg),
                "training_rows": len(split.train),
                "training_positives": train_pos,
                "training_negatives": train_neg,
                "validation_rows": len(split.validation),
                "validation_positives": val_pos,
                "validation_negatives": val_neg,
                "test_rows": len(split.test),
                "test_positives": test_pos,
                "test_negatives": test_neg,
                "test_positive_rate": positive_rate(test_pos, test_neg),
                "model": model_name,
                "tp": tp,
                "fp": fp,
                "tn": tn,
                "fn": fn,
                "precision": metrics["precision"],
                "recall": metrics["recall"],
                "f1": metrics["f1"],
                "roc_auc": metrics["roc_auc"],
                "pr_auc": metrics["pr_auc"],
                "analysis_time_seconds": round(analysis_time, 3),
                "prediction_time_seconds": round(prediction_time, 3),
            }
        )
    return records


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo",
        nargs=3,
        metavar=("NAME", "DATASET_JSONL", "LANGUAGE"),
        action="append",
        required=True,
        help="One real repository dataset to benchmark (repeatable).",
    )
    parser.add_argument("--out", required=True, help="Output JSON path.")
    parser.add_argument(
        "--envelope",
        action="store_true",
        help="Wrap records with run metadata (versions, timestamp, configs).",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    repos = [(name, dataset, language) for name, dataset, language in args.repo]
    records: list[dict] = []
    for name, dataset, language in repos:
        records.extend(benchmark_repo(name, Path(dataset), language))
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    if args.envelope:
        payload: dict = {"metadata": run_metadata(repos), "records": records}
    else:
        payload = records
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {len(records)} benchmark records to {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
