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

**Status: ✅ Complete (verified 2026-09-24 against `backend/src/analysis/`)**

### Goal

Transform raw repository data into structured information that can later be used for temporal modeling.

### Evidence

* Source metadata normalized: `SourceFile { path, content, size_bytes }` (`source.rs`), aggregates in `SourceAnalysis` (`models.rs`).
* File-level records: per-file `StructuralFeatures` (path, size, LOC, functions, complexity, nesting, in/out dependencies, coupling, dependents) built by `features::build_structural_features` from `analyzer.rs::analyze_source_files`.
* Complexity connected to parsed source: `analyze_source_files` → tree-sitter `ParsedSource` → `analyze_complexity` → `build_complexity_analysis` (`analyzer.rs:156-260,565-630`).
* File change timelines: chronological `ChangeTimeline.entries` plus `fileCommitStamps` consumer in the frontend (`lib/analysis.ts`).

---

## 3.1 Source Analysis

* [x] Source-file representation
* [x] Source line counting
* [x] Source size calculation
* [x] Binary-file filtering
* [x] Source collection pipeline

### Completed (verified against current code)

* [x] Normalize source metadata
* [x] Add file-level analysis records
* [x] Connect source analysis to repository analysis

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

### Completed (verified against current code)

* [x] Connect complexity analysis to parsed source
* [x] Produce file-level complexity metrics
* [x] Validate metrics across supported languages (parser tests per language)

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

### Completed (verified against current code)

* [x] Build file change timelines (`ChangeTimeline.entries`)
* [x] Represent commit change sets (per-entry files, additions/deletions, authors)

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
evidence string. Historical evidence (`observed`/`derived`) is kept distinct
from model output: ML risk/confidence values are produced downstream by the
Phase 6 baseline and always labeled with the model name and uncalibrated
status (see Phase 6/7).

The prediction target definition (one file in one target commit, label 1 =
changed / 0 = eligible but unchanged) was established experimentally in
Phase 5 rather than chosen arbitrarily; see Phase 5.4.

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

**Status: ✅ Complete (verified 2026-09-24; former “Planned” label was stale)**

Shipped: exported logistic-regression baseline (`ml/src/repoinsight_ml/train.py` →
`backend/src/prediction/weights.json`, served by `backend/src/prediction/mod.rs`
via `GET /api/repositories/{owner}/{repo}/predictions` and
`POST /api/local/predictions`); Random Forest / Histogram Gradient Boosting
comparison (`compare.py`, `ml/model_comparison.json`); chronological
commit-granular evaluation (`time_evaluation.py`, precision/recall/F1/ROC-AUC/PR-AUC);
calibration utilities (`calibration.py`, predictions honestly labeled uncalibrated);
feature importance (`feature_importance.py`); model persistence
(`model_persistence.py`); ablation (`ablation.py` → `ml/ablation_results.json`);
error analysis (`ml/error_analysis.py` → `ml/error_analysis.json`); multi-repo
benchmark (`ml/scripts/benchmark.py` → `ml/multi_repo_benchmark.json`).

### Goal

Train a model that predicts whether a change is likely to result in downstream modification or rework.

---

## 6.1 Baseline

Implemented:

* [x] Logistic Regression (`train.py::train_baseline`, seed 42)
* [x] Baseline metrics (precision/recall/F1/ROC-AUC/PR-AUC, `zero_division=0`)
* [x] Feature preprocessing (`preprocessing.py`, 23 features: 9 structural + 6 historical + 8 temporal)

The baseline establishes whether the engineered features contain useful predictive information.

---

## 6.2 Candidate Models

Evaluated CPU-friendly models:

* [x] Random Forest
* [x] Gradient Boosting / Histogram Gradient Boosting
* [x] Logistic Regression

The final model was selected based on measured performance rather than assumed superiority (per-repo results in `ml/model_comparison.json`, no cross-repo ranking — class distributions differ).

---

## 6.3 Evaluation

Measured:

* [x] Precision
* [x] Recall
* [x] F1
* [x] ROC-AUC
* [x] PR-AUC
* [x] Calibration (utilities in `calibration.py`; served probabilities labeled uncalibrated)

Particular attention was given to precision/recall because false risk warnings reduce developer trust (class-imbalance regime documented, e.g. axum positive rate 0.0097).

---

## 6.4 Model Validation

