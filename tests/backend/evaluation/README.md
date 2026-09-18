# Rework Detector Evaluation (Phase 4.4)

Measures the existing `detect_rework` detector
(`backend/src/analysis/propagation_history.rs`, rules unchanged:
`repeated-touch`, `fix-message`, `revert`, `related-fix`) against
independently reviewed commit pairs from this repository's own history.

## Dataset

`rework_labels.csv` — the 6 adjacent commit pairs of this repository's
first 7 commits (2026-09-04 → 2026-09-17):

| previous | current | message of `current` | actual |
|---|---|---|---|
| 147c64a | 39cdeea | Initial project setup (.gitignore tweak) | false — setup iteration, not a correction |
| 39cdeea | 852b566 | Complete Phase 1 project foundation | false — new frontend/docs work, disjoint files |
| 852b566 | 2befeff | feat: complete GitHub repository collector | false — new backend feature, disjoint files |
| 2befeff | c0a89c1 | chore: remove Rust build artifacts | false — artifact cleanup, no source overlap |
| c0a89c1 | 302fc08 | feat: complete repository analysis phase | false — new analysis feature (previous touched only ignored paths) |
| 302fc08 | 5525a4d | Phase 4: temporal change propagation… | false — planned Phase 4 feature work extending existing files, not a correction |

Labels come from manual review of `git show --name-status` output and
commit messages. Per the review rule, setup, feature development,
documentation, and build-artifact cleanup are `false`; only a genuine
correction of the earlier change would be `true`. No detector output was
used to assign labels.

## How predictions are generated

`run_evaluation()` (`backend/src/analysis/rework_evaluation.rs`):

1. Rebuilds the 7 commits from a fixture transcribed from
   `git show --name-status` (real SHAs, timestamps, messages, file lists;
   `backend/target/*` paths omitted because the detector filters them;
   per-file stats do not affect detection).
2. Computes real co-change pairs from that history. The static dependency
   list is empty (source blobs unavailable offline); this cannot affect the
   outcome here because no message is fix-like and dependency links only
   feed the `related-fix` rule.
3. Runs the **unmodified** `detect_rework` with default config (14-day window).
4. Marks a labeled pair predicted-true when the detector emits at least one
   event with exactly that `(initial_sha, rework_sha)`.

## Formulas

- TP = predicted true AND actual true
- FP = predicted true AND actual false
- TN = predicted false AND actual false
- FN = predicted false AND actual true
- precision = TP / (TP + FP)
- recall = TP / (TP + FN)
- false_positive_rate = FP / (FP + TN)

Ratios with a zero denominator are reported as null (no value), never as zero.

## Measured result

| TP | FP | TN | FN | precision | recall | FPR |
|---|---|---|---|---|---|---|
0 | 2 | 4 | 0 | 0.0 | null (no actual rework) | 0.333 (33.3% on n=6)

Both false positives are `repeated-touch`: the `.gitignore` setup tweak
(147c64a → 39cdeea) and the Phase 4 feature commit re-touching Phase 3
files (302fc08 → 5525a4d). This evaluation shows that the repeated-touch heuristic can classify
iterative feature work as candidate rework. This is an expected limitation
of a history-only repeat-touch heuristic, and why rework is always reported
as candidate with its rule, and why rework is always reported as
*candidate* with its rule.

## Limitations

- n = 6 pairs from a single young repository; this is a smoke measurement,
  not a statistically meaningful benchmark.
- Zero actual positives: recall is undefined and precision rests on 2
  predicted positives. No conclusion about recall can be drawn.
- Adjacent pairs only; non-adjacent in-window pairs are not labeled.
- Static dependency edges are unavailable in the offline fixture.

Do NOT fabricate additional examples to inflate the sample. Re-measure on a
larger, multi-repository history before using these numbers for model
selection. Served live at `GET /api/evaluation/rework`.
