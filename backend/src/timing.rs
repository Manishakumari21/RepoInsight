//! Lightweight end-to-end performance instrumentation (Phase 9).
//!
//! [`AnalysisTimings`] records per-stage wall-clock times for the local
//! repository pipeline. Stage fields are `None` until that stage
//! completes, so a failed run can never report fabricated timings.
//! Counts are plain integers (`0` when nothing was observed).
//!
//! Memory is intentionally NOT measured here: reliable peak-RSS
//! measurement is platform-dependent (libc `getrusage`, `/proc`, …).
//! Use external profiling instead, e.g.
//! `/usr/bin/time -v <backend>` or the `scripts/measure_local_analysis.py`
//! diagnostic wrapper.

use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Per-stage timings (milliseconds) and observed counts for one local
/// repository analysis run.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisTimings {
    /// Path validation + canonicalization (`validate_local_path`).
    pub repository_validation_ms: Option<u64>,
    /// `git ls-tree` + entry parsing.
    pub tree_loading_ms: Option<u64>,
    /// `git log` + commit parsing.
    pub commit_loading_ms: Option<u64>,
    /// Working-tree source file reads (blocking I/O, off async workers).
    pub source_loading_ms: Option<u64>,
    /// Tree-sitter parsing, complexity, dependencies, history, temporal
    /// analysis, scoring (`analyze_loaded_data`, CPU-bound).
    pub structural_analysis_ms: Option<u64>,
    /// Leakage-safe dataset row construction (CPU-bound).
    pub dataset_construction_ms: Option<u64>,
    /// End-to-end wall clock for the timed run.
    pub total_ms: Option<u64>,

    /// Blob entries in the Git tree.
    pub file_count: usize,
    /// Commits loaded (capped by the loader).
    pub commit_count: usize,
    /// Source files actually read from the working tree.
    pub source_file_count: usize,
    /// Dataset rows constructed (0 when the run skips that stage).
    pub dataset_row_count: usize,
}

/// Milliseconds elapsed since `start`, saturating on overflow.
pub fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_timings_report_nothing_observed() {
        let timings = AnalysisTimings::default();
        assert_eq!(timings.repository_validation_ms, None);
        assert_eq!(timings.tree_loading_ms, None);
        assert_eq!(timings.commit_loading_ms, None);
        assert_eq!(timings.source_loading_ms, None);
        assert_eq!(timings.structural_analysis_ms, None);
        assert_eq!(timings.dataset_construction_ms, None);
        assert_eq!(timings.total_ms, None);
        assert_eq!(timings.file_count, 0);
        assert_eq!(timings.commit_count, 0);
        assert_eq!(timings.source_file_count, 0);
        assert_eq!(timings.dataset_row_count, 0);
    }

    #[test]
    fn timings_round_trip_through_json() {
        let timings = AnalysisTimings {
            repository_validation_ms: Some(1),
            tree_loading_ms: Some(12),
            commit_loading_ms: Some(230),
            source_loading_ms: Some(1500),
            structural_analysis_ms: Some(800),
            dataset_construction_ms: Some(90),
            total_ms: Some(2640),
            file_count: 8984,
            commit_count: 1000,
            source_file_count: 7204,
            dataset_row_count: 209477,
        };
        let json = serde_json::to_string(&timings).expect("serializes");
        let back: AnalysisTimings = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(timings, back);
    }

    #[test]
    fn partial_timings_stay_honest() {
        // A run that fails after tree loading must not invent later stages.
        let mut timings = AnalysisTimings::default();
        timings.repository_validation_ms = Some(0);
        timings.tree_loading_ms = Some(5);
        timings.file_count = 42;
        assert_eq!(timings.commit_loading_ms, None);
        assert_eq!(timings.structural_analysis_ms, None);
        assert_eq!(timings.dataset_construction_ms, None);
        assert_eq!(timings.total_ms, None);
        assert_eq!(timings.commit_count, 0);
    }

    #[test]
    fn elapsed_ms_is_nonnegative_and_bounded() {
        let start = Instant::now();
        let ms = elapsed_ms(start);
        assert!(ms <= 60_000, "unexpected elapsed: {ms}ms");
    }
}
