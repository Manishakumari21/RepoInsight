# RepoInsight

### Repository Intelligence for Predicting Change Impact and Rework Risk

RepoInsight analyzes how a software repository has evolved over time to help developers understand **what is likely to happen when they change a part of the codebase**.

Instead of relying only on static complexity or simple file-change frequency, RepoInsight combines:

* Repository structure
* Source-code characteristics
* Dependency relationships
* Historical commit behavior
* Temporal change sequences
* File-level change propagation
* Rework and follow-up patterns

The long-term goal is to predict whether a future change is likely to require **downstream modifications or rework**.

---

## Problem

Developers often know **which file they want to change**, but not:

> "What else is likely to be affected if I change this?"

Existing repository-analysis tools commonly focus on metrics such as:

* Code complexity
* Code churn
* Hotspots
* Dependencies
* Developer activity
* Static architecture

These metrics are useful, but they do not fully capture how a **specific repository has historically reacted to changes**.

For example:

```text
Developer changes:
src/payment/service.py

Historical behavior:
service.py changed
        ↓
orders/service.py changed
        ↓
billing/invoice.py changed
        ↓
follow-up fix
```

RepoInsight aims to learn these repository-specific patterns.

---

## Core Idea

RepoInsight treats a repository as a **temporal system**, not just a collection of source files.

```text
Repository
     │
     ├── Source Structure
     │       ├── Files
     │       ├── Functions
     │       ├── Complexity
     │       └── Dependencies
     │
     └── Git History
             ├── Commits
             ├── Changed Files
             ├── Change Sequences
             ├── Co-changes
             ├── Follow-up Changes
             └── Rework Patterns
                       │
                       ▼
              Temporal Representation
                       │
                       ▼
                ML Risk Model
                       │
                       ▼
             Change Impact Prediction
```

The key research question is:

> **Can historical change sequences combined with current repository structure predict whether a software change will cause downstream modifications or rework?**

---

## Example

Suppose a developer wants to modify:

```text
src/payment/service.py
```

RepoInsight could eventually produce:

```text
CHANGE RISK: HIGH

Target:
src/payment/service.py

Historical evidence:
- 87 previous modifications
- Frequently changed with orders/service.py
- Frequently followed by billing changes
- Multiple historical follow-up fixes

Structural context:
- 14 dependencies
- 6 dependent modules
- High structural complexity

Predicted impact:

payment
   ↓
orders
   ↓
billing

Prediction confidence: 0.82
```

The system distinguishes between:

### Observed Evidence

Facts directly obtained from repository history and source analysis.

### Model Prediction

What the trained model predicts about a future change.

### Historical Examples

Previous repository changes that support the prediction.

This separation is important because RepoInsight should **not present a model prediction as historical fact**.

---

# Architecture

```text
                    ┌──────────────────┐
                    │   GitHub Repo    │
                    └────────┬─────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
                ▼                         ▼
        Repository Structure          Git History
                │                         │
                ▼                         ▼
        Source Analysis             Temporal Analysis
                │                         │
        ┌───────┼────────┐         ┌──────┼──────────┐
        │       │        │         │      │          │
      Parser Complexity Dependencies Changes Sequences Rework
        │       │        │         │      │          │
        └───────┴────────┘         └──────┴──────────┘
                │                         │
                └──────────┬──────────────┘
                           ▼
                 Feature Representation
                           │
                           ▼
                    ML Risk Prediction
                           │
                           ▼
                   Explainable Results
                           │
                           ▼
                      Dashboard
```

---

# Current Technology Stack

## Backend

* **Rust**
* **Axum**
* **Tokio**
* **Reqwest**
* **Serde**
* **Tree-sitter**

The backend is responsible for repository collection, source analysis, GitHub integration, and analysis orchestration.

## Machine Learning

* **Python**
* **Pandas**
* **NumPy**
* **Scikit-learn**

Potential CPU-friendly models:

* Logistic Regression
* Random Forest
* XGBoost

Model selection will be based on experimental evaluation rather than assuming a particular model is best.

## Frontend

* **React**
* **TypeScript**
* **Vite**

The frontend will eventually provide an interactive repository analysis and change-risk dashboard.

---

# Repository Structure

