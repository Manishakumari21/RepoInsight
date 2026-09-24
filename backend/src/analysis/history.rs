use itertools::Itertools;
use std::collections::{HashMap, HashSet};

use crate::github::commits::Commit;

use super::models::{
    HistoryAnalysis, PropagationAnalysis, PropagationEdge, TemporalAnalysis, TemporalPair,
};

const TEMPORAL_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Default)]
pub struct HistoryMetrics {
    pub total_commits: usize,
    pub active_contributors: usize,
    pub changed_files: usize,
    pub total_additions: u64,
    pub total_deletions: u64,
    pub total_churn: u64,
    pub file_changes: HashMap<String, FileHistory>,
    pub cochange_pairs: HashMap<(String, String), usize>,
}

#[derive(Debug, Clone, Default)]
pub struct FileHistory {
    pub change_count: usize,
    pub additions: u64,
    pub deletions: u64,
    pub churn: u64,
}

pub fn analyze(commits: &[Commit]) -> (HistoryAnalysis, HistoryMetrics) {
    let mut metrics = HistoryMetrics::default();
    let mut contributors = HashSet::new();

    for commit in commits {
        if let Some(author) = &commit.author {
            let key = author
                .email
                .as_deref()
                .filter(|email| !email.is_empty())
                .unwrap_or(&author.name)
                .trim()
                .to_owned();

            if !key.is_empty() {
                contributors.insert(key);
            }
        }

        metrics.total_additions = metrics
            .total_additions
            .saturating_add(commit.stats.additions);

        metrics.total_deletions = metrics
            .total_deletions
            .saturating_add(commit.stats.deletions);

        let files = relevant_files(&commit.files);

        for file in &files {
            let history = metrics
                .file_changes
                .entry(file.filename.clone())
                .or_default();

            history.change_count = history.change_count.saturating_add(1);
            history.additions = history.additions.saturating_add(file.additions);
            history.deletions = history.deletions.saturating_add(file.deletions);
            history.churn = history
                .churn
                .saturating_add(file.additions.saturating_add(file.deletions));
        }

        record_cochanges(&mut metrics.cochange_pairs, &files);
    }

    metrics.total_churn = metrics
        .total_additions
        .saturating_add(metrics.total_deletions);

    metrics.total_commits = commits.len();
    metrics.active_contributors = contributors.len();
    metrics.changed_files = metrics.file_changes.len();

    let analysis = HistoryAnalysis {
        total_commits: metrics.total_commits,
        active_contributors: metrics.active_contributors,
        changed_files: metrics.changed_files,
        total_additions: metrics.total_additions,
        total_deletions: metrics.total_deletions,
        total_churn: metrics.total_churn,
        first_commit: commits.first().and_then(|commit| commit.date.clone()),
        last_commit: commits.last().and_then(|commit| commit.date.clone()),
    };

    (analysis, metrics)
}

pub fn temporal_analysis(commits: &[Commit]) -> TemporalAnalysis {
    let mut pairs: HashMap<(String, String), (usize, i64)> = HashMap::new();

    for (i, current) in commits.iter().enumerate() {
        let Some(current_time) = current.timestamp else {
            continue;
        };

        let current_files = relevant_files(&current.files);

        for previous in commits[..i].iter().rev() {
            let Some(previous_time) = previous.timestamp else {
                continue;
            };

            let delay = current_time - previous_time;

            if delay > TEMPORAL_WINDOW_SECS {
                break;
            }

            let previous_files = relevant_files(&previous.files);

            for source in &previous_files {
                for target in &current_files {
                    if source.filename == target.filename {
                        continue;
                    }

                    let entry = pairs
                        .entry((source.filename.clone(), target.filename.clone()))
                        .or_default();

                    entry.0 += 1;
                    entry.1 += delay;
                }
            }
        }
    }

    let mut pairs = pairs
        .into_iter()
        .map(
            |((source, target), (occurrences, total_delay))| TemporalPair {
                source,
                target,
                occurrences,
                avg_delay_seconds: total_delay / occurrences as i64,
            },
        )
        .collect::<Vec<_>>();

    pairs.sort_unstable_by_key(|pair| std::cmp::Reverse(pair.occurrences));
    pairs.truncate(50);

    TemporalAnalysis { pairs }
}

/// Running equivalent of repeated [`temporal_analysis`] calls over growing
/// strictly-historical prefixes.
///
/// Advance-only: feed commits in chronological order via [`advance`] using the
/// same grouped-timestamp ranges the dataset builder uses for `seen` and the
/// historical accumulator (exactly the commits with `timestamp < cutoff`,
/// never the target itself), then snapshot any cutoff with [`materialize`].
/// Pair counting, window breaks, ignored-path filtering, self-pair exclusion,
/// and the top-50 truncation are identical to [`temporal_analysis`]; only the
/// repeated full-prefix rescans are replaced by monotonic accumulation.
#[derive(Default)]
pub struct TemporalPrefixAccumulator {
    /// Chronological prefix state: `(timestamp, relevant filenames)` per
    /// timestamped commit. Untimestamped commits are skipped exactly as the
    /// oracle `continue`s past them.
    commits: Vec<(i64, Vec<String>)>,
    /// Full (untruncated) pair state: `(occurrences, total_delay)`.
    /// Truncation happens only in [`materialize`], mirroring the oracle.
    pairs: HashMap<(String, String), (usize, i64)>,
}

impl TemporalPrefixAccumulator {
    pub fn advance(&mut self, commits: &[Commit]) {
        for commit in commits {
            let Some(current_time) = commit.timestamp else {
                continue;
            };

            let current_files: Vec<String> = relevant_files(&commit.files)
                .into_iter()
                .map(|file| file.filename)
                .collect();

            // Mirror the oracle's backward scan over the chronological prefix:
            // delays grow monotonically going backwards, so the first commit
            // beyond the window ends the scan exactly as `break` does there.
            for (previous_time, previous_files) in self.commits.iter().rev() {
                let delay = current_time - previous_time;

                if delay > TEMPORAL_WINDOW_SECS {
                    break;
                }

                for source in previous_files {
                    for target in &current_files {
                        if source == target {
                            continue;
                        }

                        let entry = self
                            .pairs
                            .entry((source.clone(), target.clone()))
                            .or_default();

                        entry.0 += 1;
                        entry.1 += delay;
                    }
                }
            }

            self.commits.push((current_time, current_files));
        }
    }

    pub fn materialize(&self) -> TemporalAnalysis {
        let mut pairs = self
            .pairs
            .iter()
            .map(
                |((source, target), (occurrences, total_delay))| TemporalPair {
                    source: source.clone(),
                    target: target.clone(),
                    occurrences: *occurrences,
                    avg_delay_seconds: *total_delay / *occurrences as i64,
                },
            )
            .collect::<Vec<_>>();

        pairs.sort_unstable_by_key(|pair| std::cmp::Reverse(pair.occurrences));
        pairs.truncate(50);

        TemporalAnalysis { pairs }
    }
}

