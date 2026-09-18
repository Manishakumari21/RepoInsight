use serde::Serialize;
use std::collections::HashMap;

use crate::github::commits::Commit;

use super::history::{HistoryMetrics, relevant_files};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HistoricalFeatures {
    pub file_path: String,
    pub previous_change_count: usize,
    pub historical_churn: u64,
    pub contributor_count: usize,
    pub time_since_last_change_secs: Option<i64>,
    pub recent_change_frequency: f64,
    pub historical_cochange_frequency: usize,
}

impl HistoricalFeatures {
    /// Zero history for files with no prior changes (e.g. first targets).
    pub fn empty(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_owned(),
            previous_change_count: 0,
            historical_churn: 0,
            contributor_count: 0,
            time_since_last_change_secs: None,
            recent_change_frequency: 0.0,
            historical_cochange_frequency: 0,
        }
    }
}

/// Commit-aware historical features using only commits strictly before
/// `cutoff_ts` (exclusive). `commits` must be timestamp-ordered; entries
/// without a usable timestamp are skipped. Unlike the repository-level
/// snapshot above, this never observes the target commit or any future
/// commit, and contributor counts are per-file rather than repository-wide.
pub fn build_historical_prefix(
    commits: &[Commit],
    cutoff_ts: i64,
    recent_window_secs: i64,
) -> HashMap<String, HistoricalFeatures> {
    #[derive(Default)]
    struct Accumulator {
        changes: usize,
        churn: u64,
        contributors: std::collections::HashSet<String>,
        last_change: Option<i64>,
        recent_changes: usize,
    }

    let mut stats: HashMap<String, Accumulator> = HashMap::new();
    let mut cochange: HashMap<(String, String), usize> = HashMap::new();

    for commit in commits {
        let Some(timestamp) = commit.timestamp else {
            continue;
        };
        if timestamp >= cutoff_ts {
            break;
        }

        let files = relevant_files(&commit.files);
        let author = commit.author.as_ref().map(|author| {
            author
                .email
                .as_deref()
                .filter(|email| !email.is_empty())
                .unwrap_or(&author.name)
                .trim()
                .to_owned()
        });

        for file in &files {
            let entry = stats.entry(file.filename.clone()).or_default();
            entry.changes += 1;
            entry.churn = entry
                .churn
                .saturating_add(file.additions.saturating_add(file.deletions));
            if let Some(name) = &author {
                if !name.is_empty() {
                    entry.contributors.insert(name.clone());
                }
            }
            entry.last_change = Some(timestamp);
            if cutoff_ts - timestamp <= recent_window_secs {
                entry.recent_changes += 1;
            }
        }

        let mut names: Vec<&str> = files.iter().map(|file| file.filename.as_str()).collect();
        names.sort_unstable();
        for (i, first) in names.iter().enumerate() {
            for second in &names[i + 1..] {
                let key = (first.to_string(), second.to_string());
                *cochange.entry(key).or_insert(0) += 1;
            }
        }
    }

    stats
        .into_iter()
        .map(|(path, entry)| {
            let cochange_frequency = cochange
                .iter()
                .filter(|((source, target), _)| source == &path || target == &path)
                .map(|(_, count)| *count)
                .sum();
            (
                path.clone(),
                HistoricalFeatures {
                    file_path: path,
                    previous_change_count: entry.changes,
                    historical_churn: entry.churn,
                    contributor_count: entry.contributors.len(),
                    time_since_last_change_secs: entry.last_change.map(|last| cutoff_ts - last),
                    recent_change_frequency: entry.recent_changes as f64
                        / recent_window_secs.max(1) as f64,
                    historical_cochange_frequency: cochange_frequency,
                },
            )
        })
        .collect()
}

