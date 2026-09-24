#![allow(
    dead_code,
    reason = "Phase 5 library API in a binary crate: exercised by unit tests, consumed by Phase 6/export tooling, not by the serving binary"
)]

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::io;

use crate::analysis::{
    dependencies::DependencyEdge,
    features::StructuralFeatures,
    historical_features::{self, HistoricalFeatures},
    history,
    models::{FollowUpAnalysis, PropagationAnalysis, ReworkAnalysis, TemporalAnalysis},
    propagation_history::{self, PropagationConfig},
    temporal_features::{self, TemporalFeatures},
};
use crate::github::commits::Commit;

pub const DEFAULT_TRAIN_RATIO: f64 = 0.7;
pub const DEFAULT_VALIDATION_RATIO: f64 = 0.15;
pub const DEFAULT_RECENT_WINDOW_SECS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Copy)]
pub struct DatasetConfig {
    pub train_ratio: f64,
    pub validation_ratio: f64,
    pub recent_window_secs: i64,
    pub propagation: PropagationConfig,
}

impl Default for DatasetConfig {
    fn default() -> Self {
        Self {
            train_ratio: DEFAULT_TRAIN_RATIO,
            validation_ratio: DEFAULT_VALIDATION_RATIO,
            recent_window_secs: DEFAULT_RECENT_WINDOW_SECS,
            propagation: PropagationConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DatasetRow {
    pub commit_sha: String,
    pub timestamp: i64,
    pub file_path: String,
    pub structural: StructuralFeatures,
    pub historical: HistoricalFeatures,
    pub temporal: TemporalFeatures,
    pub label: u8,
}

pub fn build_dataset(
    commits: &[Commit],
    structural: &HashMap<String, StructuralFeatures>,
    dependencies: &[DependencyEdge],
    config: &DatasetConfig,
) -> Vec<DatasetRow> {
    let ordered = timestamped(commits);
    let mut rows = Vec::new();
    // Monotonic historical file set: before rows for timestamp T are
    // emitted, `seen` holds exactly the files touched by commits with
    // ts < T. Afterwards it is advanced through the whole T group at once,
    // so equal-timestamp commits never observe each other. `advanced` marks
    // how much of `ordered` has been folded into `seen`.
    let mut seen: HashSet<String> = HashSet::new();
    let mut advanced = 0usize;
    // Incremental twin of `build_historical_prefix`: advanced over the same
    // grouped cutoff ranges as `seen`, so it always represents exactly the
    // commits with ts < current. Materialized per target into the identical
    // output shape the downstream code already consumes.
    let mut history_accum = historical_features::HistoricalPrefixAccumulator::default();
    let mut history_advanced = 0usize;
    // Incremental twin of `history::temporal_analysis`: advanced over the same
    // grouped cutoff ranges as `seen`, so it always represents exactly the
    // commits with ts < current. Materialized per target into the identical
    // top-50 `TemporalAnalysis` the downstream code already consumes.
    let mut temporal_accum = history::TemporalPrefixAccumulator::default();
    let mut temporal_advanced = 0usize;
    // Incremental twin of `history::build_propagation`: static dependency
    // contributions folded once, co-change contributions folded per newly
    // eligible commit; the current top-50 temporal list is folded at
    // materialize time. The merged map stays full so late-rising pairs can
    // always enter the top 100.
    let mut propagation_accum = history::PropagationPrefixAccumulator::new(dependencies);
    let mut propagation_advanced = 0usize;
    // Incremental twin of the per-target `file_times` map: full per-file
    // timestamp vectors over exactly the commits with ts < current,
    // maintained in chronological push order. Lent by reference to the
    // temporal-feature builder, so no per-target rebuild or re-sort.
    let mut file_times_accum = temporal_features::FileTimesAccumulator::default();
    let mut file_times_advanced = 0usize;
    // Incremental twin of `detect_followups` / `detect_rework`: pair
    // discovery folded once per newly eligible commit; classification runs at
    // materialize time with the present prefix co-change map, exactly as the
    // oracles classify every pair with the full prefix map on each call.
    let mut followup_rework_accum =
        propagation_history::FollowupReworkPrefixAccumulator::new(dependencies, config.propagation);
    let mut followup_rework_advanced = 0usize;

    for (index, target) in ordered.iter().enumerate() {
        let current = target.timestamp.expect("timestamped above");
        // `ordered` is sorted by (timestamp, sha), so every commit with
        // ts < current precedes every commit with ts >= current: the prefix
        // is the contiguous slice `ordered[..k]`. This replaces the previous
        // filter-then-clone with a borrow; the selected set and order are
        // identical, including equal-timestamp exclusion (strict `<`) and
        // the absence of untimestamped commits (filtered by `timestamped`).
        let prefix_len = ordered[..index]
            .partition_point(|commit| commit.timestamp.expect("timestamped above") < current);
        // `current` is non-decreasing over targets, so `prefix_len` only
        // grows: each historical commit is folded into `seen` exactly once.
        for commit in &ordered[advanced..prefix_len] {
            seen.extend(
                history::relevant_files(&commit.files)
                    .into_iter()
                    .map(|file| file.filename),
            );
        }
        advanced = prefix_len;

        let changed: HashSet<String> = history::relevant_files(&target.files)
            .iter()
            .map(|file| file.filename.clone())
            .collect();

        let positives: Vec<&String> = changed
            .iter()
            .filter(|file| structural.contains_key(*file))
            .collect();
        let mut negatives: Vec<&String> = structural
            .keys()
            .filter(|file| seen.contains(*file) && !changed.contains(*file))
            .collect();
        negatives.sort();

        if positives.is_empty() && negatives.is_empty() {
            continue;
        }

        let historical = {
            history_accum.advance(&ordered[history_advanced..prefix_len]);
            history_advanced = prefix_len;
            history_accum.materialize(current, config.recent_window_secs)
        };
        let temporal = {
            temporal_accum.advance(&ordered[temporal_advanced..prefix_len]);
            temporal_advanced = prefix_len;
            temporal_accum.materialize()
        };
        let propagation = {
            propagation_accum.advance(&ordered[propagation_advanced..prefix_len]);
            propagation_advanced = prefix_len;
            propagation_accum.materialize(&temporal)
        };
        file_times_accum.advance(&ordered[file_times_advanced..prefix_len]);
        file_times_advanced = prefix_len;
        let (followups, rework) = {
            followup_rework_accum.advance(&ordered[followup_rework_advanced..prefix_len]);
            followup_rework_advanced = prefix_len;
            followup_rework_accum.materialize(history_accum.cochange_pairs())
        };
        let temporal = temporal_for_prefix(
            &temporal,
            &propagation,
            file_times_accum.file_times(),
            &followups,
            &rework,
            config,
            current,
        );

        let mut candidates: Vec<(&String, u8)> = positives
            .iter()
            .map(|file| (*file, 1))
            .chain(negatives.iter().map(|file| (*file, 0)))
            .collect();
        candidates.sort_by(|a, b| a.0.cmp(b.0));

        for (file, label) in candidates {
            let Some(structure) = structural.get(file) else {
                continue;
            };
            let history = historical
                .get(file)
                .cloned()
                .unwrap_or_else(|| HistoricalFeatures::empty(file));
            let temporal = temporal
                .get(file)
                .cloned()
                .unwrap_or_else(|| TemporalFeatures::empty(file));
            rows.push(DatasetRow {
                commit_sha: target.sha.clone(),
                timestamp: current,
                file_path: file.clone(),
                structural: structure.clone(),
                historical: history,
                temporal: temporal,
                label,
            });
        }
    }

    rows
}

fn timestamped(commits: &[Commit]) -> Vec<Commit> {
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
    ordered
}

fn files_in(commits: &[Commit]) -> HashSet<String> {
    commits
        .iter()
        .flat_map(|commit| history::relevant_files(&commit.files))
        .map(|file| file.filename)
        .collect()
}

fn temporal_for_prefix(
    temporal: &TemporalAnalysis,
    propagation: &PropagationAnalysis,
    file_times: &HashMap<String, Vec<i64>>,
    followups: &FollowUpAnalysis,
    rework: &ReworkAnalysis,
    config: &DatasetConfig,
    current: i64,
) -> HashMap<String, TemporalFeatures> {
    let mut followup_frequency: HashMap<String, usize> = HashMap::new();
    for followup in &followups.followups {
        let files: HashSet<&String> = followup
            .source_files
            .iter()
            .chain(&followup.followup_files)
            .collect();
        for file in files {
            *followup_frequency.entry(file.clone()).or_insert(0) += 1;
        }
    }

    let mut rework_frequency: HashMap<String, usize> = HashMap::new();
    for event in &rework.events {
        *rework_frequency.entry(event.file.clone()).or_insert(0) += 1;
    }

    temporal_features::build_temporal_features(
        temporal,
        propagation,
        &followup_frequency,
        &rework_frequency,
        file_times,
        current,
        config.propagation.sequence_window_secs,
    )
    .into_iter()
    .map(|entry| (entry.file_path.clone(), entry))
    .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RemovedRow {
    pub index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct DatasetStats {
    pub total_examples: usize,
    pub positives: usize,
    pub negatives: usize,
    pub positive_ratio: f64,
    pub negative_ratio: f64,
    pub commits: usize,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ValidationReport {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub removed: Vec<RemovedRow>,
    pub stats: DatasetStats,
}

pub fn validate_dataset(rows: &[DatasetRow], commits: &[Commit]) -> ValidationReport {
    let mut changed_by_commit: HashMap<&str, HashSet<String>> = HashMap::new();
    let mut known_before: HashMap<&str, HashSet<String>> = HashMap::new();
    let ordered = timestamped(commits);

    for (index, target) in ordered.iter().enumerate() {
        let current = target.timestamp.expect("timestamped above");
        let changed: HashSet<String> = history::relevant_files(&target.files)
            .iter()
            .map(|file| file.filename.clone())
            .collect();
        changed_by_commit.insert(target.sha.as_str(), changed);
        let mut seen: HashSet<String> = HashSet::new();
        for prior in &ordered[..index] {
            if prior.timestamp.expect("timestamped above") < current {
                seen.extend(
                    history::relevant_files(&prior.files)
                        .iter()
                        .map(|file| file.filename.clone()),
                );
            }
        }
        known_before.insert(target.sha.as_str(), seen);
    }

    let mut seen_rows: HashSet<(&str, &str)> = HashSet::new();
    let mut removed = Vec::new();
    let mut valid = Vec::new();

    for (index, row) in rows.iter().enumerate() {
        let reason = if row.file_path.is_empty() || row.commit_sha.is_empty() {
            Some("empty file path or commit SHA")
        } else if row.label > 1 {
            Some("label is not binary")
        } else if row.timestamp <= 0 {
            Some("invalid timestamp")
        } else if !seen_rows.insert((row.commit_sha.as_str(), row.file_path.as_str())) {
            Some("duplicate commit/file row")
        } else {
            match changed_by_commit.get(row.commit_sha.as_str()) {
                None => Some("target commit unknown"),
                Some(changed) => {
                    if row.label == 1 && !changed.contains(row.file_path.as_str()) {
                        Some("positive file absent from target commit")
                    } else if row.label == 0
                        && !known_before
                            .get(row.commit_sha.as_str())
                            .is_some_and(|seen| seen.contains(row.file_path.as_str()))
                    {
                        Some("impossible negative: file never existed before target")
                    } else {
                        None
                    }
                }
            }
        };

        match reason {
            Some(reason) => removed.push(RemovedRow {
                index,
                reason: reason.to_owned(),
            }),
            None => valid.push(row),
        }
    }

    ValidationReport {
        total_rows: rows.len(),
        valid_rows: valid.len(),
        removed,
        stats: dataset_stats(&valid),
    }
}

pub fn dataset_stats(rows: &[&DatasetRow]) -> DatasetStats {
    let total = rows.len();
    let positives = rows.iter().filter(|row| row.label == 1).count();
    let negatives = total.saturating_sub(positives);
    let commits: HashSet<&str> = rows.iter().map(|row| row.commit_sha.as_str()).collect();
    let files: HashSet<&str> = rows.iter().map(|row| row.file_path.as_str()).collect();

    DatasetStats {
        total_examples: total,
        positives,
        negatives,
        positive_ratio: if total == 0 {
            0.0
        } else {
            positives as f64 / total as f64
        },
        negative_ratio: if total == 0 {
            0.0
        } else {
            negatives as f64 / total as f64
        },
        commits: commits.len(),
        files: files.len(),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatasetSplit {
    pub train: Vec<DatasetRow>,
    pub validation: Vec<DatasetRow>,
    pub test: Vec<DatasetRow>,
}

pub fn split_chronologically(rows: Vec<DatasetRow>, config: &DatasetConfig) -> DatasetSplit {
    let mut targets: Vec<(i64, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for row in &rows {
        if seen.insert(row.commit_sha.clone()) {
            targets.push((row.timestamp, row.commit_sha.clone()));
        }
    }
    targets.sort();

    let mut split = DatasetSplit::default();
    if targets.len() < 3 {
        split.train = rows;
        return split;
    }

    let train_end = (targets.len() as f64 * config.train_ratio).floor() as usize;
    let validation_end =
        train_end + (targets.len() as f64 * config.validation_ratio).floor() as usize;
    let train: HashSet<&str> = targets[..train_end.min(targets.len())]
        .iter()
        .map(|(_, sha)| sha.as_str())
        .collect();
    let validation: HashSet<&str> = targets
        [train_end.min(targets.len())..validation_end.min(targets.len())]
        .iter()
        .map(|(_, sha)| sha.as_str())
        .collect();

    for row in rows {
        if train.contains(row.commit_sha.as_str()) {
            split.train.push(row);
        } else if validation.contains(row.commit_sha.as_str()) {
            split.validation.push(row);
        } else {
            split.test.push(row);
        }
    }

    split
}

pub fn export_jsonl(rows: &[DatasetRow], path: &str) -> io::Result<usize> {
    let file = std::fs::File::create(path)?;
    let mut writer = io::BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut writer, row)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        io::Write::write_all(&mut writer, b"\n")?;
    }
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{complexity::ComplexityResult, dependencies::DependencyMetrics};
    use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};

    fn file(name: &str, additions: u64, deletions: u64) -> ChangedFile {
        ChangedFile {
            filename: name.to_owned(),
            additions,
            deletions,
            changes: additions + deletions,
            status: FileChangeStatus::Modified,
        }
    }

    fn commit(sha: &str, timestamp: Option<i64>, files: Vec<ChangedFile>) -> Commit {
        Commit {
            sha: sha.to_owned(),
            message: sha.to_owned(),
            author: None,
            timestamp,
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files,
        }
    }

    fn structural(path: &str) -> StructuralFeatures {
        StructuralFeatures {
            file_path: path.to_owned(),
            file_size_bytes: 100,
            lines_of_code: 10,
            function_count: 1,
            cyclomatic_complexity: 2,
            max_nesting_depth: 1,
            incoming_dependencies: 0,
            outgoing_dependencies: 0,
            coupling: 0,
            num_dependents: 0,
        }
    }

    fn history_fixture() -> Vec<Commit> {
        vec![
            commit("c1", Some(100), vec![file("a.rs", 10, 0)]),
            commit(
                "c2",
                Some(200),
                vec![file("a.rs", 5, 5), file("b.rs", 3, 0)],
            ),
            commit("c3", Some(300), vec![file("b.rs", 7, 1)]),
        ]
    }

    fn structural_index() -> HashMap<String, StructuralFeatures> {
        ["a.rs", "b.rs", "c.rs"]
            .iter()
            .map(|path| ((*path).to_owned(), structural(path)))
            .collect()
    }

    #[test]
    fn positive_and_negative_labels() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );

        let positives: Vec<&DatasetRow> = rows.iter().filter(|row| row.label == 1).collect();
        let positive_keys: HashSet<(&str, &str)> = positives
            .iter()
            .map(|row| (row.commit_sha.as_str(), row.file_path.as_str()))
            .collect();
        assert!(positive_keys.contains(&("c1", "a.rs")));
        assert!(positive_keys.contains(&("c2", "a.rs")));
        assert!(positive_keys.contains(&("c2", "b.rs")));
        assert!(positive_keys.contains(&("c3", "b.rs")));

        let negatives: Vec<&DatasetRow> = rows.iter().filter(|row| row.label == 0).collect();

        assert_eq!(negatives.len(), 1);
        assert_eq!(negatives[0].commit_sha, "c3");
        assert_eq!(negatives[0].file_path, "a.rs");
    }

    #[test]
    fn historical_features_exclude_target_commit() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );

        let row = rows
            .iter()
            .find(|row| row.commit_sha == "c2" && row.file_path == "a.rs")
            .unwrap();
        assert_eq!(row.historical.previous_change_count, 1);
        assert_eq!(row.historical.historical_churn, 10);
        assert_eq!(row.historical.time_since_last_change_secs, Some(100));

        let first = rows.iter().find(|row| row.commit_sha == "c1").unwrap();
        assert_eq!(first.historical.previous_change_count, 0);
        assert_eq!(first.historical.time_since_last_change_secs, None);
    }

    #[test]
    fn untimestamped_commits_are_excluded() {
        let mut commits = history_fixture();
        commits.push(commit("ghost", None, vec![file("a.rs", 99, 99)]));

        let rows = build_dataset(
            &commits,
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );

        assert!(rows.iter().all(|row| row.commit_sha != "ghost"));
        let last = rows
            .iter()
            .find(|row| row.commit_sha == "c3" && row.file_path == "b.rs")
            .unwrap();
        assert_eq!(last.historical.previous_change_count, 1);
    }

    #[test]
    fn prefix_slice_excludes_equal_timestamps_and_ghosts() {
        let commits = vec![
            commit("c1", Some(100), vec![file("a.rs", 1, 0)]),
            commit(
                "ghost",
                None,
                vec![file("a.rs", 50, 50), file("b.rs", 50, 50)],
            ),
            commit("c2", Some(100), vec![file("b.rs", 1, 0)]),
            commit(
                "c3",
                Some(200),
                vec![file("a.rs", 1, 0), file("b.rs", 1, 0)],
            ),
        ];

        let rows = build_dataset(
            &commits,
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );

        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|row| row.commit_sha != "ghost"));

        let second = rows
            .iter()
            .find(|row| row.commit_sha == "c2" && row.file_path == "b.rs")
            .unwrap();
        assert_eq!(second.historical.previous_change_count, 0);
        assert_eq!(second.historical.time_since_last_change_secs, None);

        let third_a = rows
            .iter()
            .find(|row| row.commit_sha == "c3" && row.file_path == "a.rs")
            .unwrap();
        assert_eq!(third_a.historical.previous_change_count, 1);
        assert_eq!(third_a.historical.historical_churn, 1);

        let third_b = rows
            .iter()
            .find(|row| row.commit_sha == "c3" && row.file_path == "b.rs")
            .unwrap();
        assert_eq!(third_b.historical.previous_change_count, 1);
    }