* [x] Chronological validation (`time_evaluation.py`, strictly-before-T prefixes)
* [x] Cross-repository evaluation where possible (4 repos × 3 models, `multi_repo_benchmark.json`)
* [x] Feature ablation (`ablation.py`, 7 group configurations)
* [x] Baseline comparison (ripple `compare_ripple_models`, impact `compare_impact_models`)
* [x] Error analysis (`error_analysis.py`, FP/FN patterns)
* [x] Model persistence (`model_persistence.py`, `export_baseline_weights.py`)

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

**Status: ✅ Complete (repository-analysis workbench: Overview, Structure, History, Coupling, Predictions, Evidence + file inspector/drawer; predictions served by backend endpoints; Ripple and Impact Simulator under the Predictions view)**

Workbench (`frontend/src/components/Dashboard.tsx`, views defined in
`frontend/src/lib/sections.ts`): Overview / Structure / History (timeline) /
Coupling / Predictions (predicted changes, ripple, impact simulator tabs) /
Evidence, with a file inspector/drawer carrying Phase 7 evidence, historical
examples, confidence and recommendations. Predictions come from `GET
/api/repositories/{owner}/{repo}/predictions` and `POST
/api/local/predictions`, computed by `backend/src/prediction/` running the
exported Phase 6 logistic-regression baseline
(`backend/src/prediction/weights.json` via
`ml/scripts/export_baseline_weights.py`) over leakage-safe dataset rows.
Ripple (`RippleForecast.tsx`) and Impact Simulator (`ImpactSimulator.tsx`)
live under the Predictions view's related/impact tabs.

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

**Status: ✅ Complete (final completion pass 2026-09-24, Candidate #4 Steps 1–8
complete; backend 133 tests green as verified by `cargo test`; release build
succeeds; only branch-varying cache key and per-commit detail endpoint remain
open by design — see remaining limitations below)**

### Goal

Turn the research prototype into a reliable working system.

---

## Backend

