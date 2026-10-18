use serde::Deserialize;

use crate::config::AppConfig;

#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub scope: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub refresh_token_expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GitHubTokenError {
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug)]
pub enum TokenExchangeError {
    MissingConfiguration,
    RequestFailed,
    InvalidResponse,
    OAuthError(String),
}

impl std::fmt::Display for TokenExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenExchangeError::MissingConfiguration => {
                write!(f, "GitHub App token exchange is not configured")
            }
            TokenExchangeError::RequestFailed => write!(f, "GitHub token exchange request failed"),
            TokenExchangeError::InvalidResponse => write!(f, "GitHub token response was invalid"),
            TokenExchangeError::OAuthError(_) => write!(f, "GitHub authorization was rejected"),
        }
    }
}

impl std::error::Error for TokenExchangeError {}

pub async fn exchange_code_at(
    config: &AppConfig,
    code: &str,
    code_verifier: &str,
    base_url: &str,
) -> Result<GitHubTokenResponse, TokenExchangeError> {
    let client_id = config
        .github_app_client_id
        .as_deref()
        .ok_or(TokenExchangeError::MissingConfiguration)?;
    let client_secret = std::env::var("GITHUB_APP_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(TokenExchangeError::MissingConfiguration)?;
    let redirect_uri = config
        .github_app_callback_url
        .as_deref()
        .ok_or(TokenExchangeError::MissingConfiguration)?;

    let client = reqwest::Client::builder()
        .user_agent("RepoInsight")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| TokenExchangeError::RequestFailed)?;

    let url = format!(
        "{}/login/oauth/access_token",
        base_url.trim_end_matches('/')
    );
    let response = client
        .post(&url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", &client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|_| TokenExchangeError::RequestFailed)?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|_| TokenExchangeError::RequestFailed)?;

    if !status.is_success() {
        return Err(TokenExchangeError::RequestFailed);
    }

    if let Ok(error) = serde_json::from_str::<GitHubTokenError>(&body) {
        if error.error.is_some() {
            return Err(TokenExchangeError::OAuthError(
                error.error.unwrap_or_default(),
            ));
        }
    }

    let token: GitHubTokenResponse =
        serde_json::from_str(&body).map_err(|_| TokenExchangeError::InvalidResponse)?;

    if token.access_token.trim().is_empty() || token.token_type.trim().is_empty() {
        return Err(TokenExchangeError::InvalidResponse);
    }

    Ok(token)
}

pub async fn exchange_code(
    config: &AppConfig,
    code: &str,
    code_verifier: &str,
) -> Result<GitHubTokenResponse, TokenExchangeError> {
    exchange_code_at(config, code, code_verifier, "https://github.com").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::collections::HashMap;

    fn config() -> AppConfig {
        let mut values = HashMap::new();
        values.insert("GITHUB_APP_CLIENT_ID".to_owned(), "client-id".to_owned());
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "https://example.com/callback".to_owned(),
        );
        AppConfig::from_map(&values).unwrap()
    }

    #[test]
    fn parses_successful_response() {
        let body = r#"{"access_token":"ghu_test","token_type":"bearer","scope":"","expires_in":28800,"refresh_token":"ghr_test","refresh_token_expires_in":15897600}"#;
        let token: GitHubTokenResponse = serde_json::from_str(body).unwrap();
        assert_eq!(token.access_token, "ghu_test");
        assert_eq!(token.expires_in, Some(28800));
    }

    #[test]
    fn detects_oauth_error() {
        let body = r#"{"error":"bad_verification_code","error_description":"bad code"}"#;
        let error: GitHubTokenError = serde_json::from_str(body).unwrap();
        assert_eq!(error.error.as_deref(), Some("bad_verification_code"));
    }

    #[test]
    fn missing_access_token_is_invalid() {
        let body = r#"{"token_type":"bearer"}"#;
        let result: Result<GitHubTokenResponse, _> = serde_json::from_str(body);
        assert!(result.is_err());
    }

    #[test]
    fn non_success_http_response_is_request_failed() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });

        let mut values = HashMap::new();
        values.insert("GITHUB_APP_CLIENT_ID".to_owned(), "client-id".to_owned());
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "https://example.com/callback".to_owned(),
        );
        let config = AppConfig::from_map(&values).unwrap();
        unsafe {
            std::env::set_var("GITHUB_APP_CLIENT_SECRET", "secret");
        }
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(exchange_code_at(
            &config,
            "code",
            "verifier",
            &format!("http://{}", addr),
        ));
        unsafe {
            std::env::remove_var("GITHUB_APP_CLIENT_SECRET");
        }
        assert!(matches!(result, Err(TokenExchangeError::RequestFailed)));
        let _ = handle.join();
    }

    #[test]
    fn connection_failure_is_request_failed() {
        let mut values = HashMap::new();
        values.insert("GITHUB_APP_CLIENT_ID".to_owned(), "client-id".to_owned());
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "https://example.com/callback".to_owned(),
        );
        let config = AppConfig::from_map(&values).unwrap();
        unsafe {
            std::env::set_var("GITHUB_APP_CLIENT_SECRET", "secret");
        }
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(exchange_code_at(
            &config,
            "code",
            "verifier",
            "http://127.0.0.1:1",
        ));
        unsafe {
            std::env::remove_var("GITHUB_APP_CLIENT_SECRET");
        }
        assert!(matches!(result, Err(TokenExchangeError::RequestFailed)));
    }

    #[test]
    fn missing_configuration_is_safe() {
        let config = AppConfig::from_map(&HashMap::new()).unwrap();
        // No env secret set in test.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(exchange_code(&config, "code", "verifier"));
        assert!(matches!(
            result,
            Err(TokenExchangeError::MissingConfiguration)
        ));
    }
}