```text
RepoInsight/
│
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── cache.rs (in-memory analysis cache, TTL 300s, cap 64)
│       │
│       ├── github/
│       │   ├── mod.rs
│       │   ├── client.rs
│       │   ├── repository.rs
│       │   ├── files.rs
│       │   └── commits.rs
│       │
│       └── analysis/
│           ├── mod.rs
│           ├── analyzer.rs
│           ├── source.rs
│           ├── complexity.rs
│           ├── dependencies.rs
│           ├── history.rs
│           ├── scoring.rs
│           ├── models.rs
│           │
│           └── parsers/
│               ├── mod.rs
│               └── tree_sitter.rs
│
├── ml/
│   ├── requirements.txt
│   └── src/
│       └── __init__.py
│
├── frontend/
│   └── src/
│       ├── api.ts (single analysis client)
│       ├── types.ts (mirrors backend RepositoryAnalysis)
│       ├── config.ts, nav.ts
│       ├── lib/ (derive, githubUrl, palette)
│       └── components/ (Dashboard, Overview, Risk, Complexity,
│           Hotspots, Explorer, Dependencies, History,
│           Cochange, Settings, Sidebar, Topbar,
│           Landing, Panel, Donut, Status)
│
├── tests/
│   ├── backend/
│   └── ml/
│
├── docs/
│   └── architecture.md
│
├── phases.md
├── README.md
└── .gitignore
```

The architecture is intentionally kept modular without creating separate files for every programming language. Tree-sitter provides the language-specific parsing layer while RepoInsight works with a normalized structural representation.

---

# Current Backend Capabilities

The GitHub integration currently supports:

### Repository information

```text
GET /api/repositories/{owner}/{repo}
```

Retrieves repository metadata including:

* Repository name
* Default branch
* Description
* Primary language
* Stars
* Forks
* Open issues
* Repository size

### Repository file tree

```text
GET /api/repositories/{owner}/{repo}/files
```

Retrieves the repository's Git tree and identifies source-file candidates.

### Commit history

```text
GET /api/repositories/{owner}/{repo}/commits
```

The collector:

* Paginates through commit history
* Supports up to 100 commits per page
* Limits collection to 1,000 commits by default
* Retrieves detailed commit information
* Collects changed files
* Collects additions/deletions
* Tracks commit authors
* Returns commits in chronological order
* Uses bounded concurrent requests (8)

### Repository analysis

```text
GET /api/repositories/{owner}/{repo}/analysis
```

Returns the combined `RepositoryAnalysis` payload:

* repository, source, complexity, history summaries
* dependency analysis (top coupled files)
* co-change pairs (top 50 by count)
* temporal pairs (top 50 by occurrences, 7-day window with average delay)
* propagation graph (top 100 edges combining temporal, co-change, and dependency signals)
* change timeline (chronological entries with SHA, order, author, files, additions/deletions; latest 300)
* change sequences (top 50 pairs/triples from consecutive commits, configurable 7-day window)
* follow-ups (top 100: same-file observed; co-change/dependency/fix-message derived)
* rework signals (top 100 candidate events with rule + evidence: repeated-touch, fix-message, revert, related-fix; 14-day window)
* historical examples (top 20 temporal/sequence cases with signals and example SHAs)
* hotspot scores with reasons
* aggregate difficulty score
* Served from in-memory cache when fresh (`no-store` on cache hit path)

### Local Git repository analysis

RepoInsight also analyzes local Git repositories — no GitHub URL, token, or
network access required.

```text
POST /api/local/analysis
Content-Type: application/json

{"path": "/home/user/projects/my-repo"}
```

```text
POST /api/local/dataset
Content-Type: application/json

{"path": "/home/user/projects/my-repo"}
```

```text
POST /api/local/predictions
Content-Type: application/json

{"path": "/home/user/projects/my-repo"}
```

The GitHub equivalent is `GET
/api/repositories/{owner}/{repo}/predictions`. Predictions run the exported
Phase 6 logistic-regression baseline over leakage-safe dataset rows and
return per-file probability, label, confidence, evidence, historical
examples and recommendations for the dashboard.

