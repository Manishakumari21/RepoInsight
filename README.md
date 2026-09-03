# Repository Intelligence & Risk Analyzer

An ML-powered developer tool that analyzes a GitHub repository to help developers understand its structure, identify complex and risky code, and predict areas that may become difficult to maintain.

## 🚀 Why This Project?

Understanding an unfamiliar repository can take significant time. Developers often need to manually inspect:

- Complex files and modules
- Frequently changing code
- Dependencies and coupling
- Bug-prone areas
- Repository history

This project brings these signals together into a single interactive analysis.

## ✨ Key Features

- 🔍 **Repository Analysis** — Analyze a repository using its GitHub URL.
- 📊 **Code Complexity** — Measure complexity and identify difficult files.
- 🔄 **Change & Churn Analysis** — Analyze how frequently code changes.
- 🔗 **Dependency Analysis** — Identify highly connected files/modules.
- ⚠️ **Risk Detection** — Detect current maintenance hotspots.
- 🤖 **ML Risk Prediction** — Predict whether a file is likely to become a future maintenance hotspot.
- 💡 **Risk Explanation** — Show the factors contributing to a file's risk score.
- 📈 **Interactive Dashboard** — Explore repository metrics, hotspots, history, and dependencies.

## 🧠 Machine Learning

The project treats repository risk prediction as a **tabular classification problem**.

### Features

Examples of extracted features include:

- Lines of Code (LOC)
- Cyclomatic Complexity
- Code Churn
- Commit Frequency
- Number of Contributors
- Number of Dependencies
- Previous Bug-Fix Changes
- Complexity Trend

### Models

We compare multiple models:

1. Logistic Regression — baseline
2. Random Forest — primary baseline
3. XGBoost — performance comparison

**SHAP** can be used to explain the model's predictions.

The target is:

> **Will this file become a maintenance hotspot in the future?**

The model produces a risk probability, for example:

```text
src/analyzer/metrics.rs

Risk Score: 0.84
Risk Level: HIGH

## 🏗️ System Architecture

The system follows a modular architecture where repository collection, analysis, machine learning, and visualization are separated into independent components.

```text
                         ┌──────────────────────┐
                         │       Developer      │
                         │  GitHub Repository   │
                         │         URL          │
                         └──────────┬───────────┘
                                    │
                                    ▼
                    ┌────────────────────────────┐
                    │       React Frontend       │
                    │                            │
                    │ • Repository Overview      │
                    │ • Complexity Dashboard     │
                    │ • Risk Analysis            │
                    │ • Hotspots                  │
                    │ • Dependency Graph          │
                    └──────────────┬─────────────┘
                                   │
                              REST API
                                   │
                                   ▼
                    ┌────────────────────────────┐
                    │       Rust + Axum           │
                    │          Backend             │
                    │                             │
                    │ • API Layer                 │
                    │ • GitHub Client             │
                    │ • Repository Analyzer       │
                    │ • Git History Analyzer      │
                    │ • Dependency Analyzer       │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    ▼              ▼              ▼
             ┌────────────┐ ┌────────────┐ ┌─────────────┐
             │ GitHub API │ │ Git History│ │ Source Code │
             │            │ │            │ │             │
             │ Repository │ │ Commits    │ │ LOC         │
             │ Files      │ │ Churn      │ │ Complexity  │
             │ Issues/PRs │ │ Changes    │ │ Dependencies│
             └─────┬──────┘ └─────┬──────┘ └──────┬──────┘
                   │              │               │
                   └──────────────┼───────────────┘
                                  ▼
                    ┌────────────────────────────┐
                    │     Feature Extraction     │
                    │                            │
                    │ • Code Metrics             │
                    │ • Churn Metrics            │
                    │ • Dependency Metrics       │
                    │ • Historical Metrics       │
                    └──────────────┬─────────────┘
                                   │
                                   ▼
                    ┌────────────────────────────┐
                    │       Python ML Layer       │
                    │                            │
                    │ Logistic Regression        │
                    │ Random Forest              │
                    │ XGBoost                    │
                    └──────────────┬─────────────┘
                                   │
                                   ▼
                    ┌────────────────────────────┐
                    │     Risk Prediction        │
                    │                            │
                    │ Risk Score                 │
                    │ Risk Level                 │
                    │ Future Hotspot Prediction  │
                    └──────────────┬─────────────┘
                                   │
                                   ▼
                    ┌────────────────────────────┐
                    │     Explainability         │
                    │                            │
                    │ SHAP + Repository Context  │
                    │                            │
                    │ "Why is this file risky?"  │
                    └──────────────┬─────────────┘
                                   │
                                   ▼
                    ┌────────────────────────────┐
                    │    Interactive Dashboard   │
                    │                            │
                    │ Risk • Complexity • Churn  │
                    │ Hotspots • Dependencies    │
                    │ Historical Trends           │
                    └────────────────────────────┘