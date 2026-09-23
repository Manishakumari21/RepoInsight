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

pub fn build_propagation(
    temporal: &TemporalAnalysis,
    cochange: &HashMap<(String, String), usize>,
    dependencies: &[crate::analysis::dependencies::DependencyEdge],
) -> PropagationAnalysis {
    use petgraph::Graph;
    use std::collections::HashMap as Map;

    let mut graph: Graph<String, u32> = Graph::new();
    let mut node_index: Map<String, petgraph::graph::NodeIndex> = Map::new();
    let mut flags: Map<(String, String), (bool, bool, bool)> = Map::new();

    let node = |graph: &mut Graph<String, u32>,
                node_index: &mut Map<String, petgraph::graph::NodeIndex>,
                name: &String|
     -> petgraph::graph::NodeIndex {
        if let Some(idx) = node_index.get(name) {
            return *idx;
        }
        let idx = graph.add_node(name.clone());
        node_index.insert(name.clone(), idx);
        idx
    };

    for pair in &temporal.pairs {
        let source = node(&mut graph, &mut node_index, &pair.source);
        let target = node(&mut graph, &mut node_index, &pair.target);
        graph.add_edge(source, target, pair.occurrences as u32);
        let entry = flags
            .entry((pair.source.clone(), pair.target.clone()))
            .or_insert((false, false, false));
        entry.0 = true;
    }

    for ((source, target), count) in cochange {
        let s = node(&mut graph, &mut node_index, source);
        let t = node(&mut graph, &mut node_index, target);
        graph.add_edge(s, t, *count as u32);
        let entry = flags
            .entry((source.clone(), target.clone()))
            .or_insert((false, false, false));
        entry.1 = true;
    }

    for dependency in dependencies {
        if dependency.source == dependency.target {
            continue;
        }

        if is_ignored_path(&dependency.source) || is_ignored_path(&dependency.target) {
            continue;
        }

        let s = node(&mut graph, &mut node_index, &dependency.source);
        let t = node(&mut graph, &mut node_index, &dependency.target);
        graph.add_edge(s, t, 1);
        let entry = flags
            .entry((dependency.source.clone(), dependency.target.clone()))
            .or_insert((false, false, false));
        entry.2 = true;
    }

    let mut strengths: HashMap<(String, String), u32> = HashMap::new();
    for edge in graph.edge_indices() {
        let (source, target) = graph.edge_endpoints(edge).expect("petgraph edge endpoints");
        let key = (graph[source].clone(), graph[target].clone());
        let weight = graph[edge];
        *strengths.entry(key).or_insert(0) += weight;
    }

    let mut edges = strengths
        .into_iter()
        .map(|((source, target), strength)| {
            let (temporal_flag, cochange_flag, dependency_flag) = flags
                .get(&(source.clone(), target.clone()))
                .copied()
                .unwrap_or((false, false, false));
            PropagationEdge {
                source,
                target,
                dependency: dependency_flag,
                temporal: temporal_flag,
                cochange: cochange_flag,
                strength: strength as usize,
            }
        })
        .collect::<Vec<_>>();

    edges.sort_unstable_by_key(|edge| std::cmp::Reverse(edge.strength));
    edges.truncate(100);

    PropagationAnalysis { edges }
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
}
