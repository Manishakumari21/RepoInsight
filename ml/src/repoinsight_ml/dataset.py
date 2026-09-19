from __future__ import annotations
from dataclasses import dataclass

import json
from pathlib import Path
from typing import Any


REQUIRED_TOP_LEVEL_FIELDS = {
    "commit_sha",
    "timestamp",
    "file_path",
    "structural",
    "historical",
    "temporal",
    "label",
}


def load_jsonl(path: str | Path) -> list[dict[str, Any]]:
    """Load and validate the Phase 5 dataset JSONL file."""
    path = Path(path)

    if not path.is_file():
        raise FileNotFoundError(f"Dataset not found: {path}")

    rows: list[dict[str, Any]] = []

    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()

            if not line:
                continue

            try:
                row = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(
                    f"Invalid JSON at line {line_number}: {exc}"
                ) from exc

            if not isinstance(row, dict):
                raise ValueError(
                    f"Line {line_number}: expected a JSON object"
                )

            missing = REQUIRED_TOP_LEVEL_FIELDS - row.keys()
            if missing:
                raise ValueError(
                    f"Line {line_number}: missing fields: "
                    f"{sorted(missing)}"
                )

            if row["label"] not in (0, 1):
                raise ValueError(
                    f"Line {line_number}: label must be 0 or 1"
                )

            rows.append(row)

    if not rows:
        raise ValueError(f"Dataset is empty: {path}")

    return rows


@dataclass(frozen=True)
class DatasetSplit:
    train: list[dict[str, Any]]
    validation: list[dict[str, Any]]
    test: list[dict[str, Any]]


def split_chronologically(
    rows: list[dict[str, Any]],
    train_ratio: float = 0.70,
    validation_ratio: float = 0.15,
) -> DatasetSplit:
    """Split dataset chronologically while keeping commits intact."""

    if not 0 < train_ratio < 1:
        raise ValueError("train_ratio must be between 0 and 1")

    if not 0 <= validation_ratio < 1:
        raise ValueError("validation_ratio must be between 0 and 1")

    if train_ratio + validation_ratio >= 1:
        raise ValueError(
            "train_ratio + validation_ratio must be less than 1"
        )

    ordered = sorted(
        rows,
        key=lambda row: (
            int(row["timestamp"]),
            str(row["commit_sha"]),
            str(row["file_path"]),
        ),
    )

    commit_order: list[str] = []
    seen_commits: set[str] = set()

    for row in ordered:
        commit_sha = str(row["commit_sha"])

        if commit_sha not in seen_commits:
            seen_commits.add(commit_sha)
            commit_order.append(commit_sha)

    commit_count = len(commit_order)

    if commit_count < 3:
        return DatasetSplit(
            train=ordered,
            validation=[],
            test=[],
        )

    train_count = max(1, int(commit_count * train_ratio))
    validation_count = max(
        1,
        int(commit_count * validation_ratio),
    )

    if train_count + validation_count >= commit_count:
        validation_count = max(1, commit_count - train_count - 1)

    train_commits = set(commit_order[:train_count])
    validation_commits = set(
        commit_order[train_count : train_count + validation_count]
    )
    test_commits = set(
        commit_order[train_count + validation_count :]
    )

    train = [row for row in ordered if row["commit_sha"] in train_commits]
    validation = [
        row for row in ordered if row["commit_sha"] in validation_commits
    ]
    test = [row for row in ordered if row["commit_sha"] in test_commits]

    return DatasetSplit(
        train=train,
        validation=validation,
        test=test,
    )