"""Change Impact Simulator: planned-change impact ranking over dataset rows.

A developer describes a *planned* change in natural language and the
simulator estimates which repository files would need attention before
anything is modified. This mirrors ``backend/src/analysis/impact.rs``:
deterministic intent extraction plus a capability map plus normalized
weighted scoring — an explainable baseline, not a novel ML technique.

Dataset rows carry ``commit_sha``/``timestamp``/``file_path`` but no file
contents and no dependency edges, so the ML side scores semantic,
historical, and path-kind signals only. Structural dependency evidence is
a backend-only signal (documented gap, not a silent omission).

Chronological discipline matches the rest of the pipeline: every test
target is scored from rows strictly before its timestamp, with the commit
message standing in for the planned-change description.
"""

from __future__ import annotations

from collections import defaultdict
from typing import Any

OPERATION_KEYWORDS = [
    ("replace", "replace"),
    ("migrate", "replace"),
    ("migration", "replace"),
    ("switch", "replace"),
    ("convert", "replace"),
    ("add", "add"),
    ("introduce", "add"),
    ("implement", "add"),
    ("support", "add"),
    ("create", "add"),
    ("remove", "remove"),
    ("delete", "remove"),
    ("drop", "remove"),
    ("deprecate", "remove"),
    ("refactor", "refactor"),
    ("restructure", "refactor"),
    ("cleanup", "refactor"),
    ("upgrade", "upgrade"),
    ("update", "upgrade"),
    ("modernize", "upgrade"),
    ("bump", "upgrade"),
    ("fix", "fix"),
]

DOMAIN_ALIASES = {
    "authentication": [
        "auth", "authentication", "login", "logout", "session", "token",
        "jwt", "oauth", "oauth2", "password", "sso", "credentials",
    ],
    "authorization": [
        "authorization", "permissions", "roles", "rbac", "acl", "policy",
    ],
    "payments": [
        "payment", "payments", "stripe", "razorpay", "billing", "invoice",
        "checkout", "subscription",
    ],
    "database": [
        "database", "postgres", "postgresql", "mysql", "sqlite", "mongo",
        "mongodb", "migration", "migrations", "schema", "query",
    ],
    "api": [
        "api", "rest", "graphql", "endpoint", "route", "controller",
        "middleware", "response",
    ],
    "caching": ["cache", "caching", "redis", "memcached"],
    "frontend": [
        "frontend", "ui", "page", "component", "redux", "zustand",
        "store", "theme",
    ],
    "users": ["user", "users", "profile", "account"],
    "configuration": ["config", "configuration", "settings", "env"],
}

TECHNOLOGIES = [
    "jwt", "oauth", "oauth2", "postgres", "postgresql", "mysql", "sqlite",
    "mongodb", "mongo", "redis", "stripe", "razorpay", "redux", "zustand",
    "rest", "graphql",
]

STOPWORDS = {
    "the", "a", "an", "to", "of", "in", "on", "for", "with", "and", "or",
    "from", "by", "as", "is", "are", "be", "our", "we", "it", "this",
    "that", "into", "all", "new", "old", "current", "existing",
}

DEFAULT_WEIGHTS = {
    "semantic": 0.45,
    "historical": 0.35,
    "kind": 0.20,
}


def _tokenize(text: str) -> list[str]:
    tokens: list[str] = []
    current = ""
    for char in text.lower():
        if char.isalnum():
            current += char
        elif current:
            tokens.append(current)
            current = ""
    if current:
        tokens.append(current)
    return [t for t in tokens if len(t) > 2 and t not in STOPWORDS]


def extract_intent(description: str) -> dict[str, Any]:
    """Deterministic change-intent extraction (no LLM, no network)."""
    lower = description.lower()
    operation = "general"
    for keyword, canonical in OPERATION_KEYWORDS:
        if keyword in lower:
            operation = canonical
            break
    tokens = _tokenize(description)
    token_set = set(tokens)

    domains: list[str] = []
    domain_terms: set[str] = set()
    for domain, aliases in DOMAIN_ALIASES.items():
        matched = False
        for alias in aliases:
            if alias in token_set or alias in lower:
                domain_terms.add(alias)
                matched = True
        if matched:
            domains.append(domain)
    domains.sort()

    techs = sorted({t for t in TECHNOLOGIES if t in token_set})
    current = techs[0] if techs else None
    target = None
    if operation == "replace" and len(techs) >= 2:
        ordered = sorted(techs, key=lambda t: lower.find(t))
        current, target = ordered[0], ordered[1]
    elif len(techs) >= 2:
        target = techs[1]

    terms = sorted(set(tokens) | domain_terms | set(techs))
    return {
        "operation": operation,
        "domains": domains,
        "terms": terms,
        "current_technology": current,
        "target_technology": target,
    }


