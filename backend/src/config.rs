use std::collections::HashMap;
use std::fmt;

/// Minimal configuration foundation for RepoInsight.
///
/// This phase only parses settings that are already used by the application.
/// Future authentication/database settings are intentionally not required here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub local_api_enabled: bool,
    pub github_token_present: bool,
    pub allowed_origins: Vec<String>,
    pub github_app_client_id: Option<String>,
    pub github_app_client_id_present: bool,
    pub github_app_client_secret_present: bool,
    pub github_app_callback_url: Option<String>,
    pub database_url_present: bool,
    pub token_encryption_key_present: bool,
    pub csrf_secret_present: bool,
    pub auth_persistence_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    setting: String,
    message: String,
}

impl ConfigError {
    pub fn new(setting: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            setting: setting.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never include secret values in configuration errors.
        write!(
            f,
            "invalid configuration for {}: {}",
            self.setting, self.message
        )
    }
}

impl std::error::Error for ConfigError {}

fn validate_callback_url(url: &str) -> Result<(), ConfigError> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| ConfigError::new("GITHUB_APP_CALLBACK_URL", "must be an absolute URL"))?;

    if scheme != "http" && scheme != "https" {
        return Err(ConfigError::new(
            "GITHUB_APP_CALLBACK_URL",
            "must use http or https",
        ));
    }

    let host = rest
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    if host.is_empty() {
        return Err(ConfigError::new(
            "GITHUB_APP_CALLBACK_URL",
            "must include a host",
        ));
    }

    let is_local = host == "localhost" || host == "127.0.0.1" || host == "::1";
    if scheme == "http" && !is_local {
        return Err(ConfigError::new(
            "GITHUB_APP_CALLBACK_URL",
            "must use https outside local development",
        ));
    }

    Ok(())
}

impl AppConfig {
    /// Parse configuration from an explicit map. This keeps tests deterministic
    /// and avoids mutating the process environment.
    pub fn from_map(values: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let host = values
            .get("HOST")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "127.0.0.1".to_owned());

