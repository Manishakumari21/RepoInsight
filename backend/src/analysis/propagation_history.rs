use std::collections::{HashMap, HashSet};

use crate::github::commits::Commit;

use super::history::relevant_files;
use super::models::{
    ChangeSequence, ChangeSequences, ChangeTimeline, FollowUp, FollowUpAnalysis, HistoricalExample,
    HistoricalExamples, ReworkAnalysis, ReworkEvent, TemporalAnalysis, TimelineEntry,
};

pub const DEFAULT_SEQUENCE_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;
pub const DEFAULT_FOLLOWUP_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;
pub const DEFAULT_REWORK_WINDOW_SECS: i64 = 14 * 24 * 60 * 60;

pub const MAX_TIMELINE_ENTRIES: usize = 300;
pub const MAX_SEQUENCES: usize = 50;
pub const MAX_FOLLOWUPS: usize = 100;
pub const MAX_REWORK_EVENTS: usize = 100;
pub const MAX_EXAMPLES: usize = 20;
const MAX_FILES_PER_COMMIT_FOR_SEQUENCES: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct PropagationConfig {
    pub sequence_window_secs: i64,
    pub followup_window_secs: i64,
    pub rework_window_secs: i64,
}

impl Default for PropagationConfig {
    fn default() -> Self {
        Self {
            sequence_window_secs: DEFAULT_SEQUENCE_WINDOW_SECS,
            followup_window_secs: DEFAULT_FOLLOWUP_WINDOW_SECS,
            rework_window_secs: DEFAULT_REWORK_WINDOW_SECS,
        }
    }
}

pub fn build_timeline(commits: &[Commit]) -> ChangeTimeline {
    let mut ordered: Vec<(usize, &Commit)> = commits.iter().enumerate().collect();
    ordered.sort_by_key(|(index, commit)| (commit.timestamp.unwrap_or(0), *index));

    let total_commits = ordered.len();
    let truncated = total_commits > MAX_TIMELINE_ENTRIES;

    let entries = ordered
        .into_iter()
        .enumerate()
        .map(|(_, (original_index, commit))| TimelineEntry {
            sha: commit.sha.clone(),
            order: original_index,
            timestamp: commit.timestamp,
            date: commit.date.clone(),
            author: commit.author.as_ref().map(|author| author.name.clone()),
            message: commit.message.clone(),
            files: relevant_files(&commit.files)
                .iter()
                .map(|file| file.filename.clone())
                .collect(),
            additions: commit.stats.additions,
            deletions: commit.stats.deletions,
        })
        .collect::<Vec<_>>();

    let entries = if truncated {
        entries[entries.len() - MAX_TIMELINE_ENTRIES..].to_vec()
    } else {
        entries
    };

    ChangeTimeline {
        entries,
        total_commits,
        truncated,
    }
}

