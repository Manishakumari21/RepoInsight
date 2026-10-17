use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

#[derive(Debug, Clone)]
pub struct OAuthTransaction {
    pub state: String,
    pub pkce: PkceChallenge,
    pub expires_at: Instant,
}

#[derive(Debug)]
pub struct OAuthTransactionStore {
    inner: Mutex<HashMap<String, OAuthTransaction>>,
    ttl: Duration,
}

impl OAuthTransactionStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn create(&self) -> OAuthTransaction {
        let state = generate_state();
        let pkce = generate_pkce();
        let transaction = OAuthTransaction {
            state: state.clone(),
            pkce,
            expires_at: Instant::now() + self.ttl,
        };

        let mut guard = self.inner.lock().expect("oauth transaction lock poisoned");
        let now = Instant::now();
        guard.retain(|_, value| value.expires_at > now);
        guard.insert(state, transaction.clone());
        transaction
    }

    pub fn consume(
        &self,
        state: &str,
        now: Instant,
    ) -> Result<OAuthTransaction, OAuthTransactionError> {
        let mut guard = self.inner.lock().expect("oauth transaction lock poisoned");
        let transaction = guard
            .remove(state)
            .ok_or(OAuthTransactionError::UnknownState)?;
        if now > transaction.expires_at {
            return Err(OAuthTransactionError::Expired);
        }
        Ok(transaction)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthTransactionError {
    UnknownState,
    Expired,
}

impl std::fmt::Display for OAuthTransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthTransactionError::UnknownState => write!(f, "unknown or already used oauth state"),
            OAuthTransactionError::Expired => write!(f, "expired oauth state"),
        }
    }
}

impl std::error::Error for OAuthTransactionError {}

pub fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_pkce() -> PkceChallenge {
    let verifier = generate_code_verifier();
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn generates_state_and_verifier() {
        let state = generate_state();
        let verifier = generate_code_verifier();
        assert!(!state.is_empty());
        assert!(!verifier.is_empty());
        assert_ne!(state, verifier);
    }

    #[test]
    fn pkce_challenge_matches_verifier() {
        let pkce = generate_pkce();
        let digest = Sha256::digest(pkce.verifier.as_bytes());
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn consumes_transaction_once() {
        let store = OAuthTransactionStore::new(Duration::from_secs(60));
        let transaction = store.create();
        let consumed = store.consume(&transaction.state, Instant::now()).unwrap();
        assert_eq!(consumed.state, transaction.state);
        assert!(store.consume(&transaction.state, Instant::now()).is_err());
    }

    #[test]
    fn rejects_unknown_state() {
        let store = OAuthTransactionStore::new(Duration::from_secs(60));
        assert!(matches!(
            store.consume("unknown", Instant::now()),
            Err(OAuthTransactionError::UnknownState)
        ));
    }

    #[test]
    fn rejects_expired_state() {
        let store = OAuthTransactionStore::new(Duration::from_secs(1));
        let transaction = store.create();
        let later = transaction.expires_at + Duration::from_secs(1);
        assert!(matches!(
            store.consume(&transaction.state, later),
            Err(OAuthTransactionError::Expired)
        ));
    }

    #[test]
    fn concurrent_consumption_allows_one_success() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(OAuthTransactionStore::new(Duration::from_secs(60)));
        let transaction = store.create();
        let state = transaction.state.clone();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let state = state.clone();
            handles.push(thread::spawn(move || {
                store.consume(&state, Instant::now()).is_ok()
            }));
        }

        let successes = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|success| *success)
            .count();

        assert_eq!(successes, 1);
    }

    #[test]
    fn errors_do_not_include_sensitive_values() {
        let error = OAuthTransactionError::UnknownState;
        let message = error.to_string();
        assert!(!message.contains("secret"));
    }
}
