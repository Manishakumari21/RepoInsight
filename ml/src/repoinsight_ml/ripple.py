"""Change Ripple Forecasting: pairwise follow-up ranking over dataset rows.

An explainable integration of co-change and temporal follow-up signals
reconstructed from leakage-safe dataset rows. Structural dependency edges
live in the backend (``DependencyEdge``); the ML side compares what history
alone can predict:

* baseline: historical co-change only,
* full: co-change + temporal follow-up.

Chronological discipline matches the rest of the pipeline: every test
target is scored from rows strictly before its timestamp.
"""

from __future__ import annotations

from collections import defaultdict
from typing import Any

FOLLOWUP_WINDOW_SECS = 7 * 24 * 60 * 60


def ordered_commits(rows: list[dict[str, Any]]) -> list[str]:
    ordered = sorted(
        rows,
        key=lambda row: (int(row["timestamp"]), str(row["commit_sha"])),
    )
    commits: list[str] = []
    seen: set[str] = set()
    for row in ordered:
        sha = str(row["commit_sha"])
        if sha not in seen:
            seen.add(sha)
            commits.append(sha)
    return commits


def _commit_files(rows: list[dict[str, Any]]) -> dict[str, set[str]]:
    by_commit: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        by_commit[str(row["commit_sha"])].add(str(row["file_path"]))
    return by_commit


def _commit_times(rows: list[dict[str, Any]]) -> dict[str, int]:
    times: dict[str, int] = {}
    for row in rows:
        sha = str(row["commit_sha"])
        stamp = int(row["timestamp"])
        if sha not in times:
            times[sha] = stamp
    return times


def build_ripple_scores(
    rows: list[dict[str, Any]],
    source: str,
    *,
    use_temporal: bool = True,
    window_secs: int = FOLLOWUP_WINDOW_SECS,
) -> list[dict[str, Any]]:
    """Rank candidate follow-up files for ``source`` from prefix rows.

    ``cochange_probability`` is P(B changes | A changes) over commits.
    ``follow_probability`` is P(B in a later commit within the window).
    ``combined_score`` is the mean of the enabled signals (equal-weight
    baseline; backend ``ripple.rs`` carries the configurable weighted
    version with structural + recent signals).
    """
    by_commit = _commit_files(rows)
    times = _commit_times(rows)
    ordered = sorted(by_commit, key=lambda sha: (times[sha], sha))

    source_commits = [sha for sha in ordered if source in by_commit[sha]]
    source_count = len(source_commits)
    if source_count == 0:
        return []

    candidates = sorted(
        {path for files in by_commit.values() for path in files} - {source}
    )
    scored: list[dict[str, Any]] = []
    for target in candidates:
        co_count = sum(
            1 for sha in source_commits if target in by_commit[sha]
        )
        co_prob = co_count / source_count
        follow_count = 0
        if use_temporal:
            for sha in source_commits:
                for later in ordered:
                    if later == sha:
                        continue
                    delay = times[later] - times[sha]
                    if delay <= 0 or delay > window_secs:
                        continue
                    if target in by_commit[later] and source not in by_commit[later]:


                        follow_count += 1
                        break
        follow_prob = follow_count / source_count
        signals = [co_prob] + ([follow_prob] if use_temporal else [])
        scored.append(
            {
                "file": target,
                "cochange_probability": co_prob,
                "follow_probability": follow_prob,
                "combined_score": sum(signals) / len(signals),
            }
        )
    scored.sort(key=lambda entry: (-entry["combined_score"], entry["file"]))
    return scored


def evaluate_ripple_ranking(
    rows: list[dict[str, Any]],
    *,
    k: int = 5,
    use_temporal: bool = True,
) -> dict[str, Any]:
    """Chronological P@K / R@K / MRR / MAP for ripple ranking.

    Last 30% of commits (minimum 2) form test targets: source is the first
    file (sorted) of a multi-file test commit, ground truth the rest. Each
    target is scored from rows strictly before its timestamp.
    """
    k = max(1, k)
    commits = ordered_commits(rows)
    if len(commits) < 3:
        return {
            "precision_at_k": None,
            "recall_at_k": None,
            "mrr": None,
            "map": None,
            "evaluated_targets": 0,
            "k": k,
        }
    split_at = len(commits) * 7 // 10
    by_commit = _commit_files(rows)
    times = _commit_times(rows)
    precisions: list[float] = []
    recalls: list[float] = []
    reciprocal_ranks: list[float] = []
    average_precisions: list[float] = []
    evaluated = 0
    for sha in commits[split_at:]:
        files = sorted(by_commit[sha])
        if len(files) < 2:
            continue
        target_ts = times[sha]
        prefix = [row for row in rows if int(row["timestamp"]) < target_ts]
        if not prefix:
            continue
        source, ground = files[0], set(files[1:])
        ranked = [
            entry["file"]
            for entry in build_ripple_scores(
                prefix, source, use_temporal=use_temporal
            )[:k]
        ]
        if not ranked:
            continue
        evaluated += 1
        hits = sum(1 for path in ranked if path in ground)
        precisions.append(hits / len(ranked))
        recalls.append(hits / len(ground))
        first = next(
            (index for index, path in enumerate(ranked) if path in ground),
            None,
        )
        reciprocal_ranks.append(1.0 / (first + 1) if first is not None else 0.0)
        relevant_seen = 0
        precision_sum = 0.0
        for index, path in enumerate(ranked):
            if path in ground:
                relevant_seen += 1
                precision_sum += relevant_seen / (index + 1)
        average_precisions.append(
            precision_sum / len(ground) if relevant_seen else 0.0
        )
    mean = lambda values: (
        sum(values) / len(values) if values else None
    )
    return {
        "precision_at_k": mean(precisions),
        "recall_at_k": mean(recalls),
        "mrr": mean(reciprocal_ranks),
        "map": mean(average_precisions),
        "evaluated_targets": evaluated,
        "k": k,
    }


def compare_ripple_models(
    rows: list[dict[str, Any]], *, k: int = 5
) -> list[dict[str, Any]]:
    """Baseline (co-change only) vs full (co-change + temporal) ripple.

    Answers the research question: does temporal change history add
    predictive information beyond simple historical co-change? No ranking
    is applied; both rows are reported.
    """
    return [
        {
            "model": "ripple_cochange_only",
            **evaluate_ripple_ranking(rows, k=k, use_temporal=False),
        },
        {
            "model": "ripple_full",
            **evaluate_ripple_ranking(rows, k=k, use_temporal=True),
        },
    ]