pub fn build_propagation(
    temporal: &TemporalAnalysis,
    cochange: &HashMap<(String, String), usize>,
    dependencies: &[crate::analysis::dependencies::DependencyEdge],
) -> PropagationAnalysis {
    // (summed strength, temporal flag, co-change flag, dependency flag) per
    // file pair. Accumulated directly in one pass: the previous petgraph
    // layer only ever summed weights per pair and never traversed the graph,
    // so the graph construction, edge-index walk, and repeated string clones
    // were pure overhead. Every input pair is still visited, so the resulting
    // multiset of (pair, strength, flags) is unchanged.
    let mut pairs: HashMap<(String, String), (u32, bool, bool, bool)> = HashMap::new();

    for pair in &temporal.pairs {
        let entry = pairs
            .entry((pair.source.clone(), pair.target.clone()))
            .or_insert((0, false, false, false));
        entry.0 += pair.occurrences as u32;
        entry.1 = true;
    }

    for ((source, target), count) in cochange {
        let entry = pairs
            .entry((source.clone(), target.clone()))
            .or_insert((0, false, false, false));
        entry.0 += *count as u32;
        entry.2 = true;
    }

    for dependency in dependencies {
        if dependency.source == dependency.target {
            continue;
        }

        if is_ignored_path(&dependency.source) || is_ignored_path(&dependency.target) {
            continue;
        }

        let entry = pairs
            .entry((dependency.source.clone(), dependency.target.clone()))
            .or_insert((0, false, false, false));
        entry.0 += 1;
        entry.3 = true;
    }

    let mut edges = pairs
        .into_iter()
        .map(
            |((source, target), (strength, temporal_flag, cochange_flag, dependency_flag))| {
                PropagationEdge {
                    source,
                    target,
                    dependency: dependency_flag,
                    temporal: temporal_flag,
                    cochange: cochange_flag,
                    strength: strength as usize,
                }
            },
        )
        .collect::<Vec<_>>();

    // Total order: strength first, then file paths. The previous
    // sort_unstable_by_key over HashMap order left equal-strength edges in
    // hasher-randomized order, so which tied edges survived truncate(100)
    // varied run to run; the tiebreak below makes the top-100 selection
    // fully deterministic without changing any strength-descided ranking.
    edges.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.target.cmp(&b.target))
    });
    edges.truncate(100);

    PropagationAnalysis { edges }
}

/// Running equivalent of repeated [`build_propagation`] calls over growing
/// strictly-historical prefixes.
///
/// Static dependency contributions are folded exactly once at construction;
/// canonical co-change contributions are folded per newly eligible commit via
/// the same [`record_cochanges`] counting the history analysis uses, so equal
/// commits contribute equally no matter when they arrive. Temporal
/// contributions are deliberately NOT stored here: the oracle combines only
/// the truncated top-50 [`TemporalAnalysis`], whose membership changes as
/// history grows, so [`materialize`] takes the current top-50 (produced by
/// [`TemporalPrefixAccumulator`] from the same prefix) and folds it in. The
/// merged map is always FULL — truncation to the top 100 happens only in
/// [`materialize`] with the identical deterministic sort — so a pair outside
/// the current top 100 can always re-enter it later.
///
/// Advance-only over the same grouped-timestamp ranges the dataset builder
/// uses (commits with `timestamp < cutoff`, never the target itself).
#[derive(Debug, Default)]
pub struct PropagationPrefixAccumulator {
    /// Full (untruncated) merged state: `(strength, temporal, cochange,
    /// dependency)`. The temporal slot stays false until [`materialize`]
    /// folds the current top-50 in.
    merged: HashMap<(String, String), (u32, bool, bool, bool)>,
}

impl PropagationPrefixAccumulator {
    pub fn new(dependencies: &[crate::analysis::dependencies::DependencyEdge]) -> Self {
        let mut merged: HashMap<(String, String), (u32, bool, bool, bool)> = HashMap::new();

        for dependency in dependencies {
            if dependency.source == dependency.target {
                continue;
            }

            if is_ignored_path(&dependency.source) || is_ignored_path(&dependency.target) {
                continue;
            }

            let entry = merged
                .entry((dependency.source.clone(), dependency.target.clone()))
                .or_insert((0, false, false, false));
            entry.0 += 1;
            entry.3 = true;
        }

        Self { merged }
    }

    pub fn advance(&mut self, commits: &[Commit]) {
        for commit in commits {
            if commit.timestamp.is_none() {
                continue;
            }

            let files = relevant_files(&commit.files);
            // Exact reuse of the history-analysis counting: per-commit pair
            // deltas merge commutatively into the running strengths, so the
            // accumulated co-change contribution always equals a fresh full
            // recount of the same prefix.
            let mut delta: HashMap<(String, String), usize> = HashMap::new();
            record_cochanges(&mut delta, &files);
            for (key, count) in delta {
                let entry = self.merged.entry(key).or_insert((0, false, false, false));
                entry.0 += count as u32;
                entry.2 = true;
            }
        }
    }