        let port_raw = values
            .get("PORT")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "3000".to_owned());

        let port = port_raw
            .parse::<u16>()
            .map_err(|_| ConfigError::new("PORT", "must be a valid TCP port number"))?;

        let local_api_enabled = values
            .get("LOCAL_API_ENABLED")
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                normalized != "0" && normalized != "false" && normalized != "no"
            })
            .unwrap_or(true);

        let github_token_present = values
            .get("GITHUB_TOKEN")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let allowed_origins = values
            .get("ALLOWED_ORIGINS")
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|origin| !origin.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        let github_app_client_id = values
            .get("GITHUB_APP_CLIENT_ID")
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let github_app_client_id_present = github_app_client_id.is_some();

        let github_app_client_secret_present = values
            .get("GITHUB_APP_CLIENT_SECRET")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let database_url_present = values
            .get("DATABASE_URL")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let token_encryption_key_present = values
            .get("TOKEN_ENCRYPTION_KEY")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let csrf_secret_present = values
            .get("CSRF_SECRET")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        let auth_persistence_enabled = values
            .get("AUTH_PERSISTENCE_ENABLED")
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                normalized == "1" || normalized == "true" || normalized == "yes"
            })
            .unwrap_or(false);

        if auth_persistence_enabled {
            if !database_url_present {
                return Err(ConfigError::new(
                    "DATABASE_URL",
                    "required when AUTH_PERSISTENCE_ENABLED is enabled",
                ));
            }
            if !token_encryption_key_present {
                return Err(ConfigError::new(
                    "TOKEN_ENCRYPTION_KEY",
                    "required when AUTH_PERSISTENCE_ENABLED is enabled",
                ));
            }
            if !csrf_secret_present {
                return Err(ConfigError::new(
                    "CSRF_SECRET",
                    "required when AUTH_PERSISTENCE_ENABLED is enabled",
                ));
            }
        }

        let github_app_callback_url = match values.get("GITHUB_APP_CALLBACK_URL") {
            None => None,
            Some(value) if value.trim().is_empty() => None,
            Some(value) => {
                let url = value.trim();
                validate_callback_url(url)?;
                Some(url.to_owned())
            }
        };

        Ok(Self {
            host,
            port,
            local_api_enabled,
            github_token_present,
            allowed_origins,
            github_app_client_id,
            github_app_client_id_present,
            github_app_client_secret_present,
            github_app_callback_url,
            database_url_present,
            token_encryption_key_present,
            csrf_secret_present,
            auth_persistence_enabled,
        })
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let mut values = HashMap::new();
        for key in [
            "HOST",
            "PORT",
            "LOCAL_API_ENABLED",
            "GITHUB_TOKEN",
            "ALLOWED_ORIGINS",
            "GITHUB_APP_CLIENT_ID",
            "GITHUB_APP_CLIENT_SECRET",
            "GITHUB_APP_CALLBACK_URL",
            "DATABASE_URL",
            "TOKEN_ENCRYPTION_KEY",
            "CSRF_SECRET",
            "AUTH_PERSISTENCE_ENABLED",
        ] {
            if let Ok(value) = std::env::var(key) {
                values.insert(key.to_owned(), value);
            }
        }
        Self::from_map(&values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let config = AppConfig::from_map(&HashMap::new()).unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert!(config.local_api_enabled);
        assert!(!config.github_token_present);
        assert!(config.allowed_origins.is_empty());
    }

    #[test]
    fn malformed_port_is_rejected_without_echoing_value() {
        let mut values = HashMap::new();
        values.insert("PORT".to_owned(), "not-a-port".to_owned());

        let error = AppConfig::from_map(&values).unwrap_err();

        assert_eq!(error.setting, "PORT");
        assert!(!error.to_string().contains("not-a-port"));
    }

    #[test]
    fn github_token_presence_is_boolean_only() {
        let mut values = HashMap::new();
        values.insert("GITHUB_TOKEN".to_owned(), "secret-value".to_owned());

        let config = AppConfig::from_map(&values).unwrap();

        assert!(config.github_token_present);
    }

    #[test]
    fn missing_github_app_settings_are_valid() {
        let config = AppConfig::from_map(&HashMap::new()).unwrap();

        assert!(!config.github_app_client_id_present);
        assert!(!config.github_app_client_secret_present);
        assert!(config.github_app_callback_url.is_none());
    }

    #[test]
    fn valid_callback_url_is_accepted() {
        let mut values = HashMap::new();
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "https://example.com/auth/github/callback".to_owned(),
        );

        let config = AppConfig::from_map(&values).unwrap();

        assert_eq!(
            config.github_app_callback_url.as_deref(),
            Some("https://example.com/auth/github/callback")
        );
    }

    #[test]
    fn malformed_callback_url_is_rejected_without_echoing_value() {
        let mut values = HashMap::new();
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "not-a-url-with-secret".to_owned(),
        );

        let error = AppConfig::from_map(&values).unwrap_err();

        assert_eq!(error.setting, "GITHUB_APP_CALLBACK_URL");
        assert!(!error.to_string().contains("not-a-url-with-secret"));
    }

    #[test]
    fn http_callback_url_is_allowed_only_for_local_hosts() {
        let mut values = HashMap::new();
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "http://localhost:3000/callback".to_owned(),
        );

        let config = AppConfig::from_map(&values).unwrap();
        assert!(config.github_app_callback_url.is_some());

        let mut values = HashMap::new();
        values.insert(
            "GITHUB_APP_CALLBACK_URL".to_owned(),
            "http://example.com/callback".to_owned(),
        );

        let error = AppConfig::from_map(&values).unwrap_err();
        assert_eq!(error.setting, "GITHUB_APP_CALLBACK_URL");
    }

    #[test]
    fn allowed_origins_are_split_and_trimmed() {
        let mut values = HashMap::new();
        values.insert(
            "ALLOWED_ORIGINS".to_owned(),
            " https://a.example,https://b.example ".to_owned(),
        );

        let config = AppConfig::from_map(&values).unwrap();

        assert_eq!(
            config.allowed_origins,
            vec![
                "https://a.example".to_owned(),
                "https://b.example".to_owned()
            ]
        );
    }
}
