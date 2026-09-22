# RepoInsight — Development Phases

## Project Goal

RepoInsight is a repository intelligence system designed to answer:

> **If a developer changes this part of a repository, what else is likely to change or require rework?**

The system combines repository structure with historical Git behavior to predict **change propagation and rework risk**.

---

# Phase 1 — Project Foundation

**Status: ✅ Complete**

### Goal

Create the base project structure and development environments.

### Completed

* [x] Initialize Git repository
* [x] Create Rust backend
* [x] Configure Axum/Tokio
* [x] Create Python ML environment
* [x] Configure NumPy/Pandas/scikit-learn
* [x] Create React + TypeScript + Vite frontend
* [x] Add documentation structure
* [x] Add `.gitignore`
* [x] Verify backend compilation
* [x] Verify frontend build

### Result

RepoInsight has a working multi-component development foundation.

---

# Phase 2 — GitHub Repository Collector

**Status: ✅ Complete**

### Goal

Collect enough repository information to perform automated analysis on arbitrary GitHub repositories.

### Repository Metadata

* [x] Repository name
* [x] Description
* [x] Default branch
* [x] Primary language
* [x] Stars
* [x] Forks
* [x] Open issues
* [x] Repository size

### File Collection

* [x] Retrieve Git tree
* [x] Detect source-file candidates
* [x] Ignore common binary files
* [x] Limit source-file size
* [x] Retrieve source contents
* [x] Decode GitHub Base64 blobs
* [x] Support concurrent source requests

### Commit Collection

* [x] Retrieve commit summaries
* [x] Paginate commit history
* [x] Support up to 100 commits/page
* [x] Limit maximum history collection
* [x] Retrieve detailed commit information
* [x] Retrieve changed files
* [x] Retrieve additions/deletions
* [x] Track commit authors
* [x] Use bounded concurrent detail requests

### API Client

* [x] Centralized GitHub client
* [x] Relative API endpoint support
* [x] Full GitHub API URL support
* [x] HTTP error handling
* [x] JSON response parsing

### Result

Raw repository structure and Git history can now be collected programmatically.

---

# Phase 3 — Repository Intelligence Foundation

**Status: 🚧 In Progress**

### Goal

Transform raw repository data into structured information that can later be used for temporal modeling.

---

## 3.1 Source Analysis

* [x] Source-file representation
* [x] Source line counting
* [x] Source size calculation
* [x] Binary-file filtering
* [x] Source collection pipeline

### Remaining

* [ ] Normalize source metadata
* [ ] Add file-level analysis records
* [ ] Connect source analysis to repository analysis

---

## 3.2 Tree-sitter Parsing

* [x] Add Tree-sitter
* [x] Add Rust grammar
* [x] Add Python grammar
* [x] Add JavaScript grammar
* [x] Add TypeScript grammar
* [x] Add TSX grammar
* [x] Centralized language detection
* [x] Parser abstraction
* [x] Validate parser tests
* [x] Extract normalized syntax information

### Important Design Rule

Do **not** create separate analysis implementations such as:

```text
rust.rs
python.rs
javascript.rs
typescript.rs
```

unless a language-specific behavior is genuinely required.

Tree-sitter should feed a common representation.

---

## 3.3 Complexity Analysis

* [x] Complexity result structure
* [x] Cyclomatic complexity foundation
* [x] Function counting
* [x] Nesting-depth calculation
* [x] Unit tests

### Remaining

* [ ] Connect complexity analysis to parsed source
* [ ] Produce file-level complexity metrics
* [ ] Validate metrics across supported languages

---

## 3.4 Dependency Analysis

* [x] Dependency edge representation
* [x] Duplicate-edge handling
* [x] Self-dependency filtering
* [x] Incoming dependency calculation
* [x] Outgoing dependency calculation
* [x] Coupling calculation
* [x] Unit tests
* [x] Extract dependency edges from parsed source
* [x] Normalize module/import relationships
* [x] Connect dependency analysis to repository files

---

## 3.5 Historical Analysis

* [x] Commit-level history representation
* [x] Contributor counting
* [x] Changed-file counting
* [x] File-level change counts
* [x] Addition/deletion tracking
* [x] Churn calculation
* [x] Preserve chronological ordering
* [x] Detect repeated change relationships

