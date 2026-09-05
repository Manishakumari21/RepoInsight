use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::client::GithubClient;

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub default_branch: String,
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub open_issues_count: u64,
    pub size: u64,
}

impl Repository {
    pub async fn fetch(client: &GithubClient, owner: &str, repo: &str) -> Result<Self> {
        let endpoint = format!("/repos/{owner}/{repo}");

        client.get(&endpoint).await
    }
}
