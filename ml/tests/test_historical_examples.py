from pathlib import Path

import pytest

from repoinsight_ml.dataset import load_jsonl
from repoinsight_ml.historical_examples import find_historical_examples

DATASET = Path(__file__).resolve().parents[1] / "data" / "dataset.jsonl"


def _rows():
    return [
        {"commit_sha": "a", "timestamp": 100, "file_path": "x.py"},
        {"commit_sha": "b", "timestamp": 200, "file_path": "x.py"},
        {"commit_sha": "b", "timestamp": 200, "file_path": "y.py"},
        {"commit_sha": "c", "timestamp": 300, "file_path": "x.py"},
    ]


def test_prior_and_cochange_found():
    out = find_historical_examples(
        _rows(), commit_sha="c", timestamp=300, file_path="x.py"
    )
    kinds = {(e["commit_sha"], e["event_type"]) for e in out}
    assert ("a", "prior_change") in kinds
    assert ("b", "prior_change") in kinds
    cochange = [e for e in out if e["event_type"] == "co_change"]
    assert len(cochange) == 1
    assert cochange[0]["related_files"] == ["y.py"]


def test_strict_chronology_no_future_no_self():
    out = find_historical_examples(
        _rows(), commit_sha="b", timestamp=200, file_path="x.py"
    )
    assert all(e["timestamp"] < 200 for e in out)
    assert all(e["commit_sha"] != "b" for e in out)


def test_no_fabrication_for_unknown_file():
    assert (
        find_historical_examples(
            _rows(), commit_sha="c", timestamp=300, file_path="nope.py"
        )
        == []
    )


def test_limit_and_determinism():
    first = find_historical_examples(
        _rows(), commit_sha="c", timestamp=300, file_path="x.py", limit=2
    )
    assert len(first) == 2
    assert first == find_historical_examples(
        _rows(), commit_sha="c", timestamp=300, file_path="x.py", limit=2
    )
    with pytest.raises(ValueError, match="limit"):
        find_historical_examples(
            _rows(), commit_sha="c", timestamp=300, file_path="x.py", limit=0
        )


def test_real_dataset_respects_chronology():
    rows = load_jsonl(DATASET)
    target = max(rows, key=lambda r: int(r["timestamp"]))
    out = find_historical_examples(
        rows,
        commit_sha=str(target["commit_sha"]),
        timestamp=int(target["timestamp"]),
        file_path=str(target["file_path"]),
        limit=5,
    )
    assert all(
        int(e["timestamp"]) < int(target["timestamp"]) for e in out
    )
    assert all(set(e) == {"commit_sha", "timestamp", "file_path",
                          "event_type", "related_files"} for e in out)