The local loader (`backend/src/local.rs`) validates the path, confirms it is
a Git repository via `git2`, then extracts the same common
representation the GitHub collector produces: repository metadata, file tree
(`git2` tree walk with `HEAD` fallback), commit history with changed files and authors
(`git log --numstat --name-status`), and working-tree source contents. Both
sources feed the identical Phase 3/4/5 pipeline (`analyze_loaded_data`,
`build_dataset_from_loaded`), so the dashboard, dataset, and ML features are
the same regardless of source.

Clear errors are returned for a missing path (`404`), a non-directory
(`400`), a non-Git directory (`422`), or an inaccessible repository (`403`).
Only the explicitly supplied repository directory is read; nothing outside it
is exposed.

### Source collection

The source-analysis layer:

* Filters non-source files
* Limits individual source files to 1 MB
* Retrieves GitHub blobs
* Decodes Base64 content
* Ignores binary/non-UTF-8 content
* Uses bounded concurrent requests (8)
* Retries transient blob/transport failures and skips files that still
  fail instead of failing the whole analysis

### Static analysis foundation

The current analysis layer includes:

* Source statistics
* Complexity analysis
* Dependency analysis
* Historical activity analysis
* Repository-level analysis models
* Tree-sitter parsing
* Dependency reference → file-path resolution
* Chronological commit ordering
* Co-change detection
* Temporal change analysis (source → target pairs, 7-day window,
  occurrences + average delay)
* Propagation graph (temporal + co-change + dependency edges with
  per-type flags and combined strength)
* Change timeline (chronological entries: SHA, order, timestamp, author,
  files, additions/deletions)
* Change sequences (ordered pairs/triples from consecutive commits,
  configurable window, occurrence + delay counting)
* Follow-up detection (deterministic rules; observed vs derived signals)
* Rework detection (explicit rules — repeated-touch, fix-message, revert,
  related-fix — each with evidence; candidates, never definite)
* Historical examples (source/target, sequence, occurrences, delay,
  signals, example SHAs)

---

# Tree-sitter Parsing

RepoInsight currently has a centralized Tree-sitter parser layer supporting:

```text
Rust
Python
JavaScript
TypeScript
TSX
```

Language detection is based on source-file paths.

The parser produces a common representation that can later be used for structural feature extraction.

Tree-sitter is **not the core identity of RepoInsight**. It is a supporting component used to obtain structural information from source code.

---

# Temporal Change Analysis

This is the core research direction of RepoInsight.

Phase 4 (including 4B) is implemented: commits are sorted chronologically and exposed as a file-level change timeline (`timeline.entries` with SHA, order, timestamp, author, files, additions/deletions); ordered change sequences (pairs and triples from consecutive commits within a configurable 7-day window) are counted with occurrences and average delay (`sequences`); files that repeatedly change together are detected (co-change); source → target changes within a 7-day window are tracked with occurrences and average delay (temporal analysis); all three signals — dependency, temporal, co-change — are combined into a propagation graph (`propagation.edges`); deterministic follow-up rules link later commits to earlier ones (`followups`, observed vs derived); explicit rework heuristics flag candidate events with the triggering rule and evidence (`rework`, 14-day window, never presented as definite); and representative cases are exposed as historical examples (`examples` with signals and example SHAs). Generated paths (`.git`, `target`, `node_modules`, `dist`, `build`) are filtered out of the historical analysis. No LLM is used; no risk/confidence predictions are produced.

Instead of treating history as:

```text
file → number of commits
```

RepoInsight aims to represent history as:

```text
Change A
   ↓
Change B
   ↓
Change C
   ↓
Follow-up / Rework
```

This allows the system to study patterns such as:

* Files repeatedly changing together
* Changes propagating across modules
* Changes followed by additional modifications
* Repeated rework
* Change sequences
* Historical downstream effects
* Repository-specific evolution patterns

The goal is to move from:

```text
"What files are risky?"
```

towards:

```text
"If I change this file, what is likely to happen next?"
```

---

# Machine Learning Approach

The ML pipeline will use repository history to construct training examples.

A simplified representation is:

```text
Historical Repository Data
          │
          ▼
Feature Extraction
          │
          ├── Structural Features
          ├── Dependency Features
          ├── Historical Features
          ├── Temporal Features
          └── Change Features
                    │
                    ▼
              Training Dataset
                    │
                    ▼
             Time-based Split
                    │
             ┌──────┴──────┐
             ▼             ▼
         Validation       Test
             │
             ▼
        Model Training
             │
             ▼
       Risk Prediction
```

