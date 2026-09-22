from __future__ import annotations

from typing import Any


def find_historical_examples(
    rows: list[dict[str, Any]],
    *,
    commit_sha: str,
    timestamp: int,
    file_path: str,
    limit: int = 5,
) -> list[dict[str, Any]]:
    """Find real past events relevant to a predicted file.

    Only rows with a timestamp strictly before the target are used, so
    explaining a historical prediction never leaks future commits.
    Event types: "prior_change" (the same file changed before) and
    "co_change" (other files changed in the same earlier commits).
    Returns an empty list when nothing relevant exists — never invented.
    """
    if limit < 1:
        raise ValueError("limit must be >= 1")
    if not file_path:
        return []

    timestamp = int(timestamp)
    prior = [
        row
        for row in rows
        if int(row.get("timestamp", 0)) < timestamp
    ]
    by_commit: dict[str, list[dict[str, Any]]] = {}
    for row in prior:
        by_commit.setdefault(str(row.get("commit_sha", "")), []).append(row)

    examples: list[dict[str, Any]] = []
    for sha, commit_rows in by_commit.items():
        files = sorted({str(row.get("file_path", "")) for row in commit_rows})
        if file_path not in files:
            continue
        stamps = [int(row.get("timestamp", 0)) for row in commit_rows]
        stamp = max(stamps) if stamps else 0
        examples.append(
            {
                "commit_sha": sha,
                "timestamp": stamp,
                "file_path": file_path,
                "event_type": "prior_change",
                "related_files": [],
            }
        )
        related = [name for name in files if name != file_path]
        if related:
            examples.append(
                {
                    "commit_sha": sha,
                    "timestamp": stamp,
                    "file_path": file_path,
                    "event_type": "co_change",
                    "related_files": related,
                }
            )

    examples.sort(
        key=lambda item: (
            -int(item["timestamp"]),
            str(item["commit_sha"]),
            str(item["event_type"]),
        )
    )
    return examples[:limit]