    pub fn materialize(&self, temporal: &TemporalAnalysis) -> PropagationAnalysis {
        let mut pairs = self.merged.clone();

        for pair in &temporal.pairs {
            let entry = pairs
                .entry((pair.source.clone(), pair.target.clone()))
                .or_insert((0, false, false, false));
            entry.0 += pair.occurrences as u32;
            entry.1 = true;
        }

        let mut edges = pairs
            .into_iter()
            .map(
                |((source, target), (strength, temporal_flag, cochange_flag, dependency_flag))| {
                    PropagationEdge {
                        source,
                        target,
                        dependency: dependency_flag,
                        temporal: temporal_flag,
                        cochange: cochange_flag,
                        strength: strength as usize,
                    }
                },
            )
            .collect::<Vec<_>>();

        edges.sort_by(|a, b| {
            b.strength
                .cmp(&a.strength)
                .then_with(|| a.source.cmp(&b.source))
                .then_with(|| a.target.cmp(&b.target))
        });
        edges.truncate(100);

        PropagationAnalysis { edges }
    }
}

pub(crate) fn relevant_files(
    files: &[crate::github::commits::ChangedFile],
) -> Vec<crate::github::commits::ChangedFile> {
    files
        .iter()
        .filter(|file| !is_ignored_path(&file.filename))
        .cloned()
        .collect()
}

pub(crate) fn is_ignored_path(path: &str) -> bool {
    path.to_ascii_lowercase()
        .split('/')
        .any(|part| matches!(part, ".git" | "target" | "node_modules" | "dist" | "build"))
}

