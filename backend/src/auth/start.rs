use reqwest::Url;

use crate::config::AppConfig;

use super::transaction::OAuthTransactionStore;

#[derive(Debug)]
pub enum AuthStartError {
    MissingConfiguration,
    InvalidConfiguration,
}

impl std::fmt::Display for AuthStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthStartError::MissingConfiguration => {
                write!(f, "GitHub App authorization is not configured")
            }
            AuthStartError::InvalidConfiguration => {
                write!(f, "GitHub App authorization configuration is invalid")
            }
        }
    }
}

impl std::error::Error for AuthStartError {}

pub fn authorization_url(
    config: &AppConfig,
    store: &OAuthTransactionStore,
) -> Result<String, AuthStartError> {
    let client_id = config
        .github_app_client_id
        .as_deref()
        .ok_or(AuthStartError::MissingConfiguration)?;

    let callback_url = config
        .github_app_callback_url
        .as_deref()
        .ok_or(AuthStartError::MissingConfiguration)?;

    let transaction = store.create();
    let mut url = Url::parse("https://github.com/login/oauth/authorize")
        .map_err(|_| AuthStartError::InvalidConfiguration)?;
    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", callback_url)
        .append_pair("state", &transaction.state)
        .append_pair("code_challenge", &transaction.pkce.challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::collections::HashMap;
    use std::time::Duration;

    fn config_with_callback() -> AppConfig {
        let mut values = HashMap::new();
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "https://example.com/callback".to_owned(),
        );
        values.insert("GITHUB_APP_CLIENT_ID".to_owned(), "client-id".to_owned());
        AppConfig::from_map(&values).unwrap()
    }

    #[test]
    fn builds_authorization_url() {
        let config = config_with_callback();
        let store = OAuthTransactionStore::new(Duration::from_secs(60));
        let url = authorization_url(&config, &store).unwrap();
        assert!(url.starts_with("https://github.com/login/oauth/authorize"));
        assert!(url.contains("client_id=client-id"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fexample.com%2Fcallback"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn missing_configuration_returns_safe_error() {
        let config = AppConfig::from_map(&HashMap::new()).unwrap();
        let store = OAuthTransactionStore::new(Duration::from_secs(60));
        let error = authorization_url(&config, &store).unwrap_err();
        assert!(!error.to_string().contains("secret"));
    }
}
