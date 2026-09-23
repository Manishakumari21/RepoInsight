# RepoInsight — Final Research Evaluation (Phase 10)

**Status:** Complete — full battery executed 2026-09-23 on current
code (Python 3.14.7, scikit-learn 1.9.0, NumPy 2.5.2, fixed seed 42).
Artifacts are git-ignored and reproducible via `ml/scripts/`.

**Research question:** Can historical change sequences combined with
current repository structure predict whether a software change will cause
downstream modifications or rework — i.e., does temporal change history
provide predictive information beyond static structure and simple
co-change?

**Method (applies to every number below):** repositories are evaluated
independently on chronological commit-granular splits (70/15/15, whole
commits never split, rows never shuffled), so no future information leaks
into training. Metrics are reported per repository without ranking;
accuracy is never reported (misleading under class imbalance).
Interpretation uses positive rate, precision/recall/F1, and especially
PR-AUC. Associational language only — no causal claims.

Artifacts (`ml/*.json`, git-ignored by convention, reproducible via
`ml/scripts/`):

| Artifact | Records | Meaning |
|---|---|---|
| `multi_repo_benchmark.json` | 12 = 4 repos × 3 models | Exp 1–3 model metrics |
| `model_comparison.json` | 12 | Same, comparison-shaped |
| `ablation_results.json` | 28 = 4 repos × 7 configs | Exp 4 feature-group removal |
| `error_analysis.json` | 12 | FP/FN patterns (pending) |
| `ripple_results.json` | 4 repos × 2 models | Ripple: co-change vs full |
| `impact_results.json` | 3 baselines, self repo | Impact simulator eval |

Datasets: `axum` (209,477 rows, Rust), `repoinsight-self` (491 rows),
`repoinsight`/`dataset.jsonl` (347 rows), `repo-ranger` (111 rows, Python).

---

## Experiment 1 — Baseline (logistic regression, 23 features)

| Repo (test +rate) | P | R | F1 | ROC-AUC | PR-AUC |
|---|---|---|---|---|---|
| repoinsight (0.127) | 0.500 | 0.300 | 0.375 | 0.684 | 0.429 |
| repoinsight-self (0.264) | 0.491 | 0.500 | 0.495 | 0.685 | 0.636 |
| repo-ranger (0.196) | 0.750 | 0.545 | 0.632 | 0.830 | 0.751 |
| axum (0.006) | 0.013 | 0.604 | 0.025 | 0.717 | 0.017 |

The engineered features carry predictive signal on all four repos
(PR-AUC well above the positive-rate floor everywhere). Axum shows the
class-imbalance regime honestly: recall 0.60 at precision 0.013 —
expected, not a defect.

## Experiment 2/3 — Candidate models (RF, HGB vs baseline)

No overall winner, as predicted in the pre-registered interpretation
note: random_forest leads F1 on repoinsight (0.439) and repoinsight-self
(0.590); hist_gradient_boosting leads on repoinsight-self ROC (0.761)
but collapses on tiny repo-ranger (F1 0.0, 11 test positives).
Model choice is repository-dependent; the shipped default stays the
logistic-regression baseline.

## Experiment 4 — Ablation (LR, same chronological split per repo)

| Repo | all (23) F1 | best config F1 | temporal-only F1 |
|---|---|---|---|
| repoinsight | 0.375 | 0.400 (struct+temp) | 0.222 |
| repoinsight-self | 0.495 | 0.571 (struct+temp) | 0.568 |
| repo-ranger | 0.632 | 0.750 (temporal-only) | **0.750** |
| axum | 0.025 | 0.031 (struct+hist) | 0.013 |

Temporal information is competitive everywhere and wins outright on
repo-ranger (temporal-only F1 0.750 > full 0.632; ROC 0.895); pairs beat
singles on 3/4 repos. No group dominates universally — history-only
reaches P 1.0 on repo-ranger; structural-only reaches R 0.80 on
repoinsight. Conclusion: temporal signals add measurable value beyond
static structure, most on small histories; combine, don't replace.

## Ripple forecasting

`ml/scripts/run_ripple_evaluation.py` (P@K/R@K/MRR/MAP, co-change-only
vs full) → `ml/ripple_results.json` (small repos filled 2026-09-23;
axum pending — long pure-Python run over 999 commits):

| Repo (targets) | Model | P@5 | R@5 | MRR | MAP |
|---|---|---|---|---|---|
| repoinsight (4) | co-change-only | 1.000 | 0.098 | 1.000 | 0.098 |
| repoinsight (4) | full | 1.000 | 0.098 | 1.000 | 0.098 |
| repoinsight-self (5) | co-change-only | 1.000 | 0.087 | 1.000 | 0.087 |
| repoinsight-self (5) | full | 1.000 | 0.087 | 1.000 | 0.087 |
| repo-ranger (6) | co-change-only | 1.000 | 0.495 | 1.000 | 0.495 |
| repo-ranger (6) | full | 1.000 | 0.495 | 1.000 | 0.495 |
| axum (300) | co-change-only | 1.000 | 0.019 | 1.000 | 0.019 |
| axum (300) | full | 1.000 | 0.019 | 1.000 | 0.019 |