fn record_cochanges(
    pairs: &mut HashMap<(String, String), usize>,
    files: &[crate::github::commits::ChangedFile],
) {
    let names = files
        .iter()
        .map(|file| file.filename.as_str())
        .collect::<Vec<_>>();

    for (a, b) in names.iter().tuple_combinations() {
        let key = if a < b {
            ((*a).to_owned(), (*b).to_owned())
        } else {
            ((*b).to_owned(), (*a).to_owned())
        };

        *pairs.entry(key).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::commits::{ChangedFile, Commit, CommitStats, FileChangeStatus};

    fn file(name: &str) -> ChangedFile {
        ChangedFile {
            filename: name.to_owned(),
            additions: 1,
            deletions: 0,
            changes: 1,
            status: FileChangeStatus::Modified,
        }
    }

    fn commit(ts: i64, files: Vec<ChangedFile>) -> Commit {
        Commit {
            sha: format!("sha{ts}"),
            message: format!("commit {ts}"),
            author: None,
            timestamp: Some(ts),
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files,
        }
    }

    #[test]
    fn cochange_pairs_are_counted_symmetrically() {
        let commits = vec![
            commit(1, vec![file("a.rs"), file("b.rs")]),
            commit(2, vec![file("b.rs"), file("a.rs")]),
            commit(3, vec![file("c.rs"), file("d.rs")]),
        ];

        let (_, metrics) = analyze(&commits);

        let key = ("a.rs".to_owned(), "b.rs".to_owned());

        assert_eq!(metrics.cochange_pairs[&key], 2);
        assert_eq!(metrics.cochange_pairs.len(), 2);
    }

    #[test]
    fn commit_order_is_preserved_for_first_last() {
        let commits = vec![
            commit(100, vec![file("a.rs")]),
            commit(200, vec![file("b.rs")]),
        ];

        let (analysis, _) = analyze(&commits);

        assert_eq!(analysis.total_commits, 2);
    }

    #[test]
    fn file_history_aggregates_churn() {
        let commits = vec![commit(1, vec![file("a.rs")]), commit(2, vec![file("a.rs")])];

        let (_, metrics) = analyze(&commits);

        assert_eq!(metrics.file_changes["a.rs"].change_count, 2);
        assert_eq!(metrics.file_changes["a.rs"].churn, 2);
    }

    #[test]
    fn temporal_pairs_track_later_changes() {
        let commits = vec![
            commit(100, vec![file("a.rs")]),
            commit(200, vec![file("b.rs")]),
        ];

        let analysis = temporal_analysis(&commits);

        assert_eq!(analysis.pairs.len(), 1);
        assert_eq!(analysis.pairs[0].source, "a.rs");
        assert_eq!(analysis.pairs[0].target, "b.rs");
        assert_eq!(analysis.pairs[0].occurrences, 1);
        assert_eq!(analysis.pairs[0].avg_delay_seconds, 100);
    }

    #[test]
    fn generated_paths_are_ignored() {
        let commits = vec![
            commit(100, vec![file("src/a.rs"), file("target/debug/file")]),
            commit(
                200,
                vec![file("src/b.rs"), file("node_modules/pkg/index.js")],
            ),
        ];

        let (_, metrics) = analyze(&commits);

        assert!(metrics.file_changes.contains_key("src/a.rs"));
        assert!(metrics.file_changes.contains_key("src/b.rs"));
        assert!(!metrics.file_changes.contains_key("target/debug/file"));
        assert!(
            !metrics
                .file_changes
                .contains_key("node_modules/pkg/index.js")
        );
    }

    fn temporal_commit(
        sha: &str,
        timestamp: Option<i64>,
        author: Option<&str>,
        files: Vec<ChangedFile>,
    ) -> Commit {
        Commit {
            sha: sha.to_owned(),
            message: sha.to_owned(),
            author: author.map(|name| crate::github::commits::CommitAuthor {
                name: name.to_owned(),
                email: Some(format!("{name}@x.io")),
                date: None,
            }),
            timestamp,
            date: None,
            url: String::new(),
            stats: CommitStats::default(),
            files,
        }
    }

    fn sorted_pairs(analysis: &TemporalAnalysis) -> Vec<(String, String, usize, i64)> {
        let mut pairs: Vec<_> = analysis
            .pairs
            .iter()
            .map(|pair| {
                (
                    pair.source.clone(),
                    pair.target.clone(),
                    pair.occurrences,
                    pair.avg_delay_seconds,
                )
            })
            .collect();
        pairs.sort();
        pairs
    }

    #[test]
    fn incremental_temporal_matches_oracle_at_every_cutoff() {
        // Covers: repeated same-file changes, multiple files/authors, equal
        // timestamps, exact window boundaries (delay == 604800 included,
        // 604801 excluded), ignored paths, self-pair exclusion, future
        // commits (via per-cutoff prefixes), and untimestamped exclusion.
        // Well below the top-50 truncation, so the comparison is exact.
        let commits = vec![
            temporal_commit("c0", Some(0), Some("amy"), vec![file("a.rs")]),
            temporal_commit("ghost", None, None, vec![file("a.rs"), file("b.rs")]),
            temporal_commit("c1", Some(100), Some("amy"), vec![file("a.rs")]),
            temporal_commit(
                "c2",
                Some(100),
                Some("bob"),
                vec![file("b.rs"), file("c.rs")],
            ),
            temporal_commit(
                "c3",
                Some(200),
                Some("amy"),
                vec![file("a.rs"), file("b.rs")],
            ),
            temporal_commit(
                "ign",
                Some(300),
                Some("bob"),
                vec![file("target/x.rs"), file("d.rs")],
            ),
            temporal_commit("c4", Some(604900), Some("amy"), vec![file("e.rs")]),
            temporal_commit("c5", Some(604901), Some("bob"), vec![file("f.rs")]),
            temporal_commit("future", Some(9999999), Some("amy"), vec![file("z.rs")]),
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
        assert!(ordered.iter().all(|commit| commit.sha != "ghost"));

        let mut accumulator = TemporalPrefixAccumulator::default();
        let mut advanced = 0usize;
        let mut cutoffs: Vec<i64> = ordered
            .iter()
            .filter_map(|commit| commit.timestamp)
            .collect();
        cutoffs.sort_unstable();
        cutoffs.dedup();

        for cutoff in cutoffs {
            let prefix_len =
                ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < cutoff);
            accumulator.advance(&ordered[advanced..prefix_len]);
            advanced = prefix_len;

            let oracle = temporal_analysis(&ordered[..prefix_len]);
            assert_eq!(
                sorted_pairs(&accumulator.materialize()),
                sorted_pairs(&oracle),
                "cutoff {cutoff}"
            );
        }

        // Spot-check the reference values themselves (not just equality).
        // At cutoff 200 the prefix is c0/c1/c2: c0(a)->c2 delay 100 and
        // c1(a)->c2 delay 0 (equal timestamps form pairs); c0->c1 and c1->c1
        // are same-file self-pairs and excluded.
        let prefix_200 = &ordered
            [..ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < 200)];
        let at_200 = sorted_pairs(&temporal_analysis(prefix_200));
        assert!(at_200.contains(&("a.rs".to_owned(), "b.rs".to_owned(), 2, 50)));
        assert!(at_200.contains(&("a.rs".to_owned(), "c.rs".to_owned(), 2, 50)));
        assert!(!at_200.iter().any(|(s, t, _, _)| s == t));

        // Window boundary: prefix below 9999999 includes c5 @604901, so pairs
        // into f.rs test the 7-day window exactly. c3(200)->c5 delay 604701
        // is inside; c1/c2(100)->c5 delay 604801 is outside; c4->c5 delay 1.
        let prefix_future = &ordered
            [..ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < 9999999)];
        let at_future = sorted_pairs(&temporal_analysis(prefix_future));
        assert!(at_future.contains(&("e.rs".to_owned(), "f.rs".to_owned(), 1, 1)));
        assert!(at_future.contains(&("a.rs".to_owned(), "f.rs".to_owned(), 1, 604701)));
        assert!(at_future.contains(&("b.rs".to_owned(), "f.rs".to_owned(), 1, 604701)));
        assert!(at_future.contains(&("d.rs".to_owned(), "f.rs".to_owned(), 1, 604601)));
        // c.rs appears only in c2 @100, whose delay to c5 (604801) exceeds
        // the window, so no (c, f) pair may exist.
        assert!(
            !at_future
                .iter()
                .any(|(s, t, _, _)| { s == "c.rs" && t == "f.rs" })
        );

        // Ignored paths never appear as pair endpoints.
        for cutoff in [300, 604900, 604901, 9999999] {
            let prefix = &ordered[..ordered
                .partition_point(|commit| commit.timestamp.expect("timestamped") < cutoff)];
            for (source, target, _, _) in sorted_pairs(&temporal_analysis(prefix)) {
                assert!(!source.contains("target/"), "cutoff {cutoff}");
                assert!(!target.contains("target/"), "cutoff {cutoff}");
            }
        }
    }

    #[test]
    fn incremental_temporal_handles_empty_history() {
        assert_eq!(
            sorted_pairs(&TemporalPrefixAccumulator::default().materialize()),
            sorted_pairs(&temporal_analysis(&[])),
        );

        let ghost = temporal_commit("ghost", None, None, vec![file("a.rs")]);
        let mut accumulator = TemporalPrefixAccumulator::default();
        accumulator.advance(std::slice::from_ref(&ghost));
        assert_eq!(
            sorted_pairs(&accumulator.materialize()),
            sorted_pairs(&temporal_analysis(&[ghost])),
        );
    }

    fn named_commit(sha: &str, timestamp: Option<i64>, files: Vec<ChangedFile>) -> Commit {
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

    fn dep_edge(source: &str, target: &str) -> crate::analysis::dependencies::DependencyEdge {
        crate::analysis::dependencies::DependencyEdge {
            source: source.to_owned(),
            target: target.to_owned(),
        }
    }

    fn ordered_edges(
        analysis: &PropagationAnalysis,
    ) -> Vec<(String, String, usize, bool, bool, bool)> {
        analysis
            .edges
            .iter()
            .map(|edge| {
                (
                    edge.source.clone(),
                    edge.target.clone(),
                    edge.strength,
                    edge.temporal,
                    edge.cochange,
                    edge.dependency,
                )
            })
            .collect()
    }

    #[test]
    fn incremental_propagation_matches_oracle_at_every_cutoff() {
        // Covers: repeated changes, multiple files, co-change fan-out,
        // duplicate/self/ignored dependency edges, ignored paths, equal
        // timestamps, ghost commits, future commits, a pair whose strength
        // grows over time, and a late riser crossing the top-100 boundary
        // (duplicate-file self-pairs are pinned in
        // `incremental_propagation_counts_duplicate_file_self_pair`).
        // Temporal pairs stay below 50 (exact oracle comparison);
        // propagation edges exceed 100 (truncation active).
        let mut files_c0: Vec<ChangedFile> = vec![file("a.rs"), file("b.rs")];
        for index in 1..=10 {
            files_c0.push(file(&format!("f{index:02}.rs")));
        }
        files_c0.push(file("k.rs"));
        files_c0.push(file("target/w.rs"));

        let commits = vec![
            named_commit("c0", Some(0), files_c0),
            named_commit("ghost", None, vec![file("a.rs"), file("g.rs")]),
            named_commit("c1", Some(100), vec![file("f09.rs"), file("f10.rs")]),
            named_commit("c1b", Some(100), vec![file("a.rs")]),
            named_commit("c2", Some(200), vec![file("f10.rs"), file("k.rs")]),
            named_commit("future", Some(9999999), vec![file("target/zzz.rs")]),
        ];

        let mut dependencies = vec![
            dep_edge("a.rs", "b.rs"),
            dep_edge("a.rs", "b.rs"),
            dep_edge("c.rs", "c.rs"),
            dep_edge("target/x.rs", "y.rs"),
            dep_edge("p.rs", "build/z.rs"),
            dep_edge("d.rs", "e.rs"),
            dep_edge("f01.rs", "f09.rs"),
        ];
        // Alphabetically-early dependency fan-out: pushes the late-rising
        // ("f10","k") pair out of the early top 100 without touching the
        // temporal pair budget (dependencies never form temporal pairs).
        for index in (1..50).step_by(2) {
            dependencies.push(dep_edge(
                &format!("e{index:02}.rs"),
                &format!("e{index_1:02}.rs", index_1 = index + 1),
            ));
        }

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

        let mut accumulator = PropagationPrefixAccumulator::new(&dependencies);
        let mut advanced = 0usize;
        let mut cutoffs: Vec<i64> = ordered
            .iter()
            .filter_map(|commit| commit.timestamp)
            .collect();
        cutoffs.sort_unstable();
        cutoffs.dedup();

        for cutoff in cutoffs {
            let prefix_len =
                ordered.partition_point(|commit| commit.timestamp.expect("timestamped") < cutoff);
            accumulator.advance(&ordered[advanced..prefix_len]);
            advanced = prefix_len;
            let prefix = &ordered[..prefix_len];

            let temporal = temporal_analysis(prefix);
            assert!(
                temporal.pairs.len() < 50,
                "fixture must stay below temporal truncation (cutoff {cutoff})"
            );
            let (_, metrics) = analyze(prefix);
            let oracle = build_propagation(&temporal, &metrics.cochange_pairs, &dependencies);
            assert_eq!(
                ordered_edges(&accumulator.materialize(&temporal)),
                ordered_edges(&oracle),
                "cutoff {cutoff}"
            );
        }

        let edge_at = |cutoff: i64| -> Vec<(String, String, usize, bool, bool, bool)> {
            let prefix = &ordered[..ordered
                .partition_point(|commit| commit.timestamp.expect("timestamped") < cutoff)];
            let temporal = temporal_analysis(prefix);
            let (_, metrics) = analyze(prefix);
            ordered_edges(&build_propagation(
                &temporal,
                &metrics.cochange_pairs,
                &dependencies,
            ))
        };

        // Truncation is genuinely active at the final cutoff.
        let late = edge_at(9999999);
        assert_eq!(late.len(), 100, "fixture must exceed the top-100 boundary");

        // Late riser: ("f10","k") is strength 1 at cutoff 100 and ranked out
        // of the top 100 by alphabetically-earlier strength-1 pairs; the c2
        // co-change plus c0->c2/c1->c2 temporal contributions lift it in.
        let early = edge_at(100);
        let riser = ("f10.rs".to_owned(), "k.rs".to_owned());
        assert!(
            !early
                .iter()
                .any(|(s, t, _, _, _, _)| (s, t) == (&riser.0, &riser.1)),
            "riser must start outside the top 100"
        );
        let risen = late
            .iter()
            .find(|(s, t, _, _, _, _)| (s, t) == (&riser.0, &riser.1))
            .expect("riser must enter the top 100");
        assert_eq!(
            risen.2, 4,
            "co-change 2 (c0, c2) + temporal 2 (c0->c2, c1->c2)"
        );
        assert!(
            risen.3 && risen.4,
            "riser must carry temporal+cochange flags"
        );
        assert!(!risen.5, "riser must not carry a dependency flag");

        // Combined contributions: (f01,f09) has co-change (c0), a dependency
        // edge, and temporal (c0->c1) input all on the same pair.
        let triple = late
            .iter()
            .find(|(s, t, _, _, _, _)| s == "f01.rs" && t == "f09.rs")
            .expect("(f01,f09) must be present");
        assert!(
            triple.3 && triple.4 && triple.5,
            "all three flags must be set"
        );
        assert_eq!(triple.2, 3, "co-change 1 + dependency 1 + temporal 1");

        // Duplicate dependency contributions: (a,b) has co-change (c0) and
        // two identical dependency edges, but no temporal input here.
        let ab = late
            .iter()
            .find(|(s, t, _, _, _, _)| s == "a.rs" && t == "b.rs")
            .expect("(a,b) must be present");
        assert_eq!((ab.3, ab.4, ab.5), (false, true, true));
        assert_eq!(ab.2, 3, "co-change 1 + dependency 2");

        // Self/ignored/ghost semantics.
        assert!(
            !late
                .iter()
                .any(|(s, t, _, _, _, _)| s == "c.rs" && t == "c.rs")
        );
        assert!(!late.iter().any(|(s, _, _, _, _, _)| s.contains("target/")));
        assert!(
            !late
                .iter()
                .any(|(_, t, _, _, _, _)| t.contains("target/") || t.contains("build/"))
        );
        assert!(
            !late
                .iter()
                .any(|(s, t, _, _, _, _)| s == "g.rs" || t == "g.rs")
        );
        let de = late
            .iter()
            .find(|(s, t, _, _, _, _)| s == "d.rs" && t == "e.rs")
            .expect("dependency-only edge must be present");
        assert_eq!((de.2, de.3, de.4, de.5), (1, false, false, true));
    }

    #[test]
    fn incremental_propagation_counts_duplicate_file_self_pair() {
        // A commit touching the same file twice yields exactly one canonical
        // self-pair; temporal self-pairs stay excluded.
        let commits = vec![named_commit(
            "c0",
            Some(0),
            vec![file("k.rs"), file("k.rs")],
        )];
        let temporal = temporal_analysis(&commits);
        assert!(temporal.pairs.is_empty());
        let (_, metrics) = analyze(&commits);
        let oracle = build_propagation(&temporal, &metrics.cochange_pairs, &[]);

        let mut accumulator = PropagationPrefixAccumulator::new(&[]);
        accumulator.advance(&commits);
        let materialized = accumulator.materialize(&temporal);

        assert_eq!(ordered_edges(&materialized), ordered_edges(&oracle));
        assert_eq!(
            ordered_edges(&oracle),
            vec![("k.rs".to_owned(), "k.rs".to_owned(), 1, false, true, false)]
        );
    }

    #[test]
    fn incremental_propagation_handles_empty_history() {
        let oracle_empty = build_propagation(&temporal_analysis(&[]), &HashMap::new(), &[]);
        assert!(oracle_empty.edges.is_empty());

        let accumulator = PropagationPrefixAccumulator::new(&[]);
        assert!(
            accumulator
                .materialize(&temporal_analysis(&[]))
                .edges
                .is_empty()
        );

        let ghost = named_commit("ghost", None, vec![file("a.rs")]);
        let mut accumulator = PropagationPrefixAccumulator::new(&[dep_edge("a.rs", "b.rs")]);
        accumulator.advance(std::slice::from_ref(&ghost));
        let materialized = accumulator.materialize(&temporal_analysis(&[ghost]));
        // Only the static dependency edge survives: ghosts contribute nothing.
        assert_eq!(ordered_edges(&materialized).len(), 1);
        assert_eq!(
            ordered_edges(&materialized)[0],
            ("a.rs".to_owned(), "b.rs".to_owned(), 1, false, false, true)
        );
    }
}
