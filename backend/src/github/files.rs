use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::client::GithubClient;

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryFile {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub size: Option<u64>,
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct GitTreeResponse {
    tree: Vec<RepositoryFile>,
    truncated: bool,
}

impl RepositoryFile {
    pub async fn fetch_tree(
        client: &GithubClient,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<Vec<Self>> {
        let endpoint = format!("/repos/{owner}/{repo}/git/trees/{branch}?recursive=1");

        let response: GitTreeResponse = client.get(&endpoint).await?;

        if response.truncated {
            eprintln!("Warning: GitHub returned a truncated file tree");
        }

        Ok(response.tree)
    }
}
