use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{Client, Response, StatusCode, header::HeaderMap};
use serde::de::DeserializeOwned;
use thiserror::Error;

const GITHUB_API_URL: &str = "https://api.github.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const RATE_LIMIT_MAX_RETRIES: u32 = 3;
const RATE_LIMIT_MAX_DELAY: Duration = Duration::from_secs(60);
const TRANSIENT_MAX_RETRIES: u32 = 3;

#[derive(Debug, Error)]
pub enum GithubError {
    #[error("GitHub API request failed: HTTP {status}: {body}")]
    Api { status: StatusCode, body: String },

    #[error("GitHub API rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("failed to request GitHub API: {url}: {source}")]
    Network { url: String, source: reqwest::Error },

    #[error("failed to parse GitHub API response: {0}")]
    Deserialize(#[from] reqwest::Error),

    #[error("failed to create GitHub HTTP client: {0}")]
    Client(#[source] reqwest::Error),
}

#[derive(Clone)]
pub struct GithubClient {
    client: Client,
    authenticated: bool,
}

impl GithubClient {
    pub fn new() -> Result<Self, GithubError> {
        let mut builder = Client::builder()
            .user_agent("RepoInsight")
            .timeout(REQUEST_TIMEOUT);

        let token = std::env::var("GITHUB_TOKEN")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let mut authenticated = false;

        if let Some(token) = &token {
            if let Ok(header_value) =
                reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            {
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(reqwest::header::AUTHORIZATION, header_value);
                builder = builder.default_headers(headers);
                authenticated = true;
            }
        } else {
            eprintln!(
                "Warning: GITHUB_TOKEN is not set; unauthenticated requests are \
                 limited to 60 GitHub API calls per hour"
            );
        }

        let client = builder.build().map_err(GithubError::Client)?;

        Ok(Self {
            client,
            authenticated,
        })
    }

    pub async fn get<T>(&self, endpoint: &str) -> Result<T, GithubError>
    where
        T: DeserializeOwned,
    {
        let url = format!("{GITHUB_API_URL}{endpoint}");

        self.get_url(&url).await
    }

    pub async fn get_url<T>(&self, url: &str) -> Result<T, GithubError>
    where
        T: DeserializeOwned,
    {
        let mut transient_retries = 0_u32;

        for rate_attempt in 0..=RATE_LIMIT_MAX_RETRIES {
            let response = match self.client.get(url).send().await {
                Ok(response) => response,
                Err(source) => {
                    if Self::is_transient_send(&source) && transient_retries < TRANSIENT_MAX_RETRIES
                    {
                        transient_retries += 1;
                        tokio::time::sleep(Self::transient_delay(transient_retries)).await;
                        continue;
                    }

                    return Err(GithubError::Network {
                        url: url.to_owned(),
                        source,
                    });
                }
            };

            let status = response.status();
            let headers = response.headers().clone();

            if Self::is_rate_limited(status, &headers) {
                let delay = Self::rate_limit_delay(&headers).unwrap_or(Duration::from_secs(2));

                if rate_attempt < RATE_LIMIT_MAX_RETRIES && delay <= RATE_LIMIT_MAX_DELAY {
                    tokio::time::sleep(delay).await;
                    continue;
                }

                let message = if self.authenticated {
                    "GitHub is currently throttling this account; retry again shortly".to_string()
                } else {
                    "unauthenticated GitHub access is limited to 60 requests per hour; \
                     set GITHUB_TOKEN or wait for the hourly reset"
                        .to_string()
                };

                return Err(GithubError::RateLimit { message });
            }

            match Self::parse_response(response).await {
                Ok(value) => return Ok(value),
                Err(GithubError::Deserialize(source))
                    if Self::is_transient_decode(&source)
                        && transient_retries < TRANSIENT_MAX_RETRIES =>
                {
                    transient_retries += 1;
                    tokio::time::sleep(Self::transient_delay(transient_retries)).await;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!()
    }

    fn is_transient_send(error: &reqwest::Error) -> bool {
        error.is_timeout() || error.is_connect() || error.is_body() || error.is_request()
    }

    fn is_transient_decode(error: &reqwest::Error) -> bool {
        if error.is_timeout() || error.is_connect() || error.is_body() {
            return true;
        }

        let message = format!("{error}").to_ascii_lowercase();

        message.contains("stream")
            || message.contains("body")
            || message.contains("connection")
            || message.contains("timeout")
            || message.contains("reset")
            || message.contains("incomplete")
            || message.contains("eof")
    }

    fn transient_delay(retries: u32) -> Duration {
        let millis = 300_u64.saturating_mul(1_u64 << retries.saturating_sub(1).min(4));

        Duration::from_millis(millis.min(5_000))
    }

    fn is_rate_limited(status: StatusCode, headers: &HeaderMap) -> bool {
        if status == StatusCode::TOO_MANY_REQUESTS {
            return true;
        }

        if status == StatusCode::FORBIDDEN {
            return headers
                .get("x-ratelimit-remaining")
                .and_then(|value| value.to_str().ok())
                .map(|value| value.trim() == "0")
                .unwrap_or(false);
        }

        false
    }

    fn rate_limit_delay(headers: &HeaderMap) -> Option<Duration> {
        if let Some(seconds) = headers
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
        {
            return Some(Duration::from_secs(seconds));
        }

        headers
            .get("x-ratelimit-reset")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<i64>().ok())
            .map(|reset| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or(0);

                Duration::from_secs((reset - now).max(0) as u64)
            })
    }

    async fn parse_response<T>(response: Response) -> Result<T, GithubError>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read GitHub error response".to_string());

            return Err(GithubError::Api { status, body });
        }

        response.json::<T>().await.map_err(GithubError::Deserialize)
    }
}
