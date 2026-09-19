from __future__ import annotations

from pathlib import Path
from typing import Any

from .compare import build_candidate_models
from .dataset import load_jsonl
from .evaluate import evaluate_model
from .features import rows_to_features


def ordered_commits(rows: list[dict[str, Any]]) -> list[str]:
    """Return commit SHAs ordered by (timestamp, sha)."""
    ordered = sorted(
        rows,
        key=lambda row: (
            int(row["timestamp"]),
            str(row["commit_sha"]),
        ),
    )
    commits: list[str] = []
    seen: set[str] = set()
    for row in ordered:
        sha = str(row["commit_sha"])
        if sha not in seen:
            seen.add(sha)
            commits.append(sha)
    return commits


def build_time_windows(
    rows: list[dict[str, Any]],
    n_windows: int = 3,
) -> list[tuple[list[str], list[str]]]:
    """Split ordered commits into expanding train / next-fold test windows.

    Commits are never split across train and test. Train always contains
    only commits strictly before the test period.
    """
    if n_windows < 1:
        raise ValueError("n_windows must be >= 1")

    commits = ordered_commits(rows)
    n_commits = len(commits)
    if n_commits < n_windows + 1:
        raise ValueError(
            f"Need at least {n_windows + 1} commits for {n_windows} windows, "
            f"got {n_commits}"
        )

    n_folds = n_windows + 1
    fold_size = n_commits // n_folds
    remainder = n_commits % n_folds

    folds: list[list[str]] = []
    start = 0
    for i in range(n_folds):
        size = fold_size + (1 if i < remainder else 0)
        folds.append(commits[start : start + size])
        start += size

    windows: list[tuple[list[str], list[str]]] = []
    for i in range(n_windows):
        train_commits = [sha for fold in folds[: i + 1] for sha in fold]
        test_commits = folds[i + 1]
        windows.append((train_commits, test_commits))

    return windows


def evaluate_time_windows(
    rows: list[dict[str, Any]],
    n_windows: int = 3,
) -> list[dict[str, Any]]:
    """Evaluate all candidates on progressively newer test periods."""
    windows = build_time_windows(rows, n_windows=n_windows)

    by_commit: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        by_commit.setdefault(str(row["commit_sha"]), []).append(row)

    results: list[dict[str, Any]] = []
    for window_index, (train_commits, test_commits) in enumerate(windows):
        train_rows = [r for sha in train_commits for r in by_commit.get(sha, [])]
        test_rows = [r for sha in test_commits for r in by_commit.get(sha, [])]
        if not train_rows or not test_rows:
            continue

        train_stamps = [int(r["timestamp"]) for r in train_rows]
        test_stamps = [int(r["timestamp"]) for r in test_rows]
        X_train, y_train = rows_to_features(train_rows)
        X_test, y_test = rows_to_features(test_rows)

        for name, model in build_candidate_models().items():
            model.fit(X_train, y_train)
            metrics = evaluate_model(model, X_test, y_test)
            results.append(
                {
                    "model": name,
                    "window": window_index,
                    "train_start": min(train_stamps),
                    "train_end": max(train_stamps),
                    "test_start": min(test_stamps),
                    "test_end": max(test_stamps),
                    "train_rows": len(train_rows),
                    "test_rows": len(test_rows),
                    "precision": metrics["precision"],
                    "recall": metrics["recall"],
                    "f1": metrics["f1"],
                    "roc_auc": metrics["roc_auc"],
                    "pr_auc": metrics["pr_auc"],
                }
            )

    return results


def evaluate_time_windows_from_path(
    dataset_path: str | Path,
    n_windows: int = 3,
) -> list[dict[str, Any]]:
    """Load JSONL then run time-based evaluation."""
    return evaluate_time_windows(load_jsonl(dataset_path), n_windows=n_windows)
