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
       GitHub Repository    Python ML Layer
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