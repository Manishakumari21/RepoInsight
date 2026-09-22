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