pub fn detect_sequences(commits: &[Commit], config: &PropagationConfig) -> ChangeSequences {
    let ordered = sorted_commits(commits);
    let file_lists: Vec<Vec<String>> = ordered
        .iter()
        .map(|commit| commit_file_names(commit))
        .collect();

    let mut counts: HashMap<Vec<String>, (usize, i64)> = HashMap::new();

    for i in 0..ordered.len().saturating_sub(1) {
        let (Some(t0), Some(t1)) = (ordered[i].timestamp, ordered[i + 1].timestamp) else {
            continue;
        };
        if t1 < t0 || t1 - t0 > config.sequence_window_secs {
            continue;
        }
        let delay = t1 - t0;
        for a in &file_lists[i] {
            for b in &file_lists[i + 1] {
                if a == b {
                    continue;
                }
                let entry = counts.entry(vec![a.clone(), b.clone()]).or_default();
                entry.0 += 1;
                entry.1 += delay;
            }
        }
    }

    for i in 0..ordered.len().saturating_sub(2) {
        let (Some(t0), Some(t2)) = (ordered[i].timestamp, ordered[i + 2].timestamp) else {
            continue;
        };
        if t2 < t0 || t2 - t0 > config.sequence_window_secs {
            continue;
        }
        let delay = t2 - t0;
        for a in &file_lists[i] {
            for b in &file_lists[i + 1] {
                if a == b {
                    continue;
                }
                for c in &file_lists[i + 2] {
                    if b == c {
                        continue;
                    }
                    let entry = counts
                        .entry(vec![a.clone(), b.clone(), c.clone()])
                        .or_default();
                    entry.0 += 1;
                    entry.1 += delay;
                }
            }
        }
    }

    let mut sequences = counts
        .into_iter()
        .map(|(files, (occurrences, total_delay))| ChangeSequence {
            avg_delay_seconds: total_delay / occurrences as i64,
            kind: if files.len() == 2 {
                "pair".to_owned()
            } else {
                "triple".to_owned()
            },
            evidence_type: "observed".to_owned(),
            files,
            occurrences,
        })
        .collect::<Vec<_>>();

    sequences.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.avg_delay_seconds.cmp(&b.avg_delay_seconds))
            .then_with(|| a.files.cmp(&b.files))
    });
    sequences.truncate(MAX_SEQUENCES);

    ChangeSequences {
        sequences,
        window_seconds: config.sequence_window_secs,
    }
}

pub fn detect_followups(
    commits: &[Commit],
    dependencies: &[crate::analysis::dependencies::DependencyEdge],
    cochange_pairs: &HashMap<(String, String), usize>,
    config: &PropagationConfig,
) -> FollowUpAnalysis {
    let ordered = sorted_commits(commits);
    let file_sets: Vec<HashSet<String>> = ordered
        .iter()
        .map(|commit| {
            relevant_files(&commit.files)
                .iter()
                .map(|file| file.filename.clone())
                .collect()
        })
        .collect();

    let dependency_links: HashSet<(String, String)> = dependencies
        .iter()
        .flat_map(|edge| {
            [
                (edge.source.clone(), edge.target.clone()),
                (edge.target.clone(), edge.source.clone()),
            ]
        })
        .collect();

    let cochange_links: HashSet<(String, String)> = cochange_pairs.keys().cloned().collect();

    let mut followups = Vec::new();

    for (j, current) in ordered.iter().enumerate() {
        let Some(current_time) = current.timestamp else {
            continue;
        };
        if file_sets[j].is_empty() {
            continue;
        }

        for i in (0..j).rev() {
            let Some(previous_time) = ordered[i].timestamp else {
                continue;
            };
            let delay = current_time - previous_time;
            if delay < 0 {
                continue;
            }
            if delay > config.followup_window_secs {
                break;
            }
            if file_sets[i].is_empty() {
                continue;
            }

            let Some(followup) = classify_followup(
                ordered[i],
                &file_sets[i],
                current,
                &file_sets[j],
                delay,
                &dependency_links,
                &cochange_links,
            ) else {
                continue;
            };

            followups.push(followup);
        }
    }

    followups.sort_by(|a: &FollowUp, b: &FollowUp| {
        a.delay_seconds
            .cmp(&b.delay_seconds)
            .then_with(|| a.source_sha.cmp(&b.source_sha))
            .then_with(|| a.followup_sha.cmp(&b.followup_sha))
    });

    let total = followups.len();
    followups.truncate(MAX_FOLLOWUPS);

    FollowUpAnalysis {
        followups,
        total,
        window_seconds: config.followup_window_secs,
    }
}

