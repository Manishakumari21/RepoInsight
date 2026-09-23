"""Tests for run_impact_evaluation git-history loading (hermetic temp repo)."""

import importlib.util
import subprocess
from pathlib import Path

from repoinsight_ml.impact import compare_impact_models

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "run_impact_evaluation.py"


def load_script_module():
    spec = importlib.util.spec_from_file_location(
        "run_impact_evaluation", SCRIPT
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(repo: Path, *args: str) -> None:
    subprocess.run(
        ["git", "-c", "user.email=t@t", "-c", "user.name=t",
         "-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main",
         *args],
        cwd=repo,
        check=True,
        capture_output=True,
    )


def make_repo(tmp_path: Path) -> Path:
    repo = tmp_path / "history"
    repo.mkdir()
    git(repo, "init", "-q")
    for index in range(6):
        (repo / "auth.py").write_text(f"version = {index}\n")
        if index % 2 == 0:
            (repo / "session.py").write_text(f"version = {index}\n")
        git(repo, "add", ".")
        git(repo, "commit", "-q", "-m", f"auth update {index}")
    return repo


def test_load_git_history_reads_real_commits(tmp_path):
    module = load_script_module()
    repo = make_repo(tmp_path)
    rows, messages = module.load_git_history(repo, 1000)
    assert len(messages) == 6
    assert all(len(sha) == 40 for sha in messages)
    assert all("auth update" in message for message in messages.values())
    assert len(rows) == 6 + 3
    assert {row["file_path"] for row in rows} == {"auth.py", "session.py"}
    stamps = [row["timestamp"] for row in rows]
    assert stamps == sorted(stamps)


def test_end_to_end_baselines_report_without_ranking(tmp_path):
    module = load_script_module()
    repo = make_repo(tmp_path)
    rows, messages = module.load_git_history(repo, 1000)
    reports = compare_impact_models(rows, messages, k=2)
    assert [report["model"] for report in reports] == [
        "impact_keyword-only",
        "impact_cochange-only",
        "impact_combined",
    ]
