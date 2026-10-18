use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn origin_allowed(origin: Option<&str>, allowed_origins: &[String]) -> bool {
    match origin {
        Some(origin) => allowed_origins.iter().any(|allowed| allowed == origin),
        None => false,
    }
}

pub fn issue_token(session_hash: &str, secret: &str) -> CsrfToken {
    let mut nonce = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(session_hash.as_bytes());
    mac.update(&nonce);
    let mac_bytes = mac.finalize().into_bytes();

    let mut token_bytes = Vec::with_capacity(64);
    token_bytes.extend_from_slice(&nonce);
    token_bytes.extend_from_slice(&mac_bytes);
    CsrfToken(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes))
}

pub fn validate_token(session_hash: &str, secret: &str, supplied: &str) -> bool {
    let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(supplied) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    if decoded.len() != 64 {
        return false;
    }
    let (nonce, supplied_mac) = decoded.split_at(32);

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(session_hash.as_bytes());
    mac.update(nonce);
    mac.verify_slice(supplied_mac).is_ok()
}

pub fn validate_request(
    session_hash: &str,
    secret: &str,
    origin: Option<&str>,
    supplied_csrf: Option<&str>,
    allowed_origins: &[String],
) -> bool {
    if !origin_allowed(origin, allowed_origins) {
        return false;
    }
    match supplied_csrf {
        Some(token) => validate_token(session_hash, secret, token),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_origin_allowed() {
        let allowed = vec!["https://app.example".to_owned()];
        assert!(origin_allowed(Some("https://app.example"), &allowed));
    }

    #[test]
    fn untrusted_origin_rejected() {
        let allowed = vec!["https://app.example".to_owned()];
        assert!(!origin_allowed(Some("https://evil.example"), &allowed));
    }

    #[test]
    fn missing_origin_rejected() {
        let allowed = vec!["https://app.example".to_owned()];
        assert!(!origin_allowed(None, &allowed));
    }

    #[test]
    fn same_session_token_validates() {
        let secret = "secret";
        let session = "session-hash";
        let token = issue_token(session, secret);
        assert!(validate_token(session, secret, token.as_str()));
    }

    #[test]
    fn wrong_session_rejected() {
        let secret = "secret";
        let token = issue_token("session-a", secret);
        assert!(!validate_token("session-b", secret, token.as_str()));
    }

    #[test]
    fn modified_nonce_rejected() {
        let secret = "secret";
        let session = "session-hash";
        let token = issue_token(session, secret);
        let mut decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(token.as_str())
            .unwrap();
        decoded[0] ^= 1;
        let tampered = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(decoded);
        assert!(!validate_token(session, secret, &tampered));
    }

    #[test]
    fn wrong_token_rejected() {
        let secret = "secret";
        assert!(!validate_token("session", secret, "not-the-token"));
    }

    #[test]
    fn request_requires_trusted_origin_and_valid_token() {
        let allowed = vec!["https://app.example".to_owned()];
        let secret = "secret";
        let session = "session-hash";
        let token = issue_token(session, secret);
        assert!(validate_request(
            session,
            secret,
            Some("https://app.example"),
            Some(token.as_str()),
            &allowed
        ));
    }

    #[test]
    fn request_rejects_missing_origin() {
        let allowed = vec!["https://app.example".to_owned()];
        let token = issue_token("session", "secret");
        assert!(!validate_request(
            "session",
            "secret",
            None,
            Some(token.as_str()),
            &allowed
        ));
    }

    #[test]
    fn request_rejects_untrusted_origin() {
        let allowed = vec!["https://app.example".to_owned()];
        let token = issue_token("session", "secret");
        assert!(!validate_request(
            "session",
            "secret",
            Some("https://evil.example"),
            Some(token.as_str()),
            &allowed
        ));
    }

    #[test]
    fn request_rejects_missing_token() {
        let allowed = vec!["https://app.example".to_owned()];
        assert!(!validate_request(
            "session",
            "secret",
            Some("https://app.example"),
            None,
            &allowed
        ));
    }

    #[test]
    fn request_rejects_wrong_session_token() {
        let allowed = vec!["https://app.example".to_owned()];
        let token = issue_token("session-a", "secret");
        assert!(!validate_request(
            "session-b",
            "secret",
            Some("https://app.example"),
            Some(token.as_str()),
            &allowed
        ));
    }

    #[test]
    fn safe_methods_not_blocked_by_origin_check_only() {
        // Origin policy is only for state-changing requests; safe methods are not represented here.
        let allowed = vec!["https://app.example".to_owned()];
        assert!(origin_allowed(Some("https://app.example"), &allowed));
    }
}