fn classify_followup(
    previous: &Commit,
    previous_files: &HashSet<String>,
    current: &Commit,
    current_files: &HashSet<String>,
    delay: i64,
    dependency_links: &HashSet<(String, String)>,
    cochange_links: &HashSet<(String, String)>,
) -> Option<FollowUp> {
    let overlap: Vec<String> = previous_files
        .intersection(current_files)
        .cloned()
        .collect();

    let mut previous_sorted: Vec<String> = previous_files.iter().cloned().collect();
    previous_sorted.sort();
    let mut current_sorted: Vec<String> = current_files.iter().cloned().collect();
    current_sorted.sort();

    let fix = is_fix_message(&current.message);

    if !overlap.is_empty() {
        let (reason, signals) = if fix {
            (
                "same-file-fix".to_owned(),
                vec!["same-file".to_owned(), "fix-message".to_owned()],
            )
        } else {
            ("same-file".to_owned(), vec!["same-file".to_owned()])
        };
        return Some(FollowUp {
            source_sha: previous.sha.clone(),
            source_files: previous_sorted,
            followup_sha: current.sha.clone(),
            followup_files: current_sorted,
            delay_seconds: delay,
            reason,
            signals,
            evidence_type: "observed".to_owned(),
        });
    }

    let related_cochange = previous_files.iter().any(|a| {
        current_files.iter().any(|b| {
            let key = canonical_pair(a, b);
            cochange_links.contains(&key)
        })
    });
    let related_dependency = previous_files.iter().any(|a| {
        current_files
            .iter()
            .any(|b| dependency_links.contains(&(a.clone(), b.clone())))
    });

    if related_cochange {
        return Some(FollowUp {
            source_sha: previous.sha.clone(),
            source_files: previous_sorted,
            followup_sha: current.sha.clone(),
            followup_files: current_sorted,
            delay_seconds: delay,
            reason: "co-change-history".to_owned(),
            signals: vec!["co-change".to_owned()],
            evidence_type: "derived".to_owned(),
        });
    }

    if related_dependency {
        return Some(FollowUp {
            source_sha: previous.sha.clone(),
            source_files: previous_sorted,
            followup_sha: current.sha.clone(),
            followup_files: current_sorted,
            delay_seconds: delay,
            reason: "dependency".to_owned(),
            signals: vec!["dependency".to_owned()],
            evidence_type: "derived".to_owned(),
        });
    }

    if fix {
        return Some(FollowUp {
            source_sha: previous.sha.clone(),
            source_files: previous_sorted,
            followup_sha: current.sha.clone(),
            followup_files: current_sorted,
            delay_seconds: delay,
            reason: "fix-message".to_owned(),
            signals: vec!["fix-message".to_owned()],
            evidence_type: "derived".to_owned(),
        });
    }

    None
}

