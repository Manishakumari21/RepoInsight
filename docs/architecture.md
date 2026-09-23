# RepoInsight Architecture

## Overview

RepoInsight is divided into three main components:

- **Frontend** — React + TypeScript interface
- **Backend** — Rust + Axum API
- **ML Layer** — Python-based data analysis and machine learning

## High-Level Architecture

```text
                    Developer
                        │
                        ▼
              ┌──────────────────┐
              │ React + TypeScript│
              │    Frontend      │
              └────────┬─────────┘
                       │
                    REST API
                       │
                       ▼
              ┌──────────────────┐
              │   Rust + Axum    │
              │     Backend      │
              └────────┬─────────┘
                       │
              ┌────────┴─────────┐
              ▼                  ▼
       GitHub / Local Git    Python ML Layer
              │                  │
              ▼                  ▼
       Repository Data      Risk Prediction
              │                  │
              └────────┬─────────┘
                       ▼
                Analysis Results
                       │
                       ▼
                  Dashboard
## Performance Instrumentation (Phase 9)

End-to-end repository-ingestion timings are measured by the backend,
separately from ML training/evaluation timings (see `ml/benchmark_results.json`):

- `backend/src/timing.rs` — `AnalysisTimings`: per-stage wall-clock
  milliseconds (each stage `None` until it completes, so failures never
  report fabricated timings) plus observed file/commit/source/row counts.
- `POST /api/local/timings` — diagnostic endpoint running the real local
  pipeline (load → analysis → dataset construction) and returning
  repository metadata with timings. Dataset NDJSON and analysis response
  contracts are unchanged.
- CPU-bound or blocking work on local paths (working-tree file reads,
  tree-sitter analysis, dataset construction, prediction scoring) runs on
  Tokio's blocking pool via `spawn_blocking`, keeping async workers free
  for lightweight requests such as `GET /health`.
- Memory is intentionally not measured in-process (platform-dependent);
  use external profiling, e.g. `/usr/bin/time -v`, or
  `scripts/measure_local_analysis.py --repo PATH --output FILE`.

## Change Ripple Forecasting (RepoInsight Ripple)

An explainable integration of structural, historical, and temporal signals —
not a novel ML technique. Given a changed file A, the backend
(`backend/src/analysis/ripple.rs`) combines:

- structural dependency (existing `DependencyEdge` graph, normalized 0/1),
- historical co-change P(B|A) (existing `cochange_pairs`),
- temporal follow-up P(B after A within 7 days, with median/avg delay),
- recent coupling (follow-up within the last 30 days),

into a documented weighted score
(`0.20/0.30/0.35/0.15`, configurable via `RippleConfig`). Candidates are
ranked (top 10, configurable confidence threshold); one greedy cycle-free
path (max depth 5) is built from high-confidence edges and omitted when
evidence is weak. Leakage prevention: training/evaluation features use only
commits strictly before T. Served by `GET
/api/repositories/{owner}/{repo}/ripple` and `POST /api/local/ripple` over
precomputed analysis structures (fast per-file queries, responses cached);
the dashboard Ripple section (`RippleForecast.tsx`) shows candidates, the
ordered path, evidence, historical examples, and neutral missing-impact
warnings. Ripple edges are visually distinct from dependency edges. ML-side
ranking comparison (co-change baseline vs full) lives in
`ml/src/repoinsight_ml/ripple.py`.

## Change Impact Simulator

A planning/architecture-analysis feature, distinct from file-level risk
prediction ("this file changed, what happens next?") and from ripple
forecasting ("when A changes, B tends to follow"). Here the change has
NOT happened: the developer describes an intention
("Replace JWT authentication with OAuth2") and the system estimates what
would need attention before implementation. No source code is modified.

Pipeline (`backend/src/analysis/impact.rs`, deterministic baseline):

1. **Intent extraction** — operation verbs (replace/add/remove/refactor/
   upgrade/…) plus domain and technology vocabularies matched against the
   description. No LLM, no network, no new dependencies.
2. **Capability map** — files grouped by best-matching domain alias, with
   fallback to top-level directory names. Derived per repository, never
   hand-written.
3. **Impact discovery** — semantic name/content matching, dependency
   adjacency to directly-matched files, historical co-change coupling,
   and test/configuration/documentation path-kind signals.
4. **Scoring** — each signal normalized to 0..1, combined with configurable
   `ImpactWeights` (semantic 0.35, dependency 0.20, historical 0.30, kind
   0.15); directly-matched files are high-impact candidates; the rest map
   to HIGH (≥0.50) / MEDIUM (≥0.15) / LOW candidate levels.
5. **Impact map + checklist** — planned change → capabilities → files with
   per-edge evidence, plus a review checklist linking each item to files.
   Each scenario request is independent, so alternative plans can be
   compared side by side.

Served by `GET /api/repositories/{owner}/{repo}/impact-simulation` and
`POST /api/local/impact-simulation` over the same precomputed
dependency/history/source structures as ripple (fast per-request queries,
responses cached); the dashboard Simulator section (`ImpactSimulator.tsx`)
shows intent, level-grouped impact, evidence, the impact map, and the
checklist. Chronological evaluation treats each test commit message as the
description with strictly-before-T prefixes
(`evaluate_impact`: precision@K, recall@K, F1@K, directory accuracy,
test/config discovery across keyword/dependency/co-change/combined
baselines); ML parity lives in `ml/src/repoinsight_ml/impact.py`.