Candidate models include:

* Logistic Regression — baseline
* Random Forest
* XGBoost

The final model will be selected using measured performance.

---

# Evaluation

Because RepoInsight predicts future repository behavior, evaluation should respect time.

We will **not rely on a random train/test split** for the main experiment.

Instead:

```text
Older History ───────────────► Newer History

     Training      Validation       Test
        │               │             │
        └───────────────┴─────────────┘
                  Time
```

Candidate evaluation metrics:

* Precision
* Recall
* F1-score
* ROC-AUC
* PR-AUC
* Calibration

This is intended to measure whether the model can generalize from a repository's past behavior to its later behavior.

---

# Explainability

A prediction should not simply say:

```text
Risk = 0.82
```

RepoInsight should explain **why**.

The result will distinguish:

```text
Observed:
src/payment/service.py frequently changes with
src/orders/service.py

Observed:
Previous changes were often followed by billing changes

Structural:
The target file has multiple dependent modules

Prediction:
High probability of downstream rework

Confidence:
0.82
```

Model explanation techniques such as feature importance or SHAP may be added after the prediction pipeline is stable.

---

# Development Roadmap

## Phase 1 — Project Foundation

* [x] Repository setup
* [x] Rust/Axum backend
* [x] Python ML environment
* [x] React/Vite frontend
* [x] Documentation structure

## Phase 2 — GitHub Repository Collector

* [x] Repository metadata
* [x] Repository file tree
* [x] Commit collection
* [x] Commit pagination
* [x] Detailed commit retrieval
* [x] Source-file collection
* [x] Concurrent API requests

## Phase 3 — Repository Intelligence Foundation

* [x] Source analysis foundation
* [x] Complexity analysis foundation
* [x] Dependency analysis foundation
* [x] History analysis foundation
* [x] Tree-sitter integration foundation

## Phase 4 — Change Propagation Engine