    #[test]
    fn generation_is_deterministic() {
        let config = DatasetConfig::default();
        let first = build_dataset(&history_fixture(), &structural_index(), &[], &config);
        let second = build_dataset(&history_fixture(), &structural_index(), &[], &config);
        assert_eq!(first, second);
    }

    #[test]
    fn rows_are_chronologically_ordered() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let stamps: Vec<i64> = rows.iter().map(|row| row.timestamp).collect();
        let mut sorted = stamps.clone();
        sorted.sort_unstable();
        assert_eq!(stamps, sorted);
    }

    #[test]
    fn validator_flags_duplicates_and_bad_rows() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let mut dirty = rows.clone();
        dirty.push(rows[0].clone());
        dirty.push(DatasetRow {
            file_path: String::new(),
            ..rows[0].clone()
        });
        dirty.push(DatasetRow {
            label: 7,
            ..rows[0].clone()
        });

        let report = validate_dataset(&dirty, &history_fixture());

        assert_eq!(report.total_rows, dirty.len());
        assert_eq!(report.removed.len(), 3);
        assert_eq!(report.valid_rows, dirty.len() - 3);
        assert!(
            report
                .removed
                .iter()
                .any(|row| row.reason.contains("duplicate"))
        );
        assert!(
            report
                .removed
                .iter()
                .any(|row| row.reason.contains("binary"))
        );
    }

    #[test]
    fn validator_flags_impossible_negatives() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let mut dirty = rows.clone();
        dirty.push(DatasetRow {
            commit_sha: "c1".to_owned(),
            timestamp: 100,
            file_path: "c.rs".to_owned(),
            structural: structural("c.rs"),
            historical: rows[0].historical.clone(),
            temporal: rows[0].temporal.clone(),
            label: 0,
        });

        let report = validate_dataset(&dirty, &history_fixture());

        assert!(
            report
                .removed
                .iter()
                .any(|row| row.reason.contains("never existed"))
        );
    }

    #[test]
    fn split_keeps_commits_whole_and_ordered() {
        let commits: Vec<Commit> = (0..10)
            .map(|index| {
                commit(
                    &format!("c{index}"),
                    Some(100 + index as i64),
                    vec![file("a.rs", 1, 0)],
                )
            })
            .collect();
        let rows = build_dataset(
            &commits,
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let split = split_chronologically(rows, &DatasetConfig::default());

        assert_eq!(
            split
                .train
                .iter()
                .map(|row| row.commit_sha.as_str())
                .collect::<HashSet<_>>()
                .len(),
            7
        );
        assert_eq!(
            split
                .validation
                .iter()
                .map(|row| row.commit_sha.as_str())
                .collect::<HashSet<_>>()
                .len(),
            1
        );
        assert_eq!(
            split
                .test
                .iter()
                .map(|row| row.commit_sha.as_str())
                .collect::<HashSet<_>>()
                .len(),
            2
        );

        let train_max = split.train.iter().map(|row| row.timestamp).max().unwrap();
        let validation_min = split
            .validation
            .iter()
            .map(|row| row.timestamp)
            .min()
            .unwrap();
        let test_min = split.test.iter().map(|row| row.timestamp).min().unwrap();
        assert!(train_max < validation_min && validation_min <= test_min);
    }

    #[test]
    fn tiny_histories_stay_in_train() {
        let commits = vec![
            commit("c1", Some(100), vec![file("a.rs", 1, 0)]),
            commit("c2", Some(200), vec![file("a.rs", 1, 0)]),
        ];
        let rows = build_dataset(
            &commits,
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );

        let split = split_chronologically(rows, &DatasetConfig::default());
        assert!(!split.train.is_empty());
        assert!(split.validation.is_empty());
        assert!(split.test.is_empty());
    }

    #[test]
    fn stats_report_class_distribution() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let refs: Vec<&DatasetRow> = rows.iter().collect();
        let stats = dataset_stats(&refs);

        assert_eq!(stats.total_examples, 5);
        assert_eq!(stats.positives, 4);
        assert_eq!(stats.negatives, 1);
        assert!((stats.positive_ratio - 0.8).abs() < f64::EPSILON);
        assert_eq!(stats.commits, 3);
        assert_eq!(stats.files, 2);

        let empty = dataset_stats(&[]);
        assert_eq!(empty.positive_ratio, 0.0);
    }

    #[test]
    fn export_writes_valid_jsonl() {
        let rows = build_dataset(
            &history_fixture(),
            &structural_index(),
            &[],
            &DatasetConfig::default(),
        );
        let path = std::env::temp_dir().join("repoinsight-dataset-test.jsonl");
        let written = export_jsonl(&rows, path.to_str().unwrap()).unwrap();

        assert_eq!(written, rows.len());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), rows.len());
        let first: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert!(first.get("label").is_some());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn structural_extraction_uses_real_source_data() {
        let source = crate::analysis::source::SourceFile {
            path: "src/example.rs".to_owned(),
            content: "fn main() {}\nfn test() {}\n".to_owned(),
            size_bytes: 27,
        };
        let complexity = ComplexityResult::new(3, 2, 1);
        let dependencies = DependencyMetrics::default();

        let result = crate::analysis::features::build_structural_features(
            &source,
            &complexity,
            &dependencies,
        );

        assert_eq!(result.lines_of_code, 2);
        assert_eq!(result.file_size_bytes, 27);
        assert_eq!(result.function_count, 2);
    }
}