* [x] API integration tests — error-status mapping covered by `main.rs` unit tests (`local_path_errors_map_to_client_statuses`, `github_api_errors_map_to_gateway_statuses`, `github_rate_limit_maps_to_429`); backend 133 tests green (verified by `cargo test`; frontend 71 and ML 81 green as of the 2026-09-24 pass)
* [x] GitHub error handling — `GithubError` → 404/401/429/502 mapping in `status_code_from`, retries + rate-limit waits in `github/client.rs`
* [x] Large repository testing — DONE 2026-09-24: axum (504 files, 1000 commits, 501 source files) → 209,477 dataset rows, see Performance below
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
Large repository — Axum (504 files, 1000 commits, 501 source files, 209,477 rows) ✅ measured (see table below)
```

End-to-end ingestion (2026-09-24, `POST /api/local/timings` on this checkout):
165 files, 21 commits, 159 source files, 1379 dataset rows —
validation 0 ms, tree 41 ms, commits 50 ms, source 4 ms,
structural 752 ms (includes tree-sitter parsing), dataset 1543 ms,
total 2395 ms, wall 2.42 s.

Large-repository ingestion (2026-09-24, same endpoint/method, `/home/manisha/axum`,
504 files, 1000 commits, 501 source files, 209,477 dataset rows — row count
matches the previously verified value exactly):

| run | validation | tree | commits | source | structural | dataset | total | wall |
|---|---|---|---|---|---|---|---|---|
| 1 | 0 ms | 44 ms | 89 ms | 75 ms | 556 ms | 86,402 ms | 87,244 ms | 87.3 s |
| 2 | 0 ms | 28 ms | 72 ms | 7 ms | 600 ms | 98,921 ms | 99,690 ms | 99.7 s |

Peak RSS (server `VmHWM` after axum analysis): 227 MB. Dataset construction
(~99% of total) previously recomputed temporal/propagation/follow-up/rework
prefixes per target commit — O(T × prefix analysis) over 1000 targets.
Candidate #4 Steps 1–8 are now COMPLETE (see "Candidate #4 optimization
sequence" below): borrowed prefix slices, monotonic `seen`, incremental
historical accumulator, shared co-change state, incremental temporal,
propagation, and file-timestamp accumulators, plus the follow-up/rework
accumulator — each checked against its original/reference implementation
with equivalence tests. Row output remains byte-identical at 209,477 rows
with exact leakage-safe semantics preserved; responses are served from the
300 s analysis cache.

Memory: process RSS via `/proc` (`VmHWM` above); in-process RSS intentionally
not implemented (platform-dependent).

---

## Phase 9 implementation notes (2026-09-22)

* Recall=N/A root cause (rework detector): `recall(0, 0)` → `None` because all 6 hand-reviewed labels are `actual_rework=false` — mathematically expected, covered by `zero_denominators_yield_none` test, and the dashboard already explains it ("Recall is n/a: the sample contains no actual rework"). ML `evaluate_model` can never emit recall N/A (`zero_division=0`); only ROC-AUC/PR-AUC are `None` on single-class splits.
* 23-feature pipeline verified: `ml/src/repoinsight_ml/features.py` (`9 structural + 6 historical + 8 temporal`); backend API structs carry the same 23 numerics plus `file_path`.
* Frontend: `analysisErrorHint` exported + tested (`errorHints.test.ts`); 71 frontend tests green as of 2026-09-24 (10 files). Responsive verified by breakpoint inspection (767/900/1024/1100px); no redesign performed.

## Phase 9 implementation notes — end-to-end instrumentation (2026-09-22)

* Completed: `AnalysisTimings` (`backend/src/timing.rs`) with per-stage
  milliseconds (validation, tree/commit/source loading, structural
  analysis, dataset construction, total) plus file/commit/source/row
  counts; stages stay `None` until complete so errors never fabricate timings.
* Completed: `POST /api/local/timings` diagnostic endpoint running the
  real local pipeline and returning repository metadata with timings.
  Dataset NDJSON and analysis response contracts are unchanged (backend 133
  tests green as verified by `cargo test`, incl. temp-repo integration tests; no Axum data used in tests).
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
  implemented (platform-dependent). Peak server `VmHWM` after the 2026-09-24
  axum analysis run above was 227 MB (observed via `/proc`, replaces the
  earlier uncontrolled ~240 MB observation).
* Timing granularity (honest accounting): `AnalysisTimings` splits
  tree / commit / source loading + structural analysis + dataset
  construction. Tree-sitter parse time lives inside
  `structural_analysis_ms` and is not reported separately — no separate
  parse-vs-history numbers are fabricated.
* Large-repository runs now DONE (see Performance table above); the
  "still open" operator-run items below are closed by those measurements.
  Reproduce with: `cargo run` in `backend/`, then
  `python3 scripts/measure_local_analysis.py --repo /path/to/large-repo --output /tmp/opencode/timings.json`
  and `/usr/bin/time -v curl -X POST localhost:3000/api/local/timings
  -H 'Content-Type: application/json' -d '{"path":"/path/to/large-repo"}'`.

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

## Final completion pass (2026-09-24)

Full audit → implement → test → benchmark → document, all changes left
uncommitted for human review. No commits or pushes made.

* Audit: full project inspected (backend 28 files, frontend ~90, ML 23 +
  scripts/tests). Findings classified P0/P1/P2; no P0 issues found —
  leakage discipline (strict `<` cutoffs, equal-timestamp exclusion,
  untimestamped-commit filtering), path canonicalization, fixed-arg git
  invocation, and error→status mapping all verified in current code.
* Preserved: Candidate #1 (change_order_count), #2 (direct propagation,
  petgraph removed), #4 Steps 1–8 (borrowed prefix slices, monotonic `seen`,
  incremental historical accumulator, shared co-change state, incremental
  temporal / propagation / file-timestamp / follow-up-rework accumulators,
  each with equivalence tests against the reference behavior) — all verified
  sound, row output unchanged at 209,477.
* Fixed (P1/P2):
  - `frontend/src/components/Dependencies.tsx` — raw coupling counts were
    rendered as percentages (`coupling * 100` + `%`); now shows link counts
    with bars scaled relative to the repository peak. README limitation note
    updated.
  - `Hotspots.tsx` / `Risk.tsx` panel hints now state heuristic status
    ("Heuristic percentile signals, not ML predictions" /
    "Heuristic aggregate, not an ML prediction").
* Completed: Candidate #4 Steps 5–8 (incremental temporal / propagation /
  file-timestamp / follow-up-rework accumulators). Each step preserves exact
  reference semantics — strict `<` cutoffs, equal-timestamp group atomicity,
  untimestamped-commit exclusion — with full retained state where needed to
  preserve exact top-K behavior, and follow-up/rework classification deferred
  where the final prefix co-change state can affect classification. See
  "Candidate #4 optimization sequence" below for per-step observations.
* Verified: `cargo fmt --check` clean, backend 133 passed (verified by
  `cargo test`; frontend 71 and ML 81 passed as of the 2026-09-24 pass), `cargo build --release` + `npm run build` succeed,
  `run_model_comparison.py` reproduces `model_comparison.json` byte-identical,
  axum benchmark 209,477 rows on both runs, peak server `VmHWM` 227 MB.
* Research evaluation (`docs/research-evaluation.md`, Phase 10) unchanged —
  numbers already honest and reproducible; no new ML claims made.

---

## Candidate #4 optimization sequence (Steps 1–8 COMPLETE)

The dataset pipeline previously reconstructed historical state from scratch
for every target commit. Candidate #4 replaced this with advance-only
incremental accumulators over the same grouped-timestamp ranges the dataset
builder uses for `seen` (commits with `timestamp < cutoff`, never the target
itself). No step relaxes the temporal leakage guarantees.

* Step 1 — borrowed historical-prefix slices: filter-then-clone replaced with
  `partition_point` borrows over the `(timestamp, sha)`-ordered commits;
  identical selection and order, including strict `<` equal-timestamp
  exclusion and untimestamped-commit filtering.
* Step 2 — monotonic seen-set advancement: each historical commit is folded
  into `seen` exactly once as the cutoff only moves forward.
* Step 3 — incremental historical feature accumulator
  (`HistoricalPrefixAccumulator` in `historical_features.rs`): advanced over
  the same grouped ranges as `seen`, materialized per target into the
  identical output shape; checked against the reference implementation with
  equivalence tests.
* Step 4 — shared incremental co-change state: exact reuse of the
  history-analysis counting, so equal commits contribute equally regardless
  of arrival order.
* Step 5 — incremental temporal accumulator (`TemporalPrefixAccumulator` in
  `history.rs`): pair counting, window breaks, ignored-path filtering,
  self-pair exclusion, and top-50 truncation identical to
  `temporal_analysis`; only repeated full-prefix rescans replaced by
  monotonic accumulation. Checked with
  `incremental_temporal_matches_oracle_at_every_cutoff`. Observation: modest
  likely-real improvement, approximately 12% mean dataset-time reduction,
  but machine variance was significant.
* Step 6 — incremental propagation accumulator
  (`PropagationPrefixAccumulator` in `history.rs`): static dependency
  contributions folded once; co-change contributions folded per newly
  eligible commit; the current top-50 temporal list folded at materialize
  time; the merged map stays full so late-rising pairs can always enter the
  top 100. Checked with
  `incremental_propagation_matches_oracle_at_every_cutoff` (including a
  late-riser case crossing the top-100 boundary). Observation: modest
  likely-real improvement, approximately 8 seconds / approximately 21%
  versus the original baseline, with noise acknowledged.
* Step 7 — incremental file-timestamp accumulator (`FileTimesAccumulator`
  in `temporal_features.rs`): full per-file timestamp vectors over exactly
  the commits with `ts < current`, lent by reference instead of rebuilt and
  re-sorted per target. Checked with
  `incremental_file_times_matches_oracle_at_every_cutoff`. Observation: no
  measurable speedup; kept because it removes repeated allocations and
  enables Step 8.
* Step 8 — incremental follow-up/rework accumulator
  (`FollowupReworkPrefixAccumulator` in `propagation_history.rs`): pair
  discovery folded once per newly eligible commit; classification runs at
  materialize time with the present prefix co-change map, exactly as the
  reference classifies pairs with the full prefix map. Checked with
  `incremental_followup_rework_matches_oracle_at_every_cutoff`.
  Observation: repeatable improvement, approximately 18.6 seconds /
  approximately 24% versus the Step-7 state.

Steps 5–8 were never deferred or skipped: all four are implemented, with
equivalence coverage against the reference/oracle behavior at every cutoff.
Row output is unchanged at 209,477 rows on the Axum benchmark.

---

# Phase 10 — Final Research Evaluation

**Status: ✅ Complete (evaluation battery executed 2026-09-23 on
current code; consolidated in `docs/research-evaluation.md`)**

## Phase 10 evaluation runs (2026-09-23)

* Re-ran the full battery on current code (Python 3.14.7, sklearn 1.9.0,
  seed 42): `benchmark.py` → `multi_repo_benchmark.json` (12 records),
  `run_ablation.py` → `ablation_results.json` (28), `run_model_comparison.py`
  → `model_comparison.json`, `error_analysis.py` → `error_analysis.json`.
* New: `run_ripple_evaluation.py --repo … --out` (multi-repo) →
  `ml/ripple_results.json` (all 4 repos; top-1 always correct, full ==
  co-change-only — same-commit ground truth documented as the cause).
* New: `run_impact_evaluation.py --git-repo …` (messages as descriptions,
  strictly-before-T prefixes; keyword/co-change/combined) →
  `ml/impact_results.json` on self history (near-null: 1 evaluable target
  out of 16 coarse commits — harness verified, validation left open).
* New: `docs/research-evaluation.md` consolidates every number with
  honest readings (no ranking, no causal claims, imbalance regime stated).
* Axum ripple cell filled via vectorized `build_ripple_scores`
  (numpy commit×file incidence algebra; outputs verified byte-identical
  to the pure-Python version on all small datasets before rerunning).
* Artifacts are git-ignored and reproducible via `ml/scripts/`.

## Ripple Forecasting (shipped)

* [x] Backend signals (`backend/src/analysis/ripple.rs`): structural,
  co-change P(B|A), temporal follow-up with median/avg delay, recent
  coupling; documented weighted baseline, configurable `RippleConfig`
  (max depth 5, max candidates 10, min confidence 0.5)
* [x] Leakage-safe prefixes + chronological P@K/R@K/MRR/MAP evaluation
  (`evaluate_ripple`); leakage regression tests
* [x] Ordered cycle-free ripple paths; neutral missing-impact detection
* [x] API: `GET /api/repositories/{owner}/{repo}/ripple`, `POST
  /api/local/ripple` (cached, `spawn_blocking`)
* [x] Frontend Ripple section (candidates, path, evidence, historical
  examples, missing impact; ripple edges visually distinct from dependency
  edges); 6 new tests
* [x] ML ranking comparison (`ml/src/repoinsight_ml/ripple.py`,
  `ml/scripts/run_ripple_evaluation.py`): co-change baseline vs full model

## Impact Simulator (shipped)

* [x] Deterministic intent extraction (`backend/src/analysis/impact.rs`):
  operation verbs + domain/technology vocabularies, no LLM, no network
* [x] Derived capability map (directory names, filenames, source content;
  no manual definitions) with per-capability review checklist
* [x] Impact discovery over semantic, dependency, historical co-change,
  and test/config/docs signals; normalized weighted baseline
  (`ImpactConfig`: semantic 0.35, dependency 0.20, historical 0.30,
  kind 0.15), HIGH/MEDIUM/LOW candidate levels
* [x] Impact map (planned change → capabilities → files) with per-edge
  evidence; scenarios stay independent for side-by-side comparison
* [x] API: `GET /api/repositories/{owner}/{repo}/impact-simulation`,
  `POST /api/local/impact-simulation` (cached, `spawn_blocking`)
* [x] Frontend Simulator section (intent display, level-grouped impact,
  evidence, impact map, checklist); 7 new tests
* [x] Chronological evaluation (`evaluate_impact`: commit message as
  description, strictly-before-T prefixes; P@K/R@K/F1@K, directory
  accuracy, test/config discovery; keyword/dependency/co-change/combined
  baselines) + ML parity (`ml/src/repoinsight_ml/impact.py`,
  `compare_impact_models`); leakage regression tests

Research positioning: RepoInsight integrates intent extraction, derived
capability maps, structural dependencies, historical co-change, and
explainable evidence to estimate planned-change impact before
implementation. Baseline comparisons measure each signal's contribution,
never overclaimed.

Research positioning: RepoInsight integrates structural dependencies,
historical co-change, temporal ordering, recent behavior, and explainable
evidence to forecast likely propagation paths. Temporal value-add is
measured (baseline vs full), never overclaimed.

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

# Current Status

Implementation is substantially complete for the current research/tooling
scope; Phases 1–10 are done as described above. The working system:

1. Accepts arbitrary supported GitHub repositories and local Git repositories.
2. Collects source structure and historical changes.
3. Builds a chronological representation of repository evolution.
4. Extracts structural, historical, and temporal features.
5. Constructs leakage-safe training data (Candidate #4 Steps 1–8 complete).
6. Trains and evaluates predictive baselines with chronological splits.
7. Predicts change propagation/rework risk per file or change, plus ripple
   forecasts and planned-change impact simulations.
8. Provides historical evidence supporting each result.
9. Explains predictions without confusing model output, heuristic signals,
   or observed facts (uncalibrated probabilities labeled as such).
10. Displays results through the interactive analysis workbench.

Remaining work is limited to explicitly documented limitations and future
research (branch-aware caching, per-commit detail endpoint, persistent
storage, richer propagation forecasting, larger multi-repository
evaluation). Final demo/packaging remains future work.

