"""Tests for the Change Impact Simulator (ML side)."""

from repoinsight_ml.impact import (
    capability_map,
    compare_impact_models,
    evaluate_impact_ranking,
    extract_intent,
    simulate_impact,
)


def _row(sha, timestamp, path):
    return {
        "commit_sha": sha,
        "timestamp": timestamp,
        "file_path": path,
        "structural": {},
        "historical": {},
        "temporal": {},
        "label": 0,
    }


def _history():
    return [
        _row("c1", 100, "src/auth/login.ts"),
        _row("c1", 100, "src/api/users.ts"),
        _row("c2", 200, "src/auth/login.ts"),
        _row("c3", 300, "src/api/users.ts"),
        _row("c4", 400, "src/auth/login.ts"),
        _row("c4", 400, "src/api/users.ts"),
        _row("c5", 500, "src/auth/login.ts"),
        _row("c5", 500, "src/api/users.ts"),
    ]


def test_intent_parses_replace_operation():
    intent = extract_intent("Replace JWT authentication with OAuth2")
    assert intent["operation"] == "replace"
    assert "authentication" in intent["domains"]
    assert intent["current_technology"] == "jwt"
    assert intent["target_technology"] == "oauth2"


def test_intent_handles_unknown_description():
    intent = extract_intent("zxqv blorp")
    assert intent["operation"] == "general"
    assert intent["domains"] == []
    assert intent["current_technology"] is None


def test_capability_map_groups_without_manual_input():
    capabilities = capability_map(
        ["src/auth/login.ts", "src/payments/stripe.ts"]
    )
    names = {entry["name"] for entry in capabilities}
    assert "authentication" in names
    assert "payments" in names


def test_semantic_matching_finds_auth_files():
    result = simulate_impact(
        "Replace JWT authentication with OAuth2",
        ["src/auth/login.ts", "src/payments/stripe.ts"],
        _history(),
    )
    paths = [item["path"] for item in result["impact"]]
    assert "src/auth/login.ts" in paths
    assert "src/payments/stripe.ts" not in paths
    login = next(
        item for item in result["impact"] if item["path"] == "src/auth/login.ts"
    )
    assert "direct" in login["categories"]


def test_historical_coupling_uses_prefix_only():
    rows = [
        _row("c1", 100, "src/auth/login.ts"),
        _row("c1", 100, "src/api/users.ts"),
    ]
    result = simulate_impact(
        "Refactor authentication",
        ["src/auth/login.ts", "src/api/users.ts"],
        rows,
    )
    users = next(
        item for item in result["impact"] if item["path"] == "src/api/users.ts"
    )
    assert "historically_coupled" in users["categories"]


def test_unknown_description_yields_nothing():
    result = simulate_impact("zxqv blorp", ["src/main.rs"], _history())
    assert result["impact"] == []


def test_chronological_evaluation_respects_time():
    rows = _history()
    messages = {sha: "auth fix" for sha in ("c1", "c2", "c3", "c4", "c5")}
    report = evaluate_impact_ranking(rows, messages, k=2)
    assert report["evaluated_targets"] >= 1
    assert report["precision_at_k"] is not None


def test_baseline_comparison_reports_all_models():
    rows = _history()
    messages = {sha: "auth fix" for sha in ("c1", "c2", "c3", "c4", "c5")}
    reports = compare_impact_models(rows, messages, k=2)
    assert [r["model"] for r in reports] == [
        "impact_keyword-only",
        "impact_cochange-only",
        "impact_combined",
    ]