pub fn build_historical_features(
    commits: &[Commit],
    metrics: &HistoryMetrics,
) -> Vec<HistoricalFeatures> {
    let mut last_change: HashMap<String, i64> = HashMap::new();
    let mut recent_changes: HashMap<String, usize> = HashMap::new();

    let latest_timestamp = commits.iter().filter_map(|commit| commit.timestamp).max();

    for commit in commits {
        let Some(timestamp) = commit.timestamp else {
            continue;
        };

        for file in relevant_files(&commit.files) {
            last_change
                .entry(file.filename.clone())
                .and_modify(|value| *value = (*value).max(timestamp))
                .or_insert(timestamp);

            if let Some(latest) = latest_timestamp {
                if latest - timestamp <= 30 * 24 * 60 * 60 {
                    *recent_changes.entry(file.filename).or_insert(0) += 1;
                }
            }
        }
    }

    metrics
        .file_changes
        .iter()
        .map(|(path, history)| {
            let time_since_last_change_secs =
                latest_timestamp.and_then(|latest| last_change.get(path).map(|last| latest - last));

            let recent_count = recent_changes.get(path).copied().unwrap_or(0);

            let recent_change_frequency = recent_count as f64 / 30.0;

            let historical_cochange_frequency = metrics
                .cochange_pairs
                .iter()
                .filter(|((source, target), _)| source == path || target == path)
                .map(|(_, count)| *count)
                .sum();

            HistoricalFeatures {
                file_path: path.clone(),
                previous_change_count: history.change_count,
                historical_churn: history.churn,
                contributor_count: metrics.active_contributors,
                time_since_last_change_secs,
                recent_change_frequency,
                historical_cochange_frequency,
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_features_from_history() {
        let metrics = HistoryMetrics {
            active_contributors: 2,
            file_changes: HashMap::from([(
                "src/main.rs".to_string(),
                super::super::history::FileHistory {
                    change_count: 3,
                    churn: 120,
                    ..Default::default()
                },
            )]),
            cochange_pairs: HashMap::from([(
                ("src/main.rs".to_string(), "src/lib.rs".to_string()),
                2,
            )]),
            ..Default::default()
        };

        let features = build_historical_features(&[], &metrics);

        assert_eq!(features.len(), 1);
        assert_eq!(features[0].file_path, "src/main.rs");
        assert_eq!(features[0].previous_change_count, 3);
        assert_eq!(features[0].historical_churn, 120);
        assert_eq!(features[0].contributor_count, 2);
        assert_eq!(features[0].time_since_last_change_secs, None);
        assert_eq!(features[0].recent_change_frequency, 0.0);
        assert_eq!(features[0].historical_cochange_frequency, 2);
    }

    #[test]
    fn missing_history_values_default_correctly() {
        let metrics = HistoryMetrics::default();

        let features = build_historical_features(&[], &metrics);

        assert!(features.is_empty());
    }

    fn prefix_commit(
        sha: &str,
        timestamp: Option<i64>,
        author: Option<(&str, &str)>,
        files: Vec<(&str, u64, u64)>,
    ) -> Commit {
        use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};

        Commit {
            sha: sha.to_owned(),
            message: sha.to_owned(),
            author: author.map(|(name, email)| crate::github::commits::CommitAuthor {
                name: name.to_owned(),
                email: Some(email.to_owned()),
                date: None,
            }),
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

    #[test]
    fn prefix_excludes_target_and_future_commits() {
        let commits = vec![
            prefix_commit(
                "c1",
                Some(100),
                Some(("amy", "amy@x.io")),
                vec![("a.rs", 10, 2)],
            ),
            prefix_commit(
                "c2",
                Some(200),
                Some(("amy", "amy@x.io")),
                vec![("a.rs", 5, 5)],
            ),
            prefix_commit(
                "target",
                Some(300),
                Some(("bob", "bob@x.io")),
                vec![("a.rs", 100, 100)],
            ),
            prefix_commit("later", Some(400), None, vec![("a.rs", 1, 1)]),
        ];

        let features = build_historical_prefix(&commits, 300, 30 * 24 * 60 * 60);
        let file = &features["a.rs"];

        assert_eq!(file.previous_change_count, 2);
        assert_eq!(file.historical_churn, 22);
        assert_eq!(file.contributor_count, 1);
        assert_eq!(file.time_since_last_change_secs, Some(100));
        assert_eq!(file.historical_cochange_frequency, 0);
    }

    #[test]
    fn prefix_counts_per_file_contributors_and_cochanges() {
        let commits = vec![
            prefix_commit(
                "c1",
                Some(100),
                Some(("amy", "amy@x.io")),
                vec![("a.rs", 1, 0), ("b.rs", 1, 0)],
            ),
            prefix_commit(
                "c2",
                Some(200),
                Some(("bob", "bob@x.io")),
                vec![("a.rs", 1, 0)],
            ),
        ];

        let features = build_historical_prefix(&commits, 1000, 30 * 24 * 60 * 60);

        assert_eq!(features["a.rs"].contributor_count, 2);
        assert_eq!(features["b.rs"].contributor_count, 1);
        assert_eq!(features["a.rs"].historical_cochange_frequency, 1);
        assert_eq!(features.get("missing.rs"), None);
    }

    #[test]
    fn prefix_skips_commits_without_timestamps() {
        let commits = vec![
            prefix_commit("c1", None, None, vec![("a.rs", 50, 50)]),
            prefix_commit("c2", Some(200), None, vec![("a.rs", 1, 0)]),
        ];

        let features = build_historical_prefix(&commits, 300, 30 * 24 * 60 * 60);

        assert_eq!(features["a.rs"].previous_change_count, 1);
        assert_eq!(features["a.rs"].historical_churn, 1);
    }
}
