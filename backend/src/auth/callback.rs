use crate::config::AppConfig;

use super::transaction::{OAuthTransaction, OAuthTransactionError, OAuthTransactionStore};

#[derive(Debug)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug)]
pub enum CallbackError {
    MissingCode,
    MissingState,
    InvalidState,
    ExpiredState,
    ReplayedState,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallbackError::MissingCode => write!(f, "missing authorization code"),
            CallbackError::MissingState => write!(f, "missing oauth state"),
            CallbackError::InvalidState => write!(f, "invalid oauth state"),
            CallbackError::ExpiredState => write!(f, "expired oauth state"),
            CallbackError::ReplayedState => write!(f, "oauth state already used"),
        }
    }
}

impl std::error::Error for CallbackError {}

pub fn validate_params(
    code: Option<&str>,
    state: Option<&str>,
) -> Result<CallbackParams, CallbackError> {
    let code = code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingCode)?;
    let state = state
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CallbackError::MissingState)?;
    Ok(CallbackParams {
        code: code.to_owned(),
        state: state.to_owned(),
    })
}

pub fn validate_query_values(
    code: &[String],
    state: &[String],
) -> Result<CallbackParams, CallbackError> {
    if code.len() != 1 {
        return Err(CallbackError::MissingCode);
    }
    if state.len() != 1 {
        return Err(CallbackError::MissingState);
    }
    validate_params(Some(&code[0]), Some(&state[0]))
}

pub fn consume_transaction(
    store: &OAuthTransactionStore,
    params: &CallbackParams,
) -> Result<OAuthTransaction, CallbackError> {
    match store.consume(&params.state, std::time::Instant::now()) {
        Ok(transaction) => Ok(transaction),
        Err(OAuthTransactionError::UnknownState) => Err(CallbackError::InvalidState),
        Err(OAuthTransactionError::Expired) => Err(CallbackError::ExpiredState),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rejects_missing_code() {
        assert!(matches!(
            validate_params(None, Some("state")),
            Err(CallbackError::MissingCode)
        ));
    }

    #[test]
    fn rejects_missing_state() {
        assert!(matches!(
            validate_params(Some("code"), None),
            Err(CallbackError::MissingState)
        ));
    }

    #[test]
    fn rejects_empty_values() {
        assert!(matches!(
            validate_params(Some(" "), Some("state")),
            Err(CallbackError::MissingCode)
        ));
        assert!(matches!(
            validate_params(Some("code"), Some(" ")),
            Err(CallbackError::MissingState)
        ));
    }

    #[test]
    fn consumes_matching_state_once() {
        let store = OAuthTransactionStore::new(Duration::from_secs(60));
        let transaction = store.create();
        let params = validate_params(Some("code"), Some(&transaction.state)).unwrap();
        let consumed = consume_transaction(&store, &params).unwrap();
        assert_eq!(consumed.state, transaction.state);
        let replay = consume_transaction(&store, &params);
        assert!(matches!(replay, Err(CallbackError::InvalidState)));
    }

    #[test]
    fn rejects_duplicate_query_parameters() {
        let result = validate_query_values(
            &["code-a".to_owned(), "code-b".to_owned()],
            &["state".to_owned()],
        );
        assert!(matches!(result, Err(CallbackError::MissingCode)));

        let result = validate_query_values(
            &["code".to_owned()],
            &["state-a".to_owned(), "state-b".to_owned()],
        );
        assert!(matches!(result, Err(CallbackError::MissingState)));
    }

    #[test]
    fn errors_do_not_include_sensitive_values() {
        let error = CallbackError::InvalidState;
        assert!(!error.to_string().contains("secret"));
    }
}