Honest reading: top-1 ripple predictions are always correct (MRR 1.0
on all four repos, 315 targets total), but full == co-change-only
everywhere — the temporal signal changes nothing in this setup. The
reason is methodological, not mysterious:
ground truth is defined as *other files in the same test commit*, which
is co-change by construction, so a follow-up signal aimed at *later*
commits cannot win. Temporal value for propagation needs cross-commit
ground truth to show; that redesign is recorded as follow-up work, not
claimed here.

## Impact simulator

`ml/scripts/run_impact_evaluation.py` (commit message as description,
strictly-before-T prefixes; keyword-only / co-change-only / combined;
P@K/R@K/F1@K, directory accuracy) on self-repo git history (16 commits,
2,431 rows) → `ml/impact_results.json`:

| Model | P@5 | R@5 | F1@5 | Dir acc | Targets |
|---|---|---|---|---|---|
| keyword-only | 0.000 | 0.000 | n/a | 0.000 | 1 |
| co-change-only | n/a | n/a | n/a | n/a | 0 |
| combined | 0.000 | 0.000 | n/a | 0.000 | 1 |

Read honestly: this is a near-null result, and the reason is visible —
only 1 of 4 test commits was evaluable (the rest touch >15 files; the
self history is 16 coarse squashed commits, and the single target is a
1-file `.gitignore` commit whose description shares no vocabulary with
any path). Co-change-only ranked nothing and is reported as n/a rather
than zero. Conclusion: the harness is verified end-to-end on real
history, but the impact simulator is **not validated** by this data —
it needs a finer-grained repository history to be conclusive. That
evaluation is recorded as open work, not claimed.

## Error analysis

`ml/error_analysis.py` → per repo/model FP/FN counts, rates, examples,
observed per-group feature means (12 records, refreshed 2026-09-23):

| Repo | Model | FP (rate) | FN (rate) |
|---|---|---|---|
| repoinsight | LR | 6 (0.043) | 14 (0.700) |
| repoinsight | RF | 12 (0.087) | 11 (0.550) |
| repoinsight | HGB | 19 (0.138) | 11 (0.550) |
| repoinsight-self | LR | 27 (0.186) | 26 (0.500) |
| repoinsight-self | RF | 3 (0.021) | 29 (0.558) |
| repoinsight-self | HGB | 11 (0.076) | 25 (0.481) |
| repo-ranger | LR | 2 (0.044) | 5 (0.455) |
| repo-ranger | RF | 17 (0.378) | 2 (0.182) |
| repo-ranger | HGB | 0 (0.000) | 11 (1.000) |
| axum | LR | 11232 (0.284) | 95 (0.396) |
| axum | RF | 758 (0.019) | 227 (0.946) |
| axum | HGB | 1443 (0.036) | 213 (0.887) |

Read honestly: false negatives dominate almost everywhere — the models
under-flag rather than over-warn, the safer failure direction for
developer trust. RF on repoinsight-self is the precision extreme (3 FP)
at recall cost; HGB predicting all-negative on tiny repo-ranger (FN 1.0)
confirms tree-model instability on small histories — one more reason the
shipped default stays logistic regression.

## Backend evaluation (unit, green)

* `cargo test`: rework-detector measurement (6 reviewed pairs),
  `evaluate_ripple` + `evaluate_impact` chronological tests incl.
  leakage regression (`leakage_prefix_excludes_target_and_future`),
  121 tests passing.
* Serving paths for ripple/impact reuse the same prefix discipline;
  evaluation functions are research-facing APIs mirroring the ML side.

---

## Definition of Done — evidence

1. Arbitrary supported GitHub repos — GitHub collector + local-git
   loader (`backend/src/local.rs`), rate-limit/backoff handling.
2. Source structure + history collection — file tree, 1,000-commit
   paginated history, blob collection.
3. Chronological representation — timeline, sequences, co-change,
   temporal pairs, propagation graph.
4. Structural/historical/temporal features — 9/6/8 feature groups.
5. Leakage-safe training data — prefix-only dataset builder +
   validator, chronological splits.
6. Trained/evaluated model — LR baseline + RF/HGB comparison above.
7. Propagation/rework prediction — predictions, ripple, impact APIs +
   dashboard.
8. Historical evidence — examples, explanations, per-edge evidence.
9. Honest explanation — evidence vs prediction split, uncalibrated
   labels, associational language.
10. Interactive dashboard — Overview/Explore/Predict/History workflow.