### Remaining

* [ ] Build file change timelines
* [ ] Represent commit change sets

---

# Phase 4 — Temporal Change Propagation Engine

**Status: ✅ 4B Complete (sequences, follow-ups, rework, examples done)**

### Goal

This is the central analytical component of RepoInsight.

Move from:

```text
file → number of changes
```

to:

```text
change → subsequent changes → downstream effect → rework
```

Implemented in `backend/src/analysis/history.rs` (pairs, propagation graph)
and `backend/src/analysis/propagation_history.rs` (timeline, sequences,
follow-ups, rework, examples). Temporal windows are configurable via
`PropagationConfig` (sequence/follow-up default 7 days, rework default 14 days).

---

## 4.1 Change Timeline

For every relevant file:

* [x] Construct chronological change history
* [x] Record commit timestamps
* [x] Record commit ordering
* [x] Record files changed together
* [x] Record additions/deletions
* [x] Track authors/contributors

Example:

```text
T1 ── A.py
T2 ── A.py + B.py
T3 ── B.py + C.py
T4 ── A.py
```

---

## 4.2 Change Sequences

Represent historical sequences such as:

```text
A → B
A → B → C
A → B → fix
A → B → C → rework
```

### Tasks

* [x] Define change-window strategy (consecutive commits within a configurable window, default 7 days)
* [x] Generate ordered change sequences (length-2 pairs and length-3 triples, deterministic)
* [x] Identify downstream changes (temporal pairs + sequence chains)
* [x] Detect repeated propagation patterns (occurrence counting with average delay)
* [x] Store historical examples (top temporal pairs and sequences with signals and SHAs)

---

## 4.3 Co-change Analysis

Determine which files repeatedly change together.

```text
A ───── B
│       │
│       └── C
│
└──── D
```

But co-change frequency should **not itself become the final risk score**.

It is a feature for the temporal model.

---

## 4.4 Follow-up and Rework Detection

Investigate patterns such as:

```text
Initial change
      ↓
Additional modification
      ↓
Correction / fix
      ↓
Rework
```

### Tasks

