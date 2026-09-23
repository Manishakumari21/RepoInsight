from __future__ import annotations

from pathlib import Path
from typing import Any

from .compare import build_candidate_models
from .dataset import load_jsonl
from .evaluate import evaluate_model
from .features import rows_to_features


def ordered_commits(rows: list[dict[str, Any]]) -> list[str]:
    import numpy as np

    if not rows:
        return []
    timestamps = np.array(
        [int(row["timestamp"]) for row in rows], dtype=np.int64
    )
    shas = np.array([str(row["commit_sha"]) for row in rows], dtype=object)
    order = np.lexsort((shas, timestamps))
    commits: list[str] = []
    seen: set[str] = set()
    for index in order.tolist():
        sha = str(rows[int(index)]["commit_sha"])
        if sha not in seen:
            seen.add(sha)
            commits.append(sha)
    return commits


def build_time_windows(
    rows: list[dict[str, Any]],
    n_windows: int = 3,
) -> list[tuple[list[str], list[str]]]:
    import numpy as np
    from sklearn.model_selection import TimeSeriesSplit, train_test_split

    if n_windows < 1:
        raise ValueError("n_windows must be >= 1")

    commits = ordered_commits(rows)
    n_commits = len(commits)
    if n_commits < n_windows + 1:
        raise ValueError(
            f"Need at least {n_windows + 1} commits for {n_windows} windows, "
            f"got {n_commits}"
        )

    commit_index = np.arange(n_commits)
    windows: list[tuple[list[str], list[str]]] = []
    if n_windows == 1:
        test_size = max(1, n_commits // 2)
        train_idx, test_idx = train_test_split(
            commit_index, test_size=test_size, shuffle=False
        )
        windows.append(
            (
                [commits[int(i)] for i in np.asarray(train_idx).tolist()],
                [commits[int(i)] for i in np.asarray(test_idx).tolist()],
            )
        )
        return windows
    splitter = TimeSeriesSplit(n_splits=n_windows)
    for train_idx, test_idx in splitter.split(commit_index):
        train_commits = [commits[int(i)] for i in train_idx.tolist()]
        test_commits = [commits[int(i)] for i in test_idx.tolist()]
        windows.append((train_commits, test_commits))

    return windows


def evaluate_time_windows(
    rows: list[dict[str, Any]],
    n_windows: int = 3,
) -> list[dict[str, Any]]:
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
    return evaluate_time_windows(load_jsonl(dataset_path), n_windows=n_windows)