* [x] File-level change sequences (ordered pairs/triples, configurable 7-day window, top 50)
* [x] Temporal change representation (7-day window, occurrences + average delay, top 50 pairs)
* [x] Co-change relationships (canonical pair counting, top 50 pairs)
* [x] Change propagation detection (temporal + co-change + dependency graph, top 100 edges)
* [x] Change timeline (chronological entries: SHA, order, timestamp, author, files, additions/deletions; latest 300)
* [x] Follow-up detection (same-file observed; co-change/dependency/fix-message derived; top 100)
* [x] Rework detection (repeated-touch, fix-message, revert, related-fix rules with evidence; top 100 candidates)
* [x] Historical examples (top 20 temporal/sequence cases with signals and example SHAs)
* [x] False-positive measurement (6 reviewed pairs from this repo's own history: TP=0, FP=2, TN=4, FN=0; precision 0.0, recall n/a, FPR 0.333; see `tests/backend/evaluation/README.md`)

Note: commits are sorted chronologically and the propagation history is exposed via `timeline.entries`, `sequences.sequences`, `followups.followups`, `rework.events`, and `examples.examples` in the analysis response (mirrored in frontend `types.ts` and shown in the dashboard's Sequences, Propagation History, and Historical Examples sections). Follow-ups are labeled observed/derived and rework is always presented as *candidate* with its rule — never as a prediction. False-positive measurement is still open.

## Phase 5 — Dataset Creation

* [ ] Define prediction target
* [ ] Generate training examples
* [ ] Extract temporal features
* [ ] Extract structural features
* [ ] Construct chronological datasets
* [ ] Prevent temporal leakage

## Phase 6 — ML Prediction

* [ ] Baseline model
* [ ] Candidate model comparison
* [ ] Time-based evaluation
* [ ] Calibration
* [ ] Feature importance
* [ ] Model persistence

## Phase 7 — Explanation

* [ ] Evidence extraction
* [ ] Prediction explanation
* [ ] Historical examples
* [ ] Confidence representation
* [ ] Human-readable recommendations

## Phase 8 — Dashboard

* [x] Repository overview (cards, commit activity, most changed/connected files, risk distribution)
* [x] Change-risk view (Prediction Explorer: search, label/probability/model filters, sorting)
* [x] Predicted impact graph (neighborhood graph with dependency/temporal/co-change edges, focus, depth, pan/zoom)
* [x] Historical evidence (history summary, co-change pairs, change sequences, propagation history, historical examples)
* [x] File exploration (directory tree with per-file prediction badges)
* [x] Model explanation (file drawer: probability, label, confidence, top evidence, historical examples, recommendations)

Predictions are served by `GET /api/repositories/{owner}/{repo}/predictions`
and `POST /api/local/predictions`: the exported Phase 6 logistic-regression
baseline runs over leakage-safe dataset rows (latest row per file).
Hotspot/difficulty scores remain heuristic percentile signals and are labeled
as such; ML probabilities are labeled with the model name and uncalibrated
status.

### Change Ripple Forecasting

RepoInsight Ripple answers "when this file changes, which other files
historically and structurally tend to change afterward?" — an explainable
integration of structural dependencies, historical co-change, temporal
follow-up ordering, and recent coupling. A dependency edge means "A depends
on B"; a ripple edge means "when A changes, B tends to change afterward".

```text
GET  /api/repositories/{owner}/{repo}/ripple?source=login.ts&max_depth=5&min_confidence=0.5
POST /api/local/ripple
{"path": "/home/user/projects/my-repo", "source": "login.ts"}
```

Each response carries ranked candidates with reasons, one greedy
cycle-free ripple path (max depth 5, never forced when evidence is weak),
per-edge historical examples, and neutral "potential missing impact"
warnings. Scores are a documented weighted baseline (`ripple_baseline`,
uncalibrated), computed from precomputed dependency/co-change/temporal
structures so per-file queries stay fast. Chronological evaluation
(precision@K, recall@K, MRR, MAP) lives in `backend/src/analysis/ripple.rs`
(`evaluate_ripple`) and `ml/src/repoinsight_ml/ripple.py`
(`compare_ripple_models`: co-change baseline vs full model).

### Change Impact Simulator

The Impact Simulator answers a different question from predictions or
ripple forecasting: "this change is only planned, what should I look at?"
A developer describes the intended change in natural language
("Replace JWT authentication with OAuth2") and RepoInsight estimates
which repository areas would need attention before anything is modified.
Analysis only — no source code is ever changed.

```text
GET  /api/repositories/{owner}/{repo}/impact-simulation?change_description=Replace+JWT+authentication+with+OAuth2
POST /api/local/impact-simulation
{"path": "/home/user/projects/my-repo", "change_description": "Replace JWT authentication with OAuth2"}
```

The pipeline is deterministic and local (no LLM, no network): intent
extraction (operation verbs + domain/technology vocabularies) → a
repository capability map derived from directory names, filenames, and
source content → impact discovery over semantic, dependency, historical
co-change, and test/config/docs signals → normalized weighted scoring
(`impact_baseline`, uncalibrated) with HIGH/MEDIUM/LOW candidate levels →
an impact map (planned change → capabilities → files), a review checklist,
and per-file evidence. Each scenario request is independent, so
alternative plans ("replace" vs "add alongside") can be compared
side by side. Chronological evaluation (commit message as description,
strictly-before-T prefixes; precision@K, recall@K, F1@K, directory
accuracy, test/config discovery) lives in
`backend/src/analysis/impact.rs` (`evaluate_impact` with keyword-only,
dependency-only, co-change-only, and combined baselines) and
`ml/src/repoinsight_ml/impact.py` (`compare_impact_models`).

## Phase 9 — Finalization

* [x] Integration testing
* [x] Performance benchmarking
* [ ] Large-repository testing
* [x] Documentation
* [x] Research evaluation
* [ ] Final demo

Research evaluation artifacts (all measured, never fabricated):
`ml/multi_repo_benchmark.json` (4 repos × 3 models),
`ml/model_comparison.json` (no ranking — class distributions differ),
`ml/ablation_results.json` (7 feature-group configurations),
`ml/error_analysis.json` (test-set FP/FN patterns).
Reproduce with `ml/scripts/benchmark.py`, `run_ablation.py`,
`run_model_comparison.py`, and `ml/error_analysis.py`; see `phases.md`
Phase 9 notes. The large Axum dataset (`ml/data/axum.jsonl`) is read-only
input — never regenerated by these tools.

---

# Design Principles

RepoInsight follows several important principles.

### 1. Repository-adaptive

The system should learn from the repository being analyzed rather than relying on hardcoded rules for a particular project.

### 2. Evidence before prediction

Historical evidence and model predictions must remain distinguishable.

### 3. Temporal correctness

Future information must never leak into historical training examples.

### 4. CPU-friendly

The system should be usable on ordinary developer machines without requiring a dedicated GPU.

### 5. Language extensibility

Language support should be centralized through parsing and normalized representations rather than duplicating the entire analysis stack for every language.

### 6. No fake intelligence

A simple metric should not be presented as ML.

A prediction should only be shown when it is backed by a trained and evaluated model.

### 7. Incremental architecture

Components should be added when they have a real responsibility rather than creating files merely to make the project appear larger.

---

# Current Status

**Status: Active Development**

Current focus:

> **Phase 10: Final Research Evaluation**

Shipped on top of the Phase 3–9 foundation: leakage-safe ML datasets
and baselines (logistic regression + RF/HGB comparison), per-file
predictions with evidence, Change Ripple Forecasting, Change Impact
Simulator, and a 4-section investigation dashboard (Overview / Explore /
Predict / History). The Phase 10 battery (benchmark, ablation, model
comparison, error analysis, ripple and impact evaluations) runs from
`ml/scripts/`; consolidated honest numbers live in
`docs/research-evaluation.md`. Generated JSON artifacts are git-ignored
and reproducible.

## Implementation status

### Implemented

* Repository metadata, file tree, and commit collection
* Source-blob collection with size/binary/UTF-8 gating, transient-error
  retries, and per-file fault tolerance
* Tree-sitter parsing for Rust, Python, JavaScript, TypeScript, TSX
* Per-file complexity and dependency-edge extraction
* Chronological commit ordering, co-change pair counting, temporal
  analysis (7-day window), and propagation graph
* Change timeline, ordered change sequences (pairs/triples), deterministic
  follow-up detection (observed/derived), rule-based candidate rework
  detection with evidence, and historical examples
* Heuristic hotspot and difficulty scoring
* Leakage-safe ML dataset, exported logistic-regression baseline with
  per-file predictions (`GET /api/repositories/{owner}/{repo}/predictions`,
  `POST /api/local/predictions`), Random Forest / HGB comparison,
  chronological evaluation, ablation, and error analysis
  (`ml/multi_repo_benchmark.json`, `ml/model_comparison.json`,
  `ml/ablation_results.json`, `ml/error_analysis.json`)
* Change Ripple Forecasting (`/ripple`) and Change Impact Simulator
  (`/impact-simulation`), local + GitHub variants
* Prediction explanation: evidence, confidence, historical examples,
  recommendations, evidence panel in the dashboard
* Workbench dashboard: Overview briefing, Structure (tree + neighborhood
  graph + inspector), Timeline, Coupling, Predictions, Evidence —
  all labeled observed-history vs model-output
* In-memory analysis cache and GitHub rate-limit handling (plus retries
  for transient transport failures)

### Partially implemented

* Dependency resolution is heuristic; unresolved references are
  returned with a `$` prefix (and can appear as propagation targets)
* History exposes totals, co-change pairs, temporal pairs, the
  propagation graph, the change timeline, sequences, follow-ups, rework
  events, and historical examples (no per-commit detail endpoint, clusters,
  or architecture graph yet)
* Co-change pairs are canonicalized alphabetically while temporal and
  dependency edges are directional, so the same file pair can appear
  as separate propagation edges

### Planned / not implemented

* Architecture graph, temporal slider, evolution replay
* AI assistant, repository-context export, reports
* Persistent database
* Per-commit detail endpoint, branch-varying cache key

### Known limitations

* The Dependencies panel shows raw coupling counts (links) with bars scaled
  relative to the repository peak — not percentages.
* The analysis cache key is `owner/repo` and does not vary by branch.
* There is no per-file or per-commit detail endpoint yet.

---

# Long-Term Goal

RepoInsight aims to become a developer tool that answers:

> **"If I change this part of my repository, what else am I likely to have to change?"**

rather than simply reporting:

> "This file is complex."

That shift—from static repository metrics to **repository-specific change behavior prediction**—is the central idea behind the project.
