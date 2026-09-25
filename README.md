# RepoInsight

### Repository Intelligence for Change Impact, Coupling, and Rework Risk

## Overview

RepoInsight is a repository analysis and research platform that combines **source-code structure, Git history, temporal behavior, and machine learning** to study how changes propagate through a software repository.

Developers usually know which file they want to change, but not what else is likely to be affected. Static metrics such as complexity, churn, or hotspot scores describe files in isolation; they do not capture how a *specific repository has historically reacted to changes*. RepoInsight addresses this by learning repository-specific patterns of the form:

```text
Change A → Change B → Change C → follow-up / rework
```

and using them to estimate what a future change may entail. It is designed as an engineering analysis workbench rather than a generic code dashboard.

## Key Capabilities

* **Repository analysis** — local Git and GitHub repository ingestion, source and dependency inspection, commit and file-history analysis with deterministic results.
* **Historical coupling / co-change analysis** — files that repeatedly change together, with occurrence counts (a model input, never itself presented as a risk score).
* **Temporal analysis** — ordered change sequences and source → target pairs within a 7-day window, with occurrences and average delay; combined with co-change and dependency signals into a propagation graph.
* **Follow-up / rework analysis** — deterministic rules (repeated-touch, fix-message, revert, related-fix) flag *candidate* rework events with triggering rule and evidence; follow-ups are labeled observed vs derived.
* **ML prediction** — leakage-safe per-file change-risk baseline (logistic regression) with evidence, historical examples, confidence, and recommendations.
* **Ripple analysis** — for a changed file, ranked lists of files that historically and structurally tend to change afterward, plus a greedy cycle-free propagation path. Shipped as a research/engineering feature; evaluation has documented limitations (see below).
* **Impact Simulator** — for a *planned* change described in natural language, an estimate of which repository areas would need attention before anything is modified. Deterministic and local (no LLM, no network). Shipped; validation data is limited (see below).
* **Evidence and explanation** — every result separates observed evidence from heuristic signals and model output, with per-feature contributions and historical examples.
* **Analysis workbench frontend** — investigation-oriented views: Overview, Structure, History (timeline), Coupling, Predictions (predicted changes, ripple, impact simulator), and Evidence.

## How It Works

```text
Git Repository (local or GitHub)
      │
      ├── Source structure ── Tree-sitter parsing, complexity,
      │                        dependency edges, structural features
      └── Git history ───────── chronological timeline, co-change pairs,
                                temporal pairs, sequences, follow-ups,
                                candidate rework events
                      │
                      ▼
      Leakage-safe feature rows (strictly-before-T prefixes)
                      │
                      ▼
      ML baseline + ripple / impact baselines + evidence layer
                      │
                      ▼
      React analysis workbench
```

RepoInsight keeps three kinds of output strictly separate:

* **Observed repository evidence** — facts from history and source analysis (change counts, co-change pairs, temporal pairs, dependency edges, timelines).
* **Heuristic signals** — deterministic rules and percentile scores (hotspot/difficulty scores, ripple and impact weighted baselines, rework candidates). These are labeled as heuristics, never as ML predictions.
* **ML predictions** — output of the trained logistic-regression baseline. All served probabilities are explicitly labeled with the model name and **uncalibrated** status; confidence levels are documented thresholds, not guarantees.

No prediction is presented as a guaranteed outcome, and heuristic scores are never described as model output.

## Research & Evaluation

The central research question is whether historical change sequences combined with repository structure can predict downstream modification or rework risk. The positioning is factual and conservative: RepoInsight provides measured baselines, not claims of novelty or superiority.

* **Leakage-safe chronological evaluation** — datasets are built from strictly-before-T commit prefixes (equal-timestamp groups are atomic; untimestamped commits excluded); chronological commit-granular train/validation/test splits; rows are never shuffled.
* **Baselines compared per repository** — logistic regression (shipped default), Random Forest, and Histogram Gradient Boosting, reported without cross-repository ranking since class distributions differ.
* **Ablation and error analysis** — feature-group removal experiments and test-set false-positive/false-negative analysis, in associational language only.
* **Ripple evaluation** — chronological precision@K/recall@K/MRR/MAP comparing a co-change-only baseline against the full model. Documented limitation: ground truth is defined as other files in the *same* commit, which favors co-change by construction; the temporal signal shows no measured gain under this setup.
* **Impact evaluation** — commit messages as change descriptions with strictly-before-T prefixes. Documented limitation: near-null on the available coarse self-history (1 evaluable target), so the simulator is shipped but not validated by that data.
* Consolidated honest numbers live in `docs/research-evaluation.md`; generated JSON artifacts are git-ignored and reproducible via `ml/scripts/`.

## Performance

The dataset pipeline was optimized through an 8-step series (Candidate #4) replacing repeated full-history rescans with incremental prefix accumulators (historical, temporal, propagation, file-timestamp, and follow-up/rework state), each checked against its original reference implementation with equivalence tests. Temporal leakage semantics were preserved exactly.

Large-repository benchmark (Axum, `/home/manisha/axum`): 504 files, 1000 commits, 501 source files, 209,477 dataset rows — approximately **87–100 seconds** cold analysis with approximately **227 MB** peak RSS on the development machine. Timing varies with hardware and system load and is not a universal guarantee. Repeated analyses of the same repository are served from the in-memory analysis cache (TTL 300 s), which substantially reduces repeat cost.

## Tech Stack

* **Backend** — Rust, Axum, Tokio, git2, Tree-sitter (Rust, Python, JavaScript, TypeScript, TSX), serde.
* **Frontend** — React, TypeScript, Vite, TanStack React Query.
* **Machine learning** — Python, scikit-learn, pandas, NumPy. No GPU required.

## Run Locally

Backend (serves the API on port 3000):

```bash
cd backend
cargo run
```

Frontend (development server):

```bash
cd frontend
npm install
npm run dev
```

ML research pipeline (training, comparison, ablation, evaluation):

```bash
# see ml/scripts/ for benchmark, ablation, model-comparison,
# ripple-evaluation, and impact-evaluation entry points
```

Analyze a local repository (no GitHub token or network required):

```bash
curl -X POST localhost:3000/api/local/analysis \
  -H 'Content-Type: application/json' \
  -d '{"path":"/home/user/projects/my-repo"}'
```

## Limitations

* Cold analysis of large repositories takes on the order of a minute or more; exact cost depends on history size and machine load.
* The analysis cache key does not vary by branch.
* There is no per-commit detail endpoint; no persistent database.
* Dependency resolution is heuristic; unresolved references carry a `$` prefix.
* ML, ripple, and impact outputs are uncalibrated baselines, not calibrated production predictions.
* Ripple and impact evaluations have the documented methodological limitations noted above.

## License

This project is licensed under the MIT License — see [LICENSE](./LICENSE).