pub fn detect_rework(
    commits: &[Commit],
    dependencies: &[crate::analysis::dependencies::DependencyEdge],
    cochange_pairs: &HashMap<(String, String), usize>,
    config: &PropagationConfig,
) -> ReworkAnalysis {
    let ordered = sorted_commits(commits);
    let file_sets: Vec<HashSet<String>> = ordered
        .iter()
        .map(|commit| {
            relevant_files(&commit.files)
                .iter()
                .map(|file| file.filename.clone())
                .collect()
        })
        .collect();

    let dependency_links: HashSet<(String, String)> = dependencies
        .iter()
        .flat_map(|edge| {
            [
                (edge.source.clone(), edge.target.clone()),
                (edge.target.clone(), edge.source.clone()),
            ]
        })
        .collect();
    let cochange_links: HashSet<(String, String)> = cochange_pairs.keys().cloned().collect();

    let mut events = Vec::new();
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();

    for (j, current) in ordered.iter().enumerate() {
        let Some(current_time) = current.timestamp else {
            continue;
        };
        if file_sets[j].is_empty() {
            continue;
        }
        let revert = is_revert_message(&current.message);
        let fix = is_fix_message(&current.message);

        for i in (0..j).rev() {
            let Some(previous_time) = ordered[i].timestamp else {
                continue;
            };
            let delay = current_time - previous_time;
            if delay < 0 {
                continue;
            }
            if delay > config.rework_window_secs {
                break;
            }
            if file_sets[i].is_empty() {
                continue;
            }

            let mut overlap: Vec<String> =
                file_sets[i].intersection(&file_sets[j]).cloned().collect();
            overlap.sort();

            for file in overlap {
                let rule = if revert {
                    "revert"
                } else if fix {
                    "fix-message"
                } else {
                    "repeated-touch"
                };
                let key = (
                    file.clone(),
                    ordered[i].sha.clone(),
                    current.sha.clone(),
                    rule.to_owned(),
                );
                if !seen.insert(key) {
                    continue;
                }
                events.push(ReworkEvent {
                    evidence: format!(
                        "candidate rework: {file} changed again after {delay}s ({} -> {}) [rule: {rule}]",
                        short_sha(&ordered[i].sha),
                        short_sha(&current.sha),
                    ),
                    file,
                    initial_sha: ordered[i].sha.clone(),
                    rework_sha: current.sha.clone(),
                    delay_seconds: delay,
                    rule: rule.to_owned(),
                });
            }

            if file_sets[i].is_disjoint(&file_sets[j]) && fix {
                let related = file_sets[i].iter().any(|a| {
                    file_sets[j].iter().any(|b| {
                        cochange_links.contains(&canonical_pair(a, b))
                            || dependency_links.contains(&(a.clone(), b.clone()))
                    })
                });
                if related {
                    let mut target_files: Vec<String> = file_sets[j].iter().cloned().collect();
                    target_files.sort();
                    let file = target_files.first().cloned().unwrap_or_default();
                    let key = (
                        file.clone(),
                        ordered[i].sha.clone(),
                        current.sha.clone(),
                        "related-fix".to_owned(),
                    );
                    if seen.insert(key) {
                        events.push(ReworkEvent {
                            evidence: format!(
                                "candidate rework: {file} (related to {}) changed with a fix message after {delay}s ({} -> {}) [rule: related-fix]",
                                join_sorted(&file_sets[i]),
                                short_sha(&ordered[i].sha),
                                short_sha(&current.sha),
                            ),
                            file,
                            initial_sha: ordered[i].sha.clone(),
                            rework_sha: current.sha.clone(),
                            delay_seconds: delay,
                            rule: "related-fix".to_owned(),
                        });
                    }
                }
            }
        }
    }

    events.sort_by(|a, b| {
        a.delay_seconds
            .cmp(&b.delay_seconds)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.initial_sha.cmp(&b.initial_sha))
            .then_with(|| a.rework_sha.cmp(&b.rework_sha))
    });

    let total = events.len();
    events.truncate(MAX_REWORK_EVENTS);

    ReworkAnalysis {
        events,
        total,
        window_seconds: config.rework_window_secs,
    }
}

pub fn build_examples(
    commits: &[Commit],
    temporal: &TemporalAnalysis,
    sequences: &ChangeSequences,
    cochange_pairs: &HashMap<(String, String), usize>,
    dependencies: &[crate::analysis::dependencies::DependencyEdge],
    config: &PropagationConfig,
) -> HistoricalExamples {
    let ordered = sorted_commits(commits);
    let dependency_links: HashSet<(String, String)> = dependencies
        .iter()
        .flat_map(|edge| {
            [
                (edge.source.clone(), edge.target.clone()),
                (edge.target.clone(), edge.source.clone()),
            ]
        })
        .collect();

    let mut examples = Vec::new();

    for (index, pair) in temporal.pairs.iter().take(10).enumerate() {
        let mut signals = vec!["temporal".to_owned()];
        if cochange_pairs.contains_key(&canonical_pair(&pair.source, &pair.target)) {
            signals.push("co-change".to_owned());
        }
        if dependency_links.contains(&(pair.source.clone(), pair.target.clone())) {
            signals.push("dependency".to_owned());
        }
        examples.push(HistoricalExample {
            id: format!("temporal-{}", index + 1),
            source_file: pair.source.clone(),
            target_file: pair.target.clone(),
            sequence: vec![pair.source.clone(), pair.target.clone()],
            occurrences: pair.occurrences,
            avg_delay_seconds: pair.avg_delay_seconds,
            signals,
            evidence_type: "observed".to_owned(),
            example_shas: supporting_pair_shas(
                &ordered,
                &pair.source,
                &pair.target,
                config.followup_window_secs,
            ),
        });
    }

    for (index, sequence) in sequences.sequences.iter().take(10).enumerate() {
        let Some(first) = sequence.files.first() else {
            continue;
        };
        let Some(last) = sequence.files.last() else {
            continue;
        };
        let mut signals = vec!["sequence".to_owned()];
        if sequence.files.len() == 2 && cochange_pairs.contains_key(&canonical_pair(first, last)) {
            signals.push("co-change".to_owned());
        }
        if sequence
            .files
            .windows(2)
            .any(|w| dependency_links.contains(&(w[0].clone(), w[1].clone())))
        {
            signals.push("dependency".to_owned());
        }
        examples.push(HistoricalExample {
            id: format!("sequence-{}", index + 1),
            source_file: first.clone(),
            target_file: last.clone(),
            sequence: sequence.files.clone(),
            occurrences: sequence.occurrences,
            avg_delay_seconds: sequence.avg_delay_seconds,
            signals,
            evidence_type: "observed".to_owned(),
            example_shas: supporting_sequence_shas(
                &ordered,
                &sequence.files,
                config.sequence_window_secs,
            ),
        });
    }

    examples.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.avg_delay_seconds.cmp(&b.avg_delay_seconds))
            .then_with(|| a.id.cmp(&b.id))
    });
    let total = examples.len();
    examples.truncate(MAX_EXAMPLES);

    HistoricalExamples { examples, total }
}

