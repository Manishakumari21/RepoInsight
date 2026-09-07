use std::collections::{HashMap, HashSet};

use crate::github::commits::Commit;

use super::models::HistoryAnalysis;

#[derive(Debug, Clone)]
pub struct HistoryMetrics {
    pub total_commits: usize,
    pub active_contributors: usize,
    pub changed_files: usize,
    pub total_additions: u64,
    pub total_deletions: u64,
    pub total_churn: u64,
    pub file_changes: HashMap<String, FileHistory>,
}

#[derive(Debug, Clone, Default)]
pub struct FileHistory {
    pub change_count: usize,
    pub additions: u64,
    pub deletions: u64,
    pub churn: u64,
}

pub fn analyze(commits: &[Commit]) -> (HistoryAnalysis, HistoryMetrics) {
    let mut contributors = HashSet::new();
    let mut file_history: HashMap<String, FileHistory> = HashMap::new();

    let mut total_additions = 0_u64;
    let mut total_deletions = 0_u64;

    for commit in commits {
        if let Some(author) = &commit.author {
            let contributor = contributor_key(author);

            if !contributor.is_empty() {
                contributors.insert(contributor);
            }
        }

        total_additions = total_additions.saturating_add(commit.stats.additions);
        total_deletions = total_deletions.saturating_add(commit.stats.deletions);

        for file in &commit.files {
            let entry = file_history.entry(file.filename.clone()).or_default();

            entry.change_count = entry.change_count.saturating_add(1);
            entry.additions = entry.additions.saturating_add(file.additions);
            entry.deletions = entry.deletions.saturating_add(file.deletions);
            entry.churn = entry
                .churn
                .saturating_add(file.additions.saturating_add(file.deletions));
        }
    }

    let total_churn = total_additions.saturating_add(total_deletions);

    let analysis = HistoryAnalysis {
        total_commits: commits.len(),
        active_contributors: contributors.len(),
        changed_files: file_history.len(),
    };

    let metrics = HistoryMetrics {
        total_commits: commits.len(),
        active_contributors: contributors.len(),
        changed_files: file_history.len(),
        total_additions,
        total_deletions,
        total_churn,
        file_changes: file_history,
    };

    (analysis, metrics)
}

fn contributor_key(author: &crate::github::commits::CommitAuthor) -> String {
    author
        .email
        .as_deref()
        .filter(|email| !email.is_empty())
        .unwrap_or(&author.name)
        .trim()
        .to_owned()
}
