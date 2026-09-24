use serde::Serialize;
use std::collections::HashMap;

use super::history::relevant_files;
use crate::analysis::models::{PropagationAnalysis, TemporalAnalysis};
use crate::github::commits::Commit;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TemporalFeatures {
    pub file_path: String,
    pub recent_change_sequence_count: usize,
    pub previous_files_changed_count: usize,
    pub change_window_count: usize,
    pub avg_change_delay_seconds: Option<i64>,
    pub change_order_count: usize,
    pub propagation_frequency: usize,
    pub followup_frequency: usize,
    pub historical_rework_frequency: usize,
}

pub fn build_temporal_features(
    temporal: &TemporalAnalysis,
    propagation: &PropagationAnalysis,
    followup_frequency: &HashMap<String, usize>,
    rework_frequency: &HashMap<String, usize>,
    file_times: &HashMap<String, Vec<i64>>,
    ref_ts: i64,
    window_secs: i64,
) -> Vec<TemporalFeatures> {
    let mut features: HashMap<String, TemporalFeatures> = HashMap::new();

    for pair in &temporal.pairs {
        for file in [&pair.source, &pair.target] {
            features
                .entry(file.clone())
                .or_insert_with(|| empty_features(file));
        }
        features
            .get_mut(&pair.source)
            .expect("inserted above")
            .recent_change_sequence_count += pair.occurrences;
    }

    for edge in &propagation.edges {
        for file in [&edge.source, &edge.target] {
            features
                .entry(file.clone())
                .or_insert_with(|| empty_features(file))
                .propagation_frequency += edge.strength;
        }
    }

    for (file, count) in followup_frequency {
        features
            .entry(file.clone())
            .or_insert_with(|| empty_features(file))
            .followup_frequency += *count;
    }

    for (file, count) in rework_frequency {
        features
            .entry(file.clone())
            .or_insert_with(|| empty_features(file))
            .historical_rework_frequency += *count;
    }

    let others = file_times.len();
    // Distinct event timestamps across the whole prefix, computed once and
    // reused for every file's change_order_count below.
    let mut ordered_times: Vec<i64> = file_times.values().flatten().copied().collect();
    ordered_times.sort_unstable();
    ordered_times.dedup();
    for entry in features.values_mut() {
        let times = file_times.get(&entry.file_path);

        entry.previous_files_changed_count = others.saturating_sub(usize::from(times.is_some()));
        entry.change_window_count = times
            .map(|stamps| {
                stamps
                    .iter()
                    .filter(|ts| ref_ts - **ts >= 0 && ref_ts - **ts <= window_secs)
                    .count()
            })
            .unwrap_or(0);
        entry.avg_change_delay_seconds = times.and_then(|stamps| mean_gap(stamps));
        entry.change_order_count = times
            .and_then(|stamps| stamps.iter().max())
            .map(|latest| ordered_times.partition_point(|ts| ts <= latest))
            .unwrap_or(0);
    }

    let mut result: Vec<_> = features.into_values().collect();
    result.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    result
}

fn mean_gap(stamps: &[i64]) -> Option<i64> {
    if stamps.len() < 2 {
        return None;
    }
    let total: i64 = stamps.windows(2).map(|pair| pair[1] - pair[0]).sum();
    Some(total / (stamps.len() - 1) as i64)
}

/// Running equivalent of the per-target `file_times` map built in the
/// dataset loop: per-file timestamp vectors over the strictly-historical
/// prefix.
///
/// A latest-only map would NOT suffice: downstream
/// [`build_temporal_features`] needs the full vectors (window counts,
/// consecutive-gap means, per-file maxima) with duplicates preserved.
/// Advance-only over the same grouped-timestamp ranges the dataset builder
/// uses (commits with `timestamp < cutoff`, never the target itself); pushes
/// arrive in chronological order, so each vector is already ascending exactly
/// as the oracle's explicit sort produces. Read the state with
/// [`file_times`](Self::file_times) and lend it directly to
/// [`build_temporal_features`] — no per-target rebuild, collect, or sort.
#[derive(Debug, Default)]
pub struct FileTimesAccumulator {
    times: HashMap<String, Vec<i64>>,
}

impl FileTimesAccumulator {
    pub fn advance(&mut self, commits: &[Commit]) {
        for commit in commits {
            let Some(timestamp) = commit.timestamp else {
                continue;
            };

            for file in relevant_files(&commit.files) {
                self.times.entry(file.filename).or_default().push(timestamp);
            }
        }
    }

    pub fn file_times(&self) -> &HashMap<String, Vec<i64>> {
        &self.times
    }
}

impl TemporalFeatures {
    #[allow(
        dead_code,
        reason = "Phase 5 library API: used by dataset construction and unit tests"
    )]
    pub fn empty(file_path: &str) -> Self {
        empty_features(file_path)
    }
}

