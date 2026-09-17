use anyhow::Result;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::client::GithubClient;

const COMMITS_PER_PAGE: usize = 100;
const MAX_COMMITS_TO_RETRIEVE: usize = 1000;
const MAX_CONCURRENT_DETAIL_REQUESTS: usize = 8;

#[derive(Debug, Clone)]
pub struct CommitFetchConfig {
    pub per_page: usize,

    pub max_commits: usize,

    pub max_concurrent_detail_requests: usize,
}

impl Default for CommitFetchConfig {
    fn default() -> Self {
        Self {
            per_page: COMMITS_PER_PAGE,
            max_commits: MAX_COMMITS_TO_RETRIEVE,
            max_concurrent_detail_requests: MAX_CONCURRENT_DETAIL_REQUESTS,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: Option<CommitAuthor>,
    #[serde(skip)]
    pub timestamp: Option<i64>,
    pub date: Option<String>,
    pub url: String,
    pub stats: CommitStats,
    pub files: Vec<ChangedFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CommitStats {
    pub additions: u64,
    pub deletions: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedFile {
    pub filename: String,
    pub additions: u64,
    pub deletions: u64,
    pub changes: u64,
    pub status: FileChangeStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeStatus {
    Added,
    Modified,
    Removed,
    Renamed,
    Copied,
    Changed,
    Unchanged,
}

#[derive(Debug, Deserialize)]
struct GithubCommitSummary {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GithubCommit {
    sha: String,
    commit: CommitDetails,
    html_url: String,
    stats: Option<CommitStats>,
    files: Option<Vec<ChangedFile>>,
}

#[derive(Debug, Deserialize)]
struct CommitDetails {
    message: String,
    author: Option<CommitAuthor>,
}

impl From<GithubCommit> for Commit {
    fn from(commit: GithubCommit) -> Self {
        let date = commit
            .commit
            .author
            .as_ref()
            .and_then(|author| author.date.clone());

        Self {
            sha: commit.sha,
            message: commit.commit.message,
            date: date.clone(),
            timestamp: parse_iso_timestamp(&date),
            author: commit.commit.author,
            url: commit.html_url,
            stats: commit.stats.unwrap_or_default(),
            files: commit.files.unwrap_or_default(),
        }
    }
}

fn parse_iso_timestamp(date: &Option<String>) -> Option<i64> {
    date.as_deref()
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.timestamp())
}

impl Commit {
    pub async fn fetch(client: &GithubClient, owner: &str, repo: &str) -> Result<Vec<Self>> {
        Self::fetch_with(client, owner, repo, CommitFetchConfig::default()).await
    }

    pub async fn fetch_with(
        client: &GithubClient,
        owner: &str,
        repo: &str,
        config: CommitFetchConfig,
    ) -> Result<Vec<Self>> {
        let shas = list_commit_shas(client, owner, repo, &config).await?;

        let mut commits = fetch_commit_details(client, owner, repo, &shas, &config).await;

        commits.sort_by_key(|commit| commit.timestamp.unwrap_or(0));

        Ok(commits)
    }
}

async fn list_commit_shas(
    client: &GithubClient,
    owner: &str,
    repo: &str,
    config: &CommitFetchConfig,
) -> Result<Vec<String>> {
    let mut shas = Vec::new();
    let mut page = 1_u32;

    loop {
        if shas.len() >= config.max_commits {
            break;
        }

        let remaining = config.max_commits - shas.len();
        let per_page = config.per_page.min(remaining).max(1);

        let endpoint = format!("/repos/{owner}/{repo}/commits?per_page={per_page}&page={page}");

        let batch: Vec<GithubCommitSummary> = client.get(&endpoint).await?;

        if batch.is_empty() {
            break;
        }

        for summary in batch {
            shas.push(summary.sha);
        }

        if shas.len() < config.max_commits {
            page += 1;
        } else {
            break;
        }
    }

    if !shas.is_empty() && shas.len() >= config.max_commits {
        eprintln!(
            "Warning: commit history collection reached the configured maximum \
             of {} commits; results may be truncated",
            config.max_commits
        );
    }

    Ok(shas)
}

async fn fetch_commit_details(
    client: &GithubClient,
    owner: &str,
    repo: &str,
    shas: &[String],
    config: &CommitFetchConfig,
) -> Vec<Commit> {
    if shas.is_empty() {
        return Vec::new();
    }

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_detail_requests));
    let mut tasks = JoinSet::new();

    for sha in shas {
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let owner = owner.to_owned();
        let repo = repo.to_owned();
        let sha = sha.clone();

        tasks.spawn(async move {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|error| anyhow::Error::msg(format!("failed to acquire semaphore: {error}")))?;

            let endpoint = format!("/repos/{owner}/{repo}/commits/{sha}");

            client
                .get::<GithubCommit>(&endpoint)
                .await
                .map(Commit::from)
                .map_err(anyhow::Error::from)
        });
    }

    let mut by_sha: HashMap<String, Commit> = HashMap::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(commit)) => {
                by_sha.insert(commit.sha.clone(), commit);
            }
            Ok(Err(error)) => {
                eprintln!("Warning: failed to fetch commit details: {error}");
            }
            Err(error) => {
                eprintln!("Warning: commit detail task failed to join: {error}");
            }
        }
    }

    shas.iter().filter_map(|sha| by_sha.remove(sha)).collect()
}
