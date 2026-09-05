use anyhow::{Context, Result};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;

const GITHUB_API_URL: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GithubClient {
    client: Client,
}

impl GithubClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("RepoInsight")
            .build()
            .context("failed to create GitHub HTTP client")?;

        Ok(Self { client })
    }

    pub async fn get<T>(&self, endpoint: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{GITHUB_API_URL}{endpoint}");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to request GitHub API: {url}"))?;

        Self::parse_response(response).await
    }

    async fn parse_response<T>(response: Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read GitHub error response".to_string());

            anyhow::bail!("GitHub API request failed: HTTP {status}: {body}");
        }

        response
            .json::<T>()
            .await
            .context("failed to parse GitHub API response")
    }
}