* [x] Define operational rework criteria (repeated-touch, fix-message, revert, related-fix rules in `detect_rework`)
* [x] Identify follow-up changes (same-file observed; co-change/dependency/fix-message derived)
* [x] Identify likely corrective changes (word-based fix-message heuristic, never substring matching)
* [x] Handle revert patterns (`Revert "..."` detection, takes precedence over repeat rules)
* [x] Validate detection rules (backend unit tests: ordering, windows, follow-ups, rework, reverts, duplicates, empty history)
* [x] Measure false positives (Phase 4.4: `tests/backend/evaluation/` — 6 reviewed adjacent pairs from this repo's own history scored against unmodified `detect_rework`; TP=0, FP=2, TN=4, FN=0; precision 0.0, recall n/a, FPR 0.333; served at `GET /api/evaluation/rework` and shown in Settings)

Follow-ups are reported as *candidate* rework with the triggering rule and
evidence string. Historical evidence (`observed`/`derived`) is never mixed
with future prediction — no ML model exists yet, so no risk/confidence values
are produced.

The definition of the prediction target must be experimentally justified rather than arbitrarily chosen.

---

# Phase 5 — Feature Engineering & Dataset Creation

**Status: ✅ Complete**

### Goal

Convert repository history and structure into ML-ready examples.

Each training example should represent a realistic historical change.

---

## 5.1 Structural Features

Potential features:

* File size
* Lines of code
* Function count
* Cyclomatic complexity
* Maximum nesting depth
* Incoming dependencies
* Outgoing dependencies
* Coupling
* Number of dependents

---

## 5.2 Historical Features

Potential features:

* Previous change count
* Recent change frequency
* Historical churn
* Number of contributors
* Time since last change
* Historical co-change frequency

---

## 5.3 Temporal Features

Potential features:

* Recent change sequence
* Previous files changed
* Change-window statistics
* Change ordering
* Propagation frequency
* Follow-up frequency
* Historical rework frequency

---

## 5.4 Dataset Construction

* [x] Define prediction unit (`backend/src/dataset.rs`: ONE ROW = one file in one historical target commit)
* [x] Define target label (`1` = file changed by the target commit; `0` = eligible file provably existing before it but unchanged)
* [x] Generate positive examples (target commit's own relevant files with structural data)
* [x] Generate negative examples (structural snapshot ∩ prefix-seen files, minus positives; files never seen before the target are excluded)
* [x] Handle class imbalance (reported via `DatasetStats` positive/negative ratios; nothing discarded silently)
* [x] Remove invalid examples (`validate_dataset` flags empty paths, non-binary labels, invalid timestamps, unknown targets)
* [x] Prevent duplicate examples (duplicate commit/file rows flagged; splits are commit-granular)
* [x] Prevent temporal leakage (strictly-before-`T` prefixes; untimestamped commits excluded)
* [x] Validate dataset quality (`ValidationReport` with per-row reasons + stats)

Implemented in `backend/src/dataset.rs` (`build_dataset`, `validate_dataset`,
`dataset_stats`, `split_chronologically`, `export_jsonl` over JSONL via the
existing `serde_json` dependency). Construction is independent of the HTTP
layer.

---

## 5.5 Chronological Splitting

The primary experiment must respect time.

```text
Older commits                         Newer commits
────────────────────────────────────────────────────►

      Training       Validation        Test
         │               │               │
         ▼               ▼               ▼
```

Do **not** use a random split as the primary evaluation strategy.

Implemented as `split_chronologically` (default 70/15/15 by target count,
configurable via `DatasetConfig`): targets ordered by (timestamp, SHA) go
oldest-first to train, then validation, then test. Whole commits stay in one
split, rows are never shuffled, histories with fewer than three targets stay
entirely in train. No ML training happens here — that is Phase 6.

## 5.6 Phase 5 Notes

* Feature groups: structural (`analysis/features.rs`, current-tree snapshot),
  historical (`analysis/historical_features.rs`, commit-aware strictly-before-`T`
  prefixes; the older repository-level snapshot API is unchanged),
  temporal (`analysis/temporal_features.rs`, prefix detectors reused, no
  placeholder maps).
* Temporal leakage prevention: per-target prefixes contain only commits with
  timestamps strictly before `T`; target-own changes, future churn/co-change/
  propagation/rework/contributors are unreachable by construction.
* Known limitation: structural features and static dependency edges are
  current-snapshot proxies, not historical reconstructions; rows require
  structural coverage, so changes to unparsed files are excluded and reported
  via validation rather than zero-filled.

---

# Phase 6 — ML Prediction

**Status: ⏳ Planned**

### Goal

Train a model that predicts whether a change is likely to result in downstream modification or rework.

---

## 6.1 Baseline

Implement:

* [ ] Logistic Regression
* [ ] Baseline metrics
* [ ] Feature preprocessing

The baseline establishes whether the engineered features contain useful predictive information.

---

## 6.2 Candidate Models

Evaluate CPU-friendly models:

* [ ] Random Forest
* [ ] Gradient Boosting / XGBoost
* [ ] Logistic Regression

The final model should be selected based on measured performance rather than assumed superiority.

---

## 6.3 Evaluation

Measure:

* [ ] Precision
* [ ] Recall
* [ ] F1
* [ ] ROC-AUC
* [ ] PR-AUC
* [ ] Calibration

Particular attention should be given to precision/recall because false risk warnings can reduce developer trust.

---

## 6.4 Model Validation

* [ ] Chronological validation
* [ ] Cross-repository evaluation where possible
* [ ] Feature ablation
* [ ] Baseline comparison
* [ ] Error analysis
* [ ] Model persistence

---

# Phase 7 — Prediction Explanation

**Status: ✅ Complete (ML-side: evidence, explanation, historical examples, confidence, recommendations; 30 new tests green, Phase 6 untouched)**

Implemented in `ml/src/repoinsight_ml/` (`evidence.py`, `explanation.py`,
`historical_examples.py`, `confidence.py`, `recommendations.py`).
End-to-end entry point `explain_prediction()` returns
`{prediction, evidence, historical_examples, confidence, recommendations}`.
Phase 7 is implemented as an ML-side explanation layer. Rust/backend
prediction integration is handled in Phase 8.

### Goal

Make predictions understandable and evidence-backed.

A result should contain:

```text
Prediction
    +
Confidence
    +
Important features
    +
Historical evidence
    +
Potential downstream files
```

Example:

```text
Risk: HIGH

Target:
src/payment/service.py

Prediction:
Likely downstream modification

Historical evidence:
Frequently changed with orders/service.py

Temporal evidence:
Previous payment changes were followed by
orders and billing changes.

Structural evidence:
6 dependent modules

Confidence:
0.82
```

---

## Explanation Requirements

* [x] Separate evidence from prediction (`evidence.py` vs `prediction`)
* [x] Show important contributing features (`top_evidence`, per-model contributions)
* [x] Show historical examples (`historical_examples.py`, strictly-before-target rows only)
* [ ] Show predicted impact path (deferred to Phase 8 dashboard graph)
* [x] Provide confidence (`confidence.py`, documented low/medium/high thresholds)
* [x] Avoid unsupported explanations (direction `unknown` unless defensible; empty list instead of fabrication)

Optional:

* [ ] SHAP-based model explanations
* [ ] Natural-language explanation layer
* [ ] RAG-based historical retrieval

RAG/LLM should remain optional and should **not determine the risk prediction**.

---

# Phase 8 — Interactive Dashboard

**Status: ✅ Complete (tabbed dashboard: Overview, Predictions, Graph, Files, History + file drawer; predictions served by new backend endpoints; 37 frontend tests green)**

Dashboard (`frontend/src/components/Dashboard.tsx` + `Header.tsx`):
sections Overview / Predictions / Graph / Files / History, file-details
drawer with Phase 7 evidence, historical examples, confidence and
recommendations. Predictions come from `GET
/api/repositories/{owner}/{repo}/predictions` and `POST
/api/local/predictions`, computed by `backend/src/prediction/` running the
exported Phase 6 logistic-regression baseline
(`backend/src/prediction/weights.json` via
`ml/scripts/export_baseline_weights.py`) over leakage-safe dataset rows.

### Goal

Provide a developer-friendly interface for exploring repository behavior.

---

## Repository Overview

* [x] Repository metadata
* [x] Source statistics
* [x] Complexity overview
* [x] Dependency overview
* [x] Historical activity
* [x] Prediction explorer with search/filter/sort (`Predictions.tsx`)
* [x] File details with evidence, confidence, recommendations (`FileDrawer.tsx`)
* [x] Interactive neighborhood graph with relationship types (`RepoGraph.tsx`)
* [x] File explorer tree (`FileExplorer.tsx`)
* [x] Commit history view (`HistoryView.tsx`)

---

## Change-Risk View

Developer selects a file or change target:

```text
┌──────────────────────────────────┐
│ src/payment/service.py           │
├──────────────────────────────────┤
│ Risk: HIGH                       │
│ Confidence: 82%                  │
│                                  │
│ Likely impact:                   │
│ payment → orders → billing       │
│                                  │
│ Historical evidence              │
│ Structural evidence              │
└──────────────────────────────────┘
```

---

## Repository Exploration

* [x] File-level analysis
* [x] Dependency graph
* [x] Change history
* [x] Propagation paths
* [x] Historical examples
* [x] Risk explanation

---

# Phase 9 — Integration, Testing & Benchmarking

**Status: 🚧 In Progress (09.1–09.7 implemented 2026-09-22; large-repo + memory profiling remain open)**

### Goal

Turn the research prototype into a reliable working system.

---

## Backend

* [x] API integration tests — error-status mapping covered by `main.rs` unit tests (`local_path_errors_map_to_client_statuses`, `github_api_errors_map_to_gateway_statuses`, `github_rate_limit_maps_to_429`); 91 backend tests green
* [x] GitHub error handling — `GithubError` → 404/401/429/502 mapping in `status_code_from`, retries + rate-limit waits in `github/client.rs`
* [ ] Large repository testing — NOT DONE (largest exercised: 74-file/14-commit self repo)
* [x] Rate-limit handling — 429 mapping + client-side backoff; 502 hint in frontend when backend unreachable
* [x] Request timeout handling — 60s reqwest timeout + transient retries
* [x] Bounded resource usage — analysis cache cap, 1MB blob limit, 8-way bounded concurrency, 300-entry timeline cap

---

## ML

* [x] Reproducible training pipeline — `train_baseline` (fixed seed 42), `compare_models`, `evaluate_time_windows_from_path`
* [x] Dataset versioning — `ml/data/dataset.jsonl` (347 rows), `ml/data/repoinsight-self.jsonl` (491 rows), `ml/data/repo-ranger.jsonl` (111 rows); provenance recorded in `backend/src/prediction/weights.json`
* [x] Model versioning — `model_name` + `calibrated` in weights.json and prediction responses
* [x] Evaluation reports — `ml/benchmark_results.json` (9 records: 3 repos × 3 models, real measurements, null where undefined)
* [x] Error analysis — TP/FP/TN/FN recorded per benchmark record via `ml/scripts/benchmark.py`
* [x] Benchmark datasets — 3 real local/GitHub-derived datasets (see `ml/scripts/benchmark.py --help`)

---

## Performance

Measured (benchmark runner records per-repo/per-model timings):

* Repository collection time — via analysis endpoint latency (not yet systematically recorded)
* Source parsing time — NOT MEASURED separately
* History processing time — NOT MEASURED separately
* Feature-generation time — recorded as `analysis_time_seconds` (load + split + featurize)
* Prediction latency — recorded as `prediction_time_seconds` (fit + evaluate)
* Memory usage — NOT MEASURED

Test with:

```text
Small repository — Repo_Ranger (14 files, 19 commits, 111 rows) ✅ measured
Medium repository — RepoInsight self (74 files, 14 commits, 491 rows) ✅ measured
Large repository — NOT DONE
```

---

## Phase 9 implementation notes (2026-09-22)

* Recall=N/A root cause (rework detector): `recall(0, 0)` → `None` because all 6 hand-reviewed labels are `actual_rework=false` — mathematically expected, covered by `zero_denominators_yield_none` test, and the dashboard already explains it ("Recall is n/a: the sample contains no actual rework"). ML `evaluate_model` can never emit recall N/A (`zero_division=0`); only ROC-AUC/PR-AUC are `None` on single-class splits.
* 23-feature pipeline verified: `ml/src/repoinsight_ml/features.py` (`9 structural + 6 historical + 8 temporal`); backend API structs carry the same 23 numerics plus `file_path`.
* Frontend: `analysisErrorHint` exported + tested (`errorHints.test.ts`); 45 frontend tests green. Responsive verified by breakpoint inspection (767/900/1024/1100px); no redesign performed.

## Phase 9 implementation notes — end-to-end instrumentation (2026-09-22)

* Completed: `AnalysisTimings` (`backend/src/timing.rs`) with per-stage
  milliseconds (validation, tree/commit/source loading, structural
  analysis, dataset construction, total) plus file/commit/source/row
  counts; stages stay `None` until complete so errors never fabricate timings.
* Completed: `POST /api/local/timings` diagnostic endpoint running the
  real local pipeline and returning repository metadata with timings.
  Dataset NDJSON and analysis response contracts are unchanged (98 backend
  tests green, incl. temp-repo integration tests; no Axum data used in tests).
* Completed: CPU-bound/blocking local work (working-tree reads,
  tree-sitter analysis, dataset construction, prediction scoring) isolated
  via `spawn_blocking`; lightweight requests such as `GET /health` no
  longer share async workers with long analyses. No job queues added.
* Completed: `scripts/measure_local_analysis.py --repo PATH --output FILE`
  external diagnostic wrapper (manual use only, never auto-run on large repos).
* Notes: end-to-end repository-ingestion timings are now available and
  remain separate from ML benchmark timings (`ml/benchmark_results.json`
  still covers load/split/featurize + fit/evaluate only).
* Notes: large-repository peak-memory measurements require external
  profiling (`/usr/bin/time -v`); in-process RSS is intentionally not
  implemented (platform-dependent). The earlier ~240 MB Axum observation
  was uncontrolled and is NOT recorded as an official measurement.
* Still open: large-repository test, per-stage parse/history timing
  split-out, memory profiling.

## Phase 9 implementation notes — research evaluation (2026-09-22)

* Completed 9.6 Multi-Repository Benchmarking: `ml/scripts/benchmark.py`
  (+`--envelope` run metadata, positive/test-positive rates) over 4 real
  datasets → `ml/multi_repo_benchmark.json` (12 records: 4 repos × 3
  models). Existing `ml/benchmark_results.json` untouched.
* Completed 9.7 Model Comparison: `ml/scripts/run_model_comparison.py` →
  `ml/model_comparison.json` (repository, model, precision/recall/F1,
  ROC-AUC, PR-AUC, prediction time + interpretation note).
* Completed 9.8 Ablation Testing: `ml/src/repoinsight_ml/ablation.py`
  (canonical groups structural 9 / historical 6 / temporal 8; configs
  all/single/pairs, LR baseline, one fixed chronological split) +
  `ml/scripts/run_ablation.py` → `ml/ablation_results.json` (28 records).
* Completed 9.9 Error Analysis: `ml/error_analysis.py` →
  `ml/error_analysis.json` (per repo/model: FP/FN counts + rates, test
  positive rate, ≤25 FP + ≤25 FN examples with commit/file/probability/
  feature context, observed per-group feature means; associational
  language only).
* Methodology notes: repositories are evaluated independently;
  chronological commit-granular splits prevent temporal leakage (same rows
  back every ablation config); model metrics are reported without ranking;
  ablation shows how performance changes when groups are removed; error
  analysis reports observed FP/FN patterns without causal claims.
* Class imbalance: axum positive rate 0.0097 (test 0.0060) — precision
  ~0.01–0.02 with recall up to 0.60, PR-AUC ~0.02–0.06 while ROC-AUC
  ~0.58–0.72. Accuracy is never reported; interpretation uses
  positive_rate/precision/recall/PR-AUC. Training algorithms unchanged.

---

# Phase 10 — Final Research Evaluation

**Status: ⏳ Planned**

### Goal

Evaluate whether RepoInsight actually provides useful predictive information beyond simple static metrics.

---

## Experiments

### Experiment 1 — Baseline

Compare against simple historical/static features.

```text
Static metrics
      ↓
Baseline model
```

### Experiment 2 — Temporal Features

```text
Static + historical features
            ↓
          Model
```

### Experiment 3 — Temporal Change Sequences

```text
Static
  +
Historical
  +
Temporal sequences
       ↓
     Model
```

### Experiment 4 — Ablation

Remove feature groups individually:

```text
Structure only
History only
Temporal only
Structure + History
Structure + History + Temporal
```

This helps determine whether temporal information actually improves prediction.

---

# Final System

The intended final workflow is:

```text
                 GitHub Repository
                        │
              ┌─────────┴─────────┐
              ▼                   ▼
       Source Structure       Git History
              │                   │
              ▼                   ▼
       Static Features      Temporal Features
              │                   │
              └─────────┬─────────┘
                        ▼
                 Feature Dataset
                        │
                        ▼
                 Trained ML Model
                        │
                        ▼
              Change Risk Prediction
                        │
             ┌──────────┴──────────┐
             ▼                     ▼
       Impact Prediction      Rework Risk
             │                     │
             └──────────┬──────────┘
                        ▼
                Evidence + Explanation
                        │
                        ▼
                   Dashboard
```

---

# Definition of Done

RepoInsight will be considered complete when it can:

1. Accept an arbitrary supported GitHub repository.
2. Collect its source structure and historical changes.
3. Build a chronological representation of repository evolution.
4. Extract structural, historical, and temporal features.
5. Construct leakage-safe training data.
6. Train and evaluate a predictive model.
7. Predict change propagation/rework risk for a target file or change.
8. Provide historical evidence supporting the result.
9. Explain the prediction without confusing model output with observed facts.
10. Display the result through an interactive developer dashboard.

---

# Current Priority

The immediate development order is:

```text
Phase 3
   ↓
Connect Tree-sitter to source analysis
   ↓
Build normalized file-level representation
   ↓
Phase 4
   ↓
Build chronological change representation
   ↓
Detect propagation/rework patterns
   ↓
Phase 5
   ↓
Create leakage-safe dataset
   ↓
Phase 6
   ↓
Train and evaluate ML model
```

