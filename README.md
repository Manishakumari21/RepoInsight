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
* hotspot scores with reasons
* aggregate difficulty score
* Served from in-memory cache when fresh (`no-store` on cache hit path)

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

Phase 4 is implemented: commits are sorted chronologically, files that repeatedly change together are detected (co-change), source → target changes within a 7-day window are tracked with occurrences and average delay (temporal analysis), and all three signals — dependency, temporal, co-change — are combined into a propagation graph exposed via the analysis response (`temporal.pairs`, `propagation.edges`). Generated paths (`.git`, `target`, `node_modules`, `dist`, `build`) are filtered out of the historical analysis.

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

* [ ] File-level change sequences
* [x] Temporal change representation (7-day window, occurrences + average delay, top 50 pairs)
* [x] Co-change relationships (canonical pair counting, top 50 pairs)
* [x] Change propagation detection (temporal + co-change + dependency graph, top 100 edges)
* [ ] Follow-up/rework detection
* [ ] Historical examples

Note: commits are sorted chronologically and the propagation graph is exposed via `temporal.pairs` and `propagation.edges` in the analysis response (mirrored in frontend `types.ts`; no dashboard UI for it yet). Sequences, follow-up/rework detection, and historical examples are not implemented yet.

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

* [x] Repository overview
* [ ] Change-risk view
* [ ] Predicted impact graph
* [x] Historical evidence (partial: history summary + co-change pairs)
* [x] File exploration (hotspot/explorer tables)
* [ ] Model explanation

Note: no ML model exists yet, so no prediction, confidence, or model
explanation is shown. Hotspot/difficulty scores are heuristic
percentile signals, not ML predictions.

## Phase 9 — Finalization

* [ ] Integration testing
* [ ] Performance benchmarking
* [ ] Large-repository testing
* [ ] Documentation
* [ ] Research evaluation
* [ ] Final demo

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

> **Temporal Change Propagation → Dataset Creation**

The GitHub collection layer, analysis foundations, and temporal change propagation engine are in place. The next major step is turning the temporal/propagation representation into ML-ready training data (**Phase 5: dataset generation**).

## Implementation status

### Implemented

* Repository metadata, file tree, and commit collection
* Source-blob collection with size/binary/UTF-8 gating, transient-error
  retries, and per-file fault tolerance
* Tree-sitter parsing for Rust, Python, JavaScript, TypeScript, TSX
* Per-file complexity and dependency-edge extraction
* Chronological commit ordering, co-change pair counting, temporal
  analysis (7-day window), and propagation graph
* Heuristic hotspot and difficulty scoring
* Dashboard overview, hotspot/explorer tables, dependency/history summaries
* In-memory analysis cache and GitHub rate-limit handling (plus retries
  for transient transport failures)

### Partially implemented

* Per-file LOC/functions/symbols are computed internally but only
  aggregates are exposed in the API response
* Dependency resolution is heuristic; unresolved references are
  returned with a `$` prefix (and can appear as propagation targets)
* History exposes totals, co-change pairs, temporal pairs, and the
  propagation graph (no per-commit timeline, sequences, clusters,
  or rework events yet)
* Co-change pairs are canonicalized alphabetically while temporal and
  dependency edges are directional, so the same file pair can appear
  as separate propagation edges

### Planned / not implemented

* Change sequences, follow-up/rework detection, historical examples
* Architecture graph, temporal slider, evolution replay, propagation UI
* Impact Simulator, what-if analysis, evidence panel, explainability
* ML dataset, model training, chronological evaluation
* AI assistant, repository-context export, reports
* Persistent database (`tests/backend` and `tests/ml` are empty;
  `ml/src` contains only `__init__.py`)

### Known limitations

* The Dependencies panel scales raw coupling counts as percentages;
  treat those bars as relative ordering, not true percentages.
* The analysis cache key is `owner/repo` and does not vary by branch.
* There is no per-file or per-commit detail endpoint yet.

---

# Long-Term Goal

RepoInsight aims to become a developer tool that answers:

> **"If I change this part of my repository, what else am I likely to have to change?"**

rather than simply reporting:

> "This file is complex."

That shift—from static repository metrics to **repository-specific change behavior prediction**—is the central idea behind the project.