def _path_tokens(path: str) -> set[str]:
    tokens: set[str] = set()
    for segment in path.lower().split("/"):
        stem = segment.split(".")[0]
        for part in stem.replace("-", " ").replace("_", " ").split():
            if len(part) > 1:
                tokens.add(part)
    return tokens


def _path_score(path: str, terms: list[str]) -> tuple[float, list[str]]:
    if not terms:
        return 0.0, []
    tokens = _path_tokens(path)
    matched = [
        term
        for term in terms
        if term in tokens
        or any(
            len(token) > 3
            and len(term) > 3
            and (token in term or term in token)
            for token in tokens
        )
    ]
    return len(matched) / max(1, min(len(terms), 4)), sorted(matched)


def classify_path(path: str) -> str:
    lower = path.lower()
    name = lower.rsplit("/", 1)[-1]
    if (
        "/test" in lower
        or "test" in name.split(".")
        or ".test." in name
        or name.startswith("test_")
    ):
        return "test"
    if (
        "config" in lower
        or "setting" in lower
        or name.startswith(".env")
        or name.endswith((".yml", ".yaml", ".toml", ".ini"))
    ):
        return "configuration"
    if lower.startswith("docs/") or "/docs/" in lower or name.endswith(".md"):
        return "documentation"
    return "source"


def capability_map(file_paths: list[str]) -> list[dict[str, Any]]:
    """Group files by best-matching domain, else top-level directory."""
    groups: dict[str, list[str]] = defaultdict(list)
    for path in file_paths:
        lower = path.lower()
        best: str | None = None
        best_hits = 0
        for domain, aliases in DOMAIN_ALIASES.items():
            hits = sum(1 for alias in aliases if alias in lower)
            if hits > best_hits:
                best, best_hits = domain, hits
        if best is None:
            top = path.split("/")[0].split(".")[0].strip("_.") or "general"
            best = top[:1].upper() + top[1:] if top != "general" else top
        groups[best].append(path)
    capabilities = [
        {"name": name, "files": sorted(files)}
        for name, files in groups.items()
    ]
    capabilities.sort(key=lambda c: (-len(c["files"]), c["name"]))
    return capabilities


def _commit_files(rows: list[dict[str, Any]]) -> dict[str, set[str]]:
    by_commit: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        by_commit[str(row["commit_sha"])].add(str(row["file_path"]))
    return by_commit


def _cochange(by_commit: dict[str, set[str]], first: str, second: str) -> int:
    return sum(
        1 for files in by_commit.values() if first in files and second in files
    )


def simulate_impact(
    description: str,
    file_paths: list[str],
    prefix_rows: list[dict[str, Any]] | None = None,
    *,
    weights: dict[str, float] | None = None,
    max_results: int = 20,
    include_low_confidence: bool = False,
) -> dict[str, Any]:
    """Score every file against the extracted intent.

    Signals are normalized to 0..1 (semantic match, historical coupling to
    directly-matched files, path-kind relevance) and combined with
    configurable weights. ``prefix_rows`` bounds the historical signal to
    repository state before time T; pass ``None`` to skip history.
    """
    intent = extract_intent(description)
    terms = intent["terms"]
    active = dict(DEFAULT_WEIGHTS)
    if weights:
        active.update(weights)
    total = sum(active.values()) or 1.0
    active = {key: value / total for key, value in active.items()}

    by_commit = _commit_files(prefix_rows) if prefix_rows else {}
    change_counts: dict[str, int] = defaultdict(int)
    for files in by_commit.values():
        for path in files:
            change_counts[path] += 1

    direct = {
        path
        for path in file_paths
        if _path_score(path, terms)[0] >= 0.5
    }

    items: list[dict[str, Any]] = []
    for path in file_paths:
        semantic, _ = _path_score(path, terms)
        kind = classify_path(path)
        kind_score = 1.0 if kind != "source" and semantic > 0.0 else 0.0
        best_prob = 0.0
        best_peer: str | None = None
        best_count = 0
        for hit in direct:
            if hit == path:
                continue
            count = _cochange(by_commit, path, hit)
            changes = change_counts.get(hit, 0)
            prob = count / changes if changes else 0.0
            if prob > best_prob:
                best_prob, best_peer, best_count = prob, hit, count
        historical = min(1.0, best_prob)
        score = (
            active["semantic"] * semantic
            + active["historical"] * historical
            + active["kind"] * kind_score
        )
        if semantic >= 0.5:
            score = max(score, 0.6)
        score = min(1.0, max(0.0, score))

        evidence: list[str] = []
        path_hit, matched = _path_score(path, terms)
        if matched:
            evidence.append(
                f"Path matches change concept: {', '.join(matched[:6])}."
            )
        if best_count:
            evidence.append(
                f"Changed with {best_peer} in {best_count} "
                f"historical commit{'s' if best_count != 1 else ''}."
            )

        categories: list[str] = []
        if semantic >= 0.5:
            categories.append("direct")
        if historical >= 0.3:
            categories.append("historically_coupled")
        if kind_score:
            categories.append(kind)
        if not categories:
            categories.append("indirect")

        level = (
            "high"
            if score >= 0.5
            else "medium"
            if score >= 0.15
            else "low"
        )
        items.append(
            {
                "path": path,
                "level": level,
                "score": score,
                "categories": categories,
                "evidence": evidence,
            }
        )

    items = [item for item in items if item["score"] >= 0.01]
    if not include_low_confidence:
        items = [item for item in items if item["level"] != "low"]
    items.sort(key=lambda item: (-item["score"], item["path"]))
    return {
        "change_description": description,
        "intent": intent,
        "impact": items[: max(1, max_results)],
        "capabilities": capability_map(file_paths),
        "model": {"name": "impact_baseline", "calibrated": False},
    }


