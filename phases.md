RepoInsight — Project Phases
Phase 1 — Project Foundation

Goal: Set up the project structure and development environment.

Create backend/, ml/, frontend/, docs/, tests/
Initialize Rust + Axum backend
Initialize Python ML environment
Initialize React + TypeScript frontend
Define API structure
Add .gitignore
Update README

Output: Empty but working full-stack project.

Phase 2 — GitHub Repository Collector

Goal: Accept a GitHub URL and collect repository information.

Build:

GitHub URL validation
Repository metadata collection
File tree collection
Commit history collection
Contributors
Issues / PR information
Repository language information

Example:

GitHub URL
    ↓
GitHub API
    ↓
Repository Data
    ├── Files
    ├── Commits
    ├── Contributors
    ├── Issues
    └── Pull Requests

Output: RepoInsight can understand the basic structure/history of a GitHub repository.

Phase 3 — Code & Repository Analysis

Goal: Calculate the current state of the repository.

For each file/module, calculate things like:

LOC
Cyclomatic complexity
Number of functions/classes
Dependencies
Coupling
File size
Language

Git-based metrics:

Commit frequency
Code churn
Number of contributors
Recent changes
Bug-fix history

Output: A structured dataset describing each file.

Phase 4 — Repository Intelligence Dashboard

Goal: Make the analysis useful before ML.

Create the first frontend.

Dashboard should show:

Repository Overview
       │
       ├── Complexity
       ├── Most Changed Files
       ├── Dependency Graph
       ├── Current Hotspots
       └── Repository Activity

Example:

Repository Health

Files              342
Commits            2,841
Contributors       27

High Complexity    18 files
High Churn         24 files
Hotspots           11 files

Output: User can enter a repo and actually explore it.

Phase 5 — Dataset Creation for ML ⭐

This is one of the most important phases.

We need historical repository data.

Instead of:

Current file → predict risk

we want:

Past state of file
       ↓
What happened afterward?
       ↓
Did it become a maintenance hotspot?

Build a dataset containing:

File + Historical Features → Future Outcome

Features could include:

Complexity
LOC
Churn
Commit frequency
Contributors
Dependencies
Previous bug fixes
Complexity growth

Target:

Did this file become a maintenance hotspot in the next N commits/time period?

Output: Training dataset.

Phase 6 — ML Risk Prediction

Now train the models.

Start simple:

Logistic Regression
        ↓
Random Forest
        ↓
XGBoost

Compare:

Accuracy
Precision
Recall
F1
ROC-AUC

Most importantly, use time-based evaluation:

Older repository history
        ↓
      TRAIN
        ↓
Newer repository history
        ↓
       TEST

Don't randomly mix future commits into training data.

Output:

File: src/analyzer.rs

Risk: 0.84
Level: HIGH
Phase 7 — Risk Explanation ⭐

A prediction alone isn't very useful.

The developer should know:

Why is this file risky?

Use SHAP to explain the ML prediction.

Example:

src/analyzer.rs

Risk: HIGH — 84%

Main factors:

↑ High code churn          +24%
↑ High complexity          +21%
↑ Frequently modified      +18%
↑ Many dependencies        +12%
↓ Few contributors          -4%

This makes the project much more useful than simply saying:

"Risk = 84%"

Output: Explainable risk predictions.

Phase 8 — Repository Intelligence + Risk Integration

Now combine everything.

                 RepoInsight
                      │
        ┌─────────────┴─────────────┐
        ↓                           ↓
 Repository Understanding       Risk Analysis
        │                           │
   Complexity                  ML Prediction
   Dependencies                Risk Score
   Git History                 SHAP Explanation
   Hotspots                    Future Hotspots
        └─────────────┬─────────────┘
                      ↓
             Developer Dashboard

The user can click a file and see:

src/analyzer.rs

Complexity:     High
Churn:          Very High
Dependencies:   12
Current Risk:   High
Future Risk:    84%

Why?
• Frequently modified
• Complexity increasing
• High dependency count
• Previous bug-fix activity
Phase 9 — Optional RAG / AI Explanation

Don't start with this.

After the core system works, we can add RAG/LLM to answer questions such as:

"Why is this file difficult to maintain?"

"What does this module do?"

"What should I be careful about when modifying it?"

RAG should explain the repository data, not decide the risk itself.

Repository Data
      ↓
Risk Model → Risk
      ↓
RAG/LLM → Human-readable explanation

This keeps the ML part scientifically meaningful.

Phase 10 — Testing, Benchmarking & Finalization

Test the complete system with multiple real repositories.

Evaluate:

Repository collection accuracy
Metric extraction
ML performance
Prediction quality
API performance
Frontend functionality

Then add:

Docker setup
Documentation
Architecture diagram
ML methodology
Dataset documentation
Demo repositories
🗺️ Overall Roadmap
PHASE 1
Project Foundation
       ↓
PHASE 2
GitHub Collector
       ↓
PHASE 3
Code + Git Analysis
       ↓
PHASE 4
Dashboard
       ↓
PHASE 5
ML Dataset
       ↓
PHASE 6
Risk Prediction
       ↓
PHASE 7
Risk Explanation
       ↓
PHASE 8
Full RepoInsight Integration
       ↓
PHASE 9
Optional RAG/AI
       ↓
PHASE 10
Testing + Benchmark + Final Demo
⭐ Most important point


Repository data → metrics → historical dataset → define risk → ML → explanation → UI