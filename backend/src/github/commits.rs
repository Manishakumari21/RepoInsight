use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::client::GithubClient;

#[derive(Debug, Deserialize, Serialize)]
pub struct Commit {
    pub sha: String,
    pub message: String,
    pub author: Option<CommitAuthor>,
    pub date: Option<String>,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubCommit {
    sha: String,
    commit: CommitDetails,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct CommitDetails {
    message: String,
    author: Option<CommitAuthor>,
}

impl From<GithubCommit> for Commit {
    fn from(commit: GithubCommit) -> Self {
        Self {
            sha: commit.sha,
            message: commit.commit.message,
            date: commit
                .commit
                .author
                .as_ref()
                .and_then(|author| author.date.clone()),
            author: commit.commit.author,
            url: commit.html_url,
        }
    }
}

impl Commit {
    pub async fn fetch(client: &GithubClient, owner: &str, repo: &str) -> Result<Vec<Self>> {
        let endpoint = format!("/repos/{owner}/{repo}/commits?per_page=30");

        let commits: Vec<GithubCommit> = client.get(&endpoint).await?;

        Ok(commits.into_iter().map(Self::from).collect())
    }
}