fn sorted_commits(commits: &[Commit]) -> Vec<&Commit> {
    let mut ordered: Vec<(usize, &Commit)> = commits.iter().enumerate().collect();
    ordered.sort_by_key(|(index, commit)| (commit.timestamp.unwrap_or(0), *index));
    ordered.into_iter().map(|(_, commit)| commit).collect()
}

fn commit_file_names(commit: &Commit) -> Vec<String> {
    let mut names: HashSet<String> = relevant_files(&commit.files)
        .iter()
        .map(|file| file.filename.clone())
        .collect();
    let mut sorted: Vec<String> = names.drain().collect();
    sorted.sort();
    sorted.truncate(MAX_FILES_PER_COMMIT_FOR_SEQUENCES);
    sorted
}

fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

fn message_tokens(message: &str) -> Vec<String> {
    message
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn is_fix_message(message: &str) -> bool {
    let tokens: HashSet<String> = message_tokens(message).into_iter().collect();
    [
        "fix",
        "fixes",
        "fixed",
        "fixing",
        "bugfix",
        "bugfixes",
        "hotfix",
        "hotfixes",
        "patch",
        "patches",
        "patched",
        "revert",
        "reverts",
        "reverted",
        "reverting",
        "followup",
        "followups",
        "bug",
        "bugs",
    ]
    .iter()
    .any(|word| tokens.contains(*word))
}

pub fn is_revert_message(message: &str) -> bool {
    let trimmed = message.trim_start().to_ascii_lowercase();
    if trimmed.starts_with("revert") {
        return true;
    }
    let tokens: HashSet<String> = message_tokens(message).into_iter().collect();
    ["revert", "reverts", "reverted", "reverting"]
        .iter()
        .any(|word| tokens.contains(*word))
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
}

fn join_sorted(files: &HashSet<String>) -> String {
    let mut sorted: Vec<&String> = files.iter().collect();
    sorted.sort();
    sorted
        .into_iter()
        .take(3)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn supporting_pair_shas(
    ordered: &[&Commit],
    source: &str,
    target: &str,
    window_secs: i64,
) -> Vec<String> {
    let mut shas = Vec::new();
    for (j, current) in ordered.iter().enumerate() {
        let Some(current_time) = current.timestamp else {
            continue;
        };
        let current_names: HashSet<String> = relevant_files(&current.files)
            .iter()
            .map(|file| file.filename.clone())
            .collect();
        if !current_names.contains(target) {
            continue;
        }
        for previous in ordered[..j].iter().rev() {
            let Some(previous_time) = previous.timestamp else {
                continue;
            };
            let delay = current_time - previous_time;
            if delay > window_secs {
                break;
            }
            let previous_names: HashSet<String> = relevant_files(&previous.files)
                .iter()
                .map(|file| file.filename.clone())
                .collect();
            if previous_names.contains(source) {
                shas.push(previous.sha.clone());
                shas.push(current.sha.clone());
                break;
            }
        }
        if shas.len() >= 4 {
            break;
        }
    }
    shas.sort();
    shas.dedup();
    shas.truncate(4);
    shas
}

fn supporting_sequence_shas(
    ordered: &[&Commit],
    files: &[String],
    window_secs: i64,
) -> Vec<String> {
    if files.len() < 2 {
        return Vec::new();
    }
    let mut shas = Vec::new();
    if files.len() == 2 {
        return supporting_pair_shas(ordered, &files[0], &files[1], window_secs);
    }
    for window in ordered.windows(3) {
        let (Some(t0), Some(t2)) = (window[0].timestamp, window[2].timestamp) else {
            continue;
        };
        if t2 < t0 || t2 - t0 > window_secs {
            continue;
        }
        let names: Vec<HashSet<String>> = window
            .iter()
            .map(|commit| {
                relevant_files(&commit.files)
                    .iter()
                    .map(|file| file.filename.clone())
                    .collect()
            })
            .collect();
        if names[0].contains(&files[0])
            && names[1].contains(&files[1])
            && names[2].contains(&files[2])
        {
            shas.push(window[0].sha.clone());
            shas.push(window[1].sha.clone());
            shas.push(window[2].sha.clone());
            break;
        }
    }
    shas.sort();
    shas.dedup();
    shas.truncate(4);
    shas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::commits::{ChangedFile, CommitStats, FileChangeStatus};

    fn file(name: &str) -> ChangedFile {
        ChangedFile {
            filename: name.to_owned(),
            additions: 1,
            deletions: 0,
            changes: 1,
            status: FileChangeStatus::Modified,
        }
    }

    fn commit(sha: &str, ts: i64, message: &str, files: Vec<ChangedFile>) -> Commit {
        Commit {
            sha: sha.to_owned(),
            message: message.to_owned(),
            author: None,
            timestamp: Some(ts),
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files,
        }
    }

    fn config() -> PropagationConfig {
        PropagationConfig {
            sequence_window_secs: 7 * 24 * 60 * 60,
            followup_window_secs: 7 * 24 * 60 * 60,
            rework_window_secs: 14 * 24 * 60 * 60,
        }
    }

    #[test]
    fn empty_history_yields_empty_results() {
        let commits: Vec<Commit> = Vec::new();
        let cfg = config();

        assert!(build_timeline(&commits).entries.is_empty());
        assert!(detect_sequences(&commits, &cfg).sequences.is_empty());
        assert!(
            detect_followups(&commits, &[], &HashMap::new(), &cfg)
                .followups
                .is_empty()
        );
        assert!(
            detect_rework(&commits, &[], &HashMap::new(), &cfg)
                .events
                .is_empty()
        );
    }

    #[test]
    fn timeline_is_chronological_with_order_and_details() {
        let commits = vec![
            commit("c3", 300, "third", vec![file("b.rs")]),
            commit("c1", 100, "first", vec![file("a.rs")]),
            commit("c2", 200, "second", vec![file("a.rs"), file("b.rs")]),
        ];

        let timeline = build_timeline(&commits);

        assert_eq!(timeline.total_commits, 3);
        assert!(!timeline.truncated);
        let shas: Vec<&str> = timeline
            .entries
            .iter()
            .map(|entry| entry.sha.as_str())
            .collect();
        assert_eq!(shas, vec!["c1", "c2", "c3"]);
        assert_eq!(timeline.entries[1].files.len(), 2);
        assert_eq!(timeline.entries[0].timestamp, Some(100));
    }

    #[test]
    fn sequences_detect_pair_and_triple() {
        let commits = vec![
            commit("c1", 100, "one", vec![file("a.rs")]),
            commit("c2", 200, "two", vec![file("b.rs")]),
            commit("c3", 300, "three", vec![file("c.rs")]),
        ];

        let sequences = detect_sequences(&commits, &config());

        let pair = sequences
            .sequences
            .iter()
            .find(|s| s.files == vec!["a.rs", "b.rs"])
            .expect("pair a->b");
        assert_eq!(pair.kind, "pair");
        assert_eq!(pair.evidence_type, "observed");

        let triple = sequences
            .sequences
            .iter()
            .find(|s| s.files == vec!["a.rs", "b.rs", "c.rs"])
            .expect("triple a->b->c");
        assert_eq!(triple.kind, "triple");
        assert_eq!(triple.occurrences, 1);
    }

    #[test]
    fn temporal_window_excludes_distant_sequences() {
        let commits = vec![
            commit("c1", 100, "one", vec![file("a.rs")]),
            commit("c2", 100 + 30 * 24 * 60 * 60, "two", vec![file("b.rs")]),
        ];
        let cfg = PropagationConfig {
            sequence_window_secs: 7 * 24 * 60 * 60,
            ..config()
        };

        assert!(detect_sequences(&commits, &cfg).sequences.is_empty());
    }

    #[test]
    fn same_file_followup_is_observed() {
        let commits = vec![
            commit("c1", 100, "add feature", vec![file("a.rs")]),
            commit("c2", 200, "extend feature", vec![file("a.rs")]),
        ];

        let result = detect_followups(&commits, &[], &HashMap::new(), &config());

        assert_eq!(result.followups.len(), 1);
        assert_eq!(result.followups[0].reason, "same-file");
        assert_eq!(result.followups[0].evidence_type, "observed");
    }

    #[test]
    fn fix_message_does_not_match_substrings() {
        assert!(!is_fix_message("add prefix handling"));
        assert!(!is_fix_message("update suffix tree"));
        assert!(is_fix_message("fix login redirect"));
        assert!(is_fix_message("Fixes #42"));
    }

    #[test]
    fn revert_commits_are_detected() {
        assert!(is_revert_message("Revert \"add feature\""));
        assert!(is_revert_message("revert broken change"));
        assert!(!is_revert_message("add feature"));
    }

    #[test]
    fn revert_rule_takes_precedence_over_repeated_touch() {
        let commits = vec![
            commit("c1", 100, "add feature", vec![file("a.rs")]),
            commit("c2", 200, "Revert \"add feature\"", vec![file("a.rs")]),
        ];

        let result = detect_rework(&commits, &[], &HashMap::new(), &config());

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].rule, "revert");
        assert!(result.events[0].evidence.contains("[rule: revert]"));
    }

    #[test]
    fn rework_reports_rule_and_evidence() {
        let commits = vec![
            commit("c1", 100, "add feature", vec![file("a.rs")]),
            commit("c2", 200, "fix edge case", vec![file("a.rs")]),
        ];

        let result = detect_rework(&commits, &[], &HashMap::new(), &config());

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].rule, "fix-message");
        assert!(result.events[0].evidence.contains("candidate rework"));
        assert!(result.events[0].evidence.contains("[rule: fix-message]"));
    }

    #[test]
    fn duplicate_pairs_are_prevented() {
        let commits = vec![
            commit("c1", 100, "one", vec![file("a.rs")]),
            commit("c2", 200, "two", vec![file("a.rs")]),
        ];

        let followups = detect_followups(&commits, &[], &HashMap::new(), &config());
        assert_eq!(followups.followups.len(), 1);

        let rework = detect_rework(&commits, &[], &HashMap::new(), &config());
        assert_eq!(rework.events.len(), 1);
    }

    #[test]
    fn unrelated_commits_produce_no_followup() {
        let commits = vec![
            commit("c1", 100, "add docs", vec![file("docs.md")]),
            commit("c2", 200, "update readme", vec![file("readme.md")]),
        ];

        let result = detect_followups(&commits, &[], &HashMap::new(), &config());
        assert!(result.followups.is_empty());
    }
}