fn empty_features(file_path: &str) -> TemporalFeatures {
    TemporalFeatures {
        file_path: file_path.to_owned(),
        recent_change_sequence_count: 0,
        previous_files_changed_count: 0,
        change_window_count: 0,
        avg_change_delay_seconds: None,
        change_order_count: 0,
        propagation_frequency: 0,
        followup_frequency: 0,
        historical_rework_frequency: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::models::{PropagationEdge, TemporalPair};

    fn pair(source: &str, target: &str, occurrences: usize, avg: i64) -> TemporalPair {
        TemporalPair {
            source: source.to_owned(),
            target: target.to_owned(),
            occurrences,
            avg_delay_seconds: avg,
        }
    }

    fn times(pairs: &[(&str, Vec<i64>)]) -> HashMap<String, Vec<i64>> {
        pairs
            .iter()
            .map(|(file, stamps)| ((*file).to_owned(), stamps.clone()))
            .collect()
    }

    #[test]
    fn aggregates_sequence_and_propagation_counts() {
        let temporal = TemporalAnalysis {
            pairs: vec![pair("a.rs", "b.rs", 3, 100), pair("a.rs", "c.rs", 2, 50)],
        };
        let propagation = PropagationAnalysis {
            edges: vec![PropagationEdge {
                source: "a.rs".to_owned(),
                target: "b.rs".to_owned(),
                dependency: false,
                temporal: true,
                cochange: false,
                strength: 4,
            }],
        };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::new(),
            &HashMap::new(),
            &times(&[("a.rs", vec![100, 200, 300]), ("b.rs", vec![150])]),
            300,
            7 * 24 * 60 * 60,
        );

        let file = result
            .iter()
            .find(|entry| entry.file_path == "a.rs")
            .unwrap();
        assert_eq!(file.recent_change_sequence_count, 5);
        assert_eq!(file.propagation_frequency, 4);
        assert_eq!(file.previous_files_changed_count, 1);
        assert_eq!(file.change_window_count, 3);
        assert_eq!(file.avg_change_delay_seconds, Some(100));
        assert_eq!(file.change_order_count, 4);
    }

    #[test]
    fn avg_delay_needs_two_changes() {
        let temporal = TemporalAnalysis { pairs: vec![] };
        let propagation = PropagationAnalysis { edges: vec![] };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::from([("solo.rs".to_owned(), 2)]),
            &HashMap::from([("solo.rs".to_owned(), 1)]),
            &times(&[("solo.rs", vec![500])]),
            500,
            60,
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].avg_change_delay_seconds, None);
        assert_eq!(result[0].followup_frequency, 2);
        assert_eq!(result[0].historical_rework_frequency, 1);
        assert_eq!(result[0].change_window_count, 1);
    }

    #[test]
    fn unknown_files_get_zero_time_fields() {
        let temporal = TemporalAnalysis {
            pairs: vec![pair("$dep", "a.rs", 1, 10)],
        };
        let propagation = PropagationAnalysis { edges: vec![] };

        let result = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::new(),
            &HashMap::new(),
            &times(&[("a.rs", vec![10])]),
            10,
            60,
        );

        let file = result
            .iter()
            .find(|entry| entry.file_path == "$dep")
            .unwrap();
        assert_eq!(file.avg_change_delay_seconds, None);
        assert_eq!(file.change_order_count, 0);
        assert_eq!(file.previous_files_changed_count, 1);
    }

    fn stamp_commit(sha: &str, timestamp: Option<i64>, files: Vec<(&str, u64, u64)>) -> Commit {
        use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};

        Commit {
            sha: sha.to_owned(),
            message: sha.to_owned(),
            author: None,
            timestamp,
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files: files
                .into_iter()
                .map(|(filename, additions, deletions)| ChangedFile {
                    filename: filename.to_owned(),
                    additions,
                    deletions,
                    changes: additions + deletions,
                    status: FileChangeStatus::Modified,
                })
                .collect(),
        }
    }

    /// Pre-Step-7 per-target construction, kept verbatim as the oracle:
    /// relevant files of each prefix commit push their timestamp, then every
    /// vector is sorted.
    fn oracle_file_times(commits: &[Commit]) -> HashMap<String, Vec<i64>> {
        let mut file_times: HashMap<String, Vec<i64>> = HashMap::new();
        for commit in commits {
            let Some(timestamp) = commit.timestamp else {
                continue;
            };
            for file in relevant_files(&commit.files) {
                file_times.entry(file.filename).or_default().push(timestamp);
            }
        }
        for stamps in file_times.values_mut() {
            stamps.sort_unstable();
        }
        file_times
    }

    #[test]
    fn incremental_file_times_matches_oracle_at_every_cutoff() {
        // Covers: repeats, multiple files, equal timestamps (same file twice
        // at one ts via two commits), same-commit duplicates, ghosts,
        // ignored paths, sparse single-file commits, future commits, and an
        // empty prefix. The test compares full maps: keys plus exact ordered
        // timestamp sequences, duplicates included.
        let commits = vec![
            stamp_commit("c0", Some(0), vec![("a.rs", 1, 0), ("b.rs", 1, 0)]),
            stamp_commit("ghost", None, vec![("a.rs", 5, 5), ("x.rs", 1, 0)]),
            stamp_commit("c1", Some(100), vec![("a.rs", 1, 0)]),
            stamp_commit("c1b", Some(100), vec![("a.rs", 2, 0), ("b.rs", 1, 0)]),
            stamp_commit("dup", Some(200), vec![("d.rs", 1, 0), ("d.rs", 2, 0)]),
            stamp_commit(
                "ign",
                Some(300),
                vec![("target/w.rs", 9, 9), ("e.rs", 1, 0)],
            ),
            stamp_commit("c2", Some(400), vec![("a.rs", 1, 0), ("e.rs", 1, 0)]),
            stamp_commit("future", Some(9999), vec![("z.rs", 1, 0)]),
        ];

        // Same ordering as the dataset builder: timestamped only,
        // sorted by (timestamp, sha).
        let mut ordered: Vec<Commit> = commits
            .iter()
            .filter(|commit| commit.timestamp.is_some())
            .cloned()
            .collect();
        ordered.sort_by(|a, b| {
            (a.timestamp, &a.sha)
                .partial_cmp(&(b.timestamp, &b.sha))
                .expect("timestamps present")
        });

        // Empty prefix first: both states must be empty.
        let accumulator = FileTimesAccumulator::default();
        assert_eq!(accumulator.file_times(), &oracle_file_times(&[]));

        let mut accumulator = FileTimesAccumulator::default();
        let mut advanced = 0usize;
        let mut cutoffs: Vec<i64> = ordered
            .iter()
            .filter_map(|commit| commit.timestamp)
            .collect();
        cutoffs.sort_unstable();
        cutoffs.dedup();

        // Fixed downstream inputs: only `file_times` varies, so any output
        // difference would come from the accumulator state itself.
        let temporal = TemporalAnalysis {
            pairs: vec![pair("a.rs", "b.rs", 2, 50)],
        };
        let propagation = PropagationAnalysis { edges: vec![] };

        for cutoff in cutoffs {
            let prefix_len =
                ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < cutoff);
            accumulator.advance(&ordered[advanced..prefix_len]);
            advanced = prefix_len;
            let prefix = &ordered[..prefix_len];

            let oracle = oracle_file_times(prefix);
            assert_eq!(accumulator.file_times(), &oracle, "cutoff {cutoff}");

            for current in [cutoff, cutoff + 50] {
                assert_eq!(
                    build_temporal_features(
                        &temporal,
                        &propagation,
                        &HashMap::from([("a.rs".to_owned(), 1)]),
                        &HashMap::new(),
                        accumulator.file_times(),
                        current,
                        150,
                    ),
                    build_temporal_features(
                        &temporal,
                        &propagation,
                        &HashMap::from([("a.rs".to_owned(), 1)]),
                        &HashMap::new(),
                        &oracle,
                        current,
                        150,
                    ),
                    "consumer output at ref_ts {current} (cutoff {cutoff})"
                );
            }
        }

        // Spot-check the reference values themselves (not just equality):
        // same-commit duplicates and equal-timestamp commits both preserve
        // repeated stamps in chronological order; ghosts/ignored never appear.
        let full = oracle_file_times(
            &ordered
                [..ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < 9999)],
        );
        assert_eq!(full.get("a.rs"), Some(&vec![0, 100, 100, 400]));
        assert_eq!(full.get("d.rs"), Some(&vec![200, 200]));
        assert_eq!(full.get("b.rs"), Some(&vec![0, 100]));
        assert_eq!(full.get("e.rs"), Some(&vec![300, 400]));
        assert!(!full.contains_key("x.rs"));
        assert!(!full.keys().any(|key| key.contains("target/")));
        // a.rs at ref_ts 250 with window 150 sees stamps {100, 100}, while the
        // delay mean runs over the full duplicate-aware vector [0,100,100,400]:
        // gaps (100, 0, 300) sum to 400 over 3 intervals.
        let features = build_temporal_features(
            &temporal,
            &propagation,
            &HashMap::new(),
            &HashMap::new(),
            &full,
            250,
            150,
        );
        let file = features
            .iter()
            .find(|entry| entry.file_path == "a.rs")
            .unwrap();
        assert_eq!(file.change_window_count, 2);
        assert_eq!(file.avg_change_delay_seconds, Some(133));
        assert_eq!(file.previous_files_changed_count, 3);
    }

    #[test]
    fn incremental_file_times_ignores_ghosts_when_advanced_directly() {
        let ghost = stamp_commit("ghost", None, vec![("a.rs", 1, 0)]);
        let mut accumulator = FileTimesAccumulator::default();
        accumulator.advance(std::slice::from_ref(&ghost));
        assert!(accumulator.file_times().is_empty());
    }
}