def evaluate_impact_ranking(
    rows: list[dict[str, Any]],
    messages: dict[str, str],
    *,
    k: int = 5,
    baseline: str = "combined",
) -> dict[str, Any]:
    """Chronological evaluation with the commit message as description.

    Test targets are the last fifth of commits (minimum thresholds apply).
    ``baseline`` selects a signal mask: ``combined``, ``keyword-only``,
    or ``cochange-only``.
    """
    k = max(1, k)
    ordered = sorted(
        {str(row["commit_sha"]) for row in rows},
        key=lambda sha: (
            min(int(r["timestamp"]) for r in rows if str(r["commit_sha"]) == sha),
            sha,
        ),
    )
    if len(ordered) < 5:
        return {
            "precision_at_k": None,
            "recall_at_k": None,
            "f1_at_k": None,
            "directory_accuracy": None,
            "evaluated_targets": 0,
            "k": k,
            "baseline": baseline,
        }
    times = {
        sha: min(int(r["timestamp"]) for r in rows if str(r["commit_sha"]) == sha)
        for sha in ordered
    }
    by_commit = _commit_files(rows)
    weights = {
        "combined": None,
        "keyword-only": {"semantic": 1.0, "historical": 0.0, "kind": 0.0},
        "cochange-only": {"semantic": 0.0, "historical": 1.0, "kind": 0.0},
    }.get(baseline, None)

    precisions: list[float] = []
    recalls: list[float] = []
    dir_scores: list[float] = []
    evaluated = 0
    for sha in ordered[len(ordered) * 4 // 5:]:
        message = messages.get(sha, "")
        actual = sorted(by_commit[sha])
        if not message.strip() or not actual or len(actual) > 15:
            continue
        prefix = [r for r in rows if int(r["timestamp"]) < times[sha]]
        if not prefix:
            continue
        files = sorted({str(r["file_path"]) for r in rows})
        ranked = [
            item["path"]
            for item in simulate_impact(
                message,
                files,
                prefix,
                weights=weights,
                max_results=k,
                include_low_confidence=True,
            )["impact"][:k]
        ]
        if not ranked:
            continue
        evaluated += 1
        actual_set = set(actual)
        hits = sum(1 for path in ranked if path in actual_set)
        precisions.append(hits / len(ranked))
        recalls.append(hits / len(actual))
        predicted_dirs = {
            p.rsplit("/", 1)[0] if "/" in p else "" for p in ranked
        }
        actual_dirs = {
            p.rsplit("/", 1)[0] if "/" in p else "" for p in actual
        }
        dir_scores.append(
            sum(1 for d in actual_dirs if d in predicted_dirs) / len(actual_dirs)
            if actual_dirs
            else 1.0
        )

    def mean(values: list[float]) -> float | None:
        return sum(values) / len(values) if values else None

    precision, recall = mean(precisions), mean(recalls)
    f1 = (
        2 * precision * recall / (precision + recall)
        if precision is not None
        and recall is not None
        and precision + recall > 0
        else None
    )
    return {
        "precision_at_k": precision,
        "recall_at_k": recall,
        "f1_at_k": f1,
        "directory_accuracy": mean(dir_scores),
        "evaluated_targets": evaluated,
        "k": k,
        "baseline": baseline,
    }


def compare_impact_models(
    rows: list[dict[str, Any]], messages: dict[str, str], *, k: int = 5
) -> list[dict[str, Any]]:
    """Report keyword-only, co-change-only, and combined impact models.

    Answers whether history adds predictive information beyond filename
    matching. No ranking is applied between the reported rows.
    """
    return [
        {
            "model": f"impact_{baseline}",
            **evaluate_impact_ranking(
                rows, messages, k=k, baseline=baseline
            ),
        }
        for baseline in ("keyword-only", "cochange-only", "combined")
    ]
