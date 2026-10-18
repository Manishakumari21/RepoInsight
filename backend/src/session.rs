use base64::Engine;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: Uuid,
    pub github_user_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug)]
pub enum SessionError {
    NotFound,
    Expired,
    Revoked,
    Database,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::NotFound => write!(f, "session not found"),
            SessionError::Expired => write!(f, "session expired"),
            SessionError::Revoked => write!(f, "session revoked"),
            SessionError::Database => write!(f, "session database error"),
        }
    }
}

impl std::error::Error for SessionError {}

pub fn hash_session_id(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(digest)
}

pub fn generate_session_id() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

pub async fn create_session(
    pool: &PgPool,
    github_user_id: &str,
    ttl_seconds: i64,
) -> Result<(String, SessionRecord), SessionError> {
    let raw_id = generate_session_id();
    let hash = hash_session_id(&raw_id);
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::seconds(ttl_seconds);

    sqlx::query(
        "INSERT INTO sessions (id, session_hash, github_user_id, created_at, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(&hash)
    .bind(github_user_id)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|_| SessionError::Database)?;

    Ok((
        raw_id,
        SessionRecord {
            id,
            github_user_id: github_user_id.to_owned(),
            created_at: now,
            expires_at,
            revoked_at: None,
        },
    ))
}

pub async fn lookup_session(
    pool: &PgPool,
    raw_id: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<SessionRecord, SessionError> {
    let hash = hash_session_id(raw_id);
    let record = sqlx::query_as::<_, (Uuid, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, github_user_id, created_at, expires_at, revoked_at FROM sessions WHERE session_hash = $1",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| SessionError::Database)?;

    let Some((id, github_user_id, created_at, expires_at, revoked_at)) = record else {
        return Err(SessionError::NotFound);
    };

    if revoked_at.is_some() {
        return Err(SessionError::Revoked);
    }
    if now > expires_at {
        return Err(SessionError::Expired);
    }

    Ok(SessionRecord {
        id,
        github_user_id,
        created_at,
        expires_at,
        revoked_at,
    })
}

pub fn extract_session_cookie(cookie_header: &str) -> Option<String> {
    let mut found: Option<String> = None;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("__Host-session=") {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            if found.is_some() {
                return None;
            }
            found = Some(value.to_owned());
        }
    }
    found
}

pub async fn authenticated_session(
    pool: &PgPool,
    cookie_header: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<SessionRecord, SessionError> {
    let raw_id = cookie_header
        .and_then(extract_session_cookie)
        .ok_or(SessionError::NotFound)?;
    lookup_session(pool, &raw_id, now).await
}

#[derive(Debug)]
pub enum SessionPersistError {
    Database,
    Encryption,
    Configuration,
}

impl std::fmt::Display for SessionPersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionPersistError::Database => write!(f, "session persistence failed"),
            SessionPersistError::Encryption => write!(f, "token encryption failed"),
            SessionPersistError::Configuration => {
                write!(f, "session persistence is not configured")
            }
        }
    }
}

impl std::error::Error for SessionPersistError {}

pub async fn persist_authenticated_session(
    pool: &PgPool,
    github_user_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    access_expires_in: Option<u64>,
    refresh_expires_in: Option<u64>,
    key: &[u8],
) -> Result<String, SessionPersistError> {
    persist_authenticated_session_inner(
        pool,
        github_user_id,
        access_token,
        refresh_token,
        access_expires_in,
        refresh_expires_in,
        key,
        false,
    )
    .await
}

#[cfg(test)]
pub async fn persist_authenticated_session_with_fault_injection(
    pool: &PgPool,
    github_user_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    access_expires_in: Option<u64>,
    refresh_expires_in: Option<u64>,
    key: &[u8],
    force_token_insert_failure: bool,
) -> Result<String, SessionPersistError> {
    persist_authenticated_session_inner(
        pool,
        github_user_id,
        access_token,
        refresh_token,
        access_expires_in,
        refresh_expires_in,
        key,
        force_token_insert_failure,
    )
    .await
}

async fn persist_authenticated_session_inner(
    pool: &PgPool,
    github_user_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
    access_expires_in: Option<u64>,
    refresh_expires_in: Option<u64>,
    key: &[u8],
    force_token_insert_failure: bool,
) -> Result<String, SessionPersistError> {
    if key.len() != 32 {
        return Err(SessionPersistError::Configuration);
    }

    let raw_id = generate_session_id();
    let session_hash = hash_session_id(&raw_id);
    let session_uuid = uuid::Uuid::new_v4();
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::seconds(28800);

    let (access_ciphertext, access_nonce) =
        crate::crypto::encrypt_token(key, access_token.as_bytes())
            .map_err(|_| SessionPersistError::Encryption)?;
    let refresh_encrypted = if let Some(refresh) = refresh_token {
        let (ct, nonce) = crate::crypto::encrypt_token(key, refresh.as_bytes())
            .map_err(|_| SessionPersistError::Encryption)?;
        Some((ct, nonce))
    } else {
        None
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| SessionPersistError::Database)?;

    sqlx::query(
        "INSERT INTO sessions (id, session_hash, github_user_id, created_at, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(session_uuid)
    .bind(&session_hash)
    .bind(github_user_id)
    .bind(now)
    .bind(expires_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| SessionPersistError::Database)?;

    #[cfg(test)]
    if force_token_insert_failure {
        return Err(SessionPersistError::Database);
    }

    sqlx::query(
        "INSERT INTO github_tokens (session_id, encrypted_access_token, access_token_nonce, encrypted_refresh_token, refresh_token_nonce, access_token_expires_at, refresh_token_expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(session_uuid)
    .bind(&access_ciphertext)
    .bind(&access_nonce)
    .bind(refresh_encrypted.as_ref().map(|(ct, _)| ct.clone()))
    .bind(refresh_encrypted.as_ref().map(|(_, n)| n.clone()))
    .bind(access_expires_in.map(|s| now + chrono::Duration::seconds(s as i64)))
    .bind(refresh_expires_in.map(|s| now + chrono::Duration::seconds(s as i64)))
    .execute(&mut *tx)
    .await
    .map_err(|_| SessionPersistError::Database)?;

    tx.commit()
        .await
        .map_err(|_| SessionPersistError::Database)?;

    Ok(raw_id)
}

pub async fn revoke_session(pool: &PgPool, raw_id: &str) -> Result<(), SessionError> {
    let hash = hash_session_id(raw_id);
    sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE session_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .execute(pool)
    .await
    .map_err(|_| SessionError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_session_id("abc"), hash_session_id("abc"));
        assert_ne!(hash_session_id("abc"), hash_session_id("def"));
    }

    #[test]
    fn generate_session_id_nonempty() {
        let id = generate_session_id();
        assert!(!id.is_empty());
    }

    #[test]
    fn extracts_named_session_cookie() {
        let header = "foo=bar; __Host-session=abc123; baz=qux";
        assert_eq!(extract_session_cookie(header), Some("abc123".to_owned()));
    }

    #[test]
    fn rejects_missing_or_empty_session_cookie() {
        assert!(extract_session_cookie("foo=bar").is_none());
        assert!(extract_session_cookie("__Host-session=").is_none());
    }

    #[test]
    fn session_cookie_with_surrounding_whitespace() {
        let header = "  foo=bar ; __Host-session = value ; baz=qux  ";
        // Note: strict name match does not allow spaces around '='.
        assert!(extract_session_cookie(header).is_none());
    }

    #[test]
    fn session_cookie_before_and_after_unrelated() {
        assert_eq!(
            extract_session_cookie("foo=bar; __Host-session=abc; baz=qux"),
            Some("abc".to_owned())
        );
        assert_eq!(
            extract_session_cookie("__Host-session=abc; foo=bar"),
            Some("abc".to_owned())
        );
    }

    #[test]
    fn duplicate_session_cookies_rejected() {
        assert!(extract_session_cookie("__Host-session=abc; __Host-session=abc").is_none());
        assert!(extract_session_cookie("__Host-session=abc; __Host-session=def").is_none());
    }

    #[test]
    fn similar_cookie_names_rejected() {
        assert!(extract_session_cookie("X-Host-session=abc").is_none());
        assert!(extract_session_cookie("__Host-sessionX=abc").is_none());
    }

    #[test]
    fn malformed_percent_encoded_value_preserved() {
        assert_eq!(
            extract_session_cookie("__Host-session=abc%20def"),
            Some("abc%20def".to_owned())
        );
    }

    #[tokio::test]
    #[ignore]
    async fn db_session_create_stores_hash() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (raw_id, record) = create_session(&pool, "user-db", 60).await.unwrap();
        let result = async {
            let hash = hash_session_id(&raw_id);
            let stored: (String,) =
                sqlx::query_as("SELECT session_hash FROM sessions WHERE id = $1")
                    .bind(record.id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(stored.0, hash);
            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn db_session_expired_rejected() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (raw_id, record) = create_session(&pool, "user-expired", 0).await.unwrap();
        let result = async {
            let later = chrono::Utc::now() + chrono::Duration::seconds(1);
            let lookup = lookup_session(&pool, &raw_id, later).await;
            assert!(matches!(lookup, Err(SessionError::Expired)));
            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn db_session_revoked_rejected() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (raw_id, record) = create_session(&pool, "user-revoked", 60).await.unwrap();
        let result = async {
            revoke_session(&pool, &raw_id).await.unwrap();
            let now = chrono::Utc::now();
            let lookup = lookup_session(&pool, &raw_id, now).await;
            assert!(matches!(lookup, Err(SessionError::Revoked)));
            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn db_persist_transaction_rollback() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let key = vec![7u8; 32];

        let result = crate::session::persist_authenticated_session_with_fault_injection(
            &pool,
            "user-rollback-test",
            "access-token",
            Some("refresh-token"),
            Some(3600),
            Some(7200),
            &key,
            true,
        )
        .await;

        assert!(result.is_err());

        let session_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE github_user_id = $1")
                .bind("user-rollback-test")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(session_count.0, 0);

        let token_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM github_tokens WHERE session_id IN (SELECT id FROM sessions WHERE github_user_id = $1)",
        )
        .bind("user-rollback-test")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(token_count.0, 0);

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id IN (SELECT id FROM sessions WHERE github_user_id = $1)")
            .bind("user-rollback-test")
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE github_user_id = $1")
            .bind("user-rollback-test")
            .execute(&pool)
            .await;
    }

    #[tokio::test]
    #[ignore]
    async fn db_token_encrypted_storage() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (_raw_id, session) = create_session(&pool, "user-token", 60).await.unwrap();
        let key = vec![7u8; 32];
        let plaintext = b"secret-token";
        let (ciphertext, nonce) = crate::crypto::encrypt_token(&key, plaintext).unwrap();

        let result = async {
            sqlx::query(
                "INSERT INTO github_tokens (session_id, encrypted_access_token, access_token_nonce) VALUES ($1, $2, $3)",
            )
            .bind(session.id)
            .bind(&ciphertext)
            .bind(&nonce)
            .execute(&pool)
            .await
            .unwrap();

            let row: (Vec<u8>, Vec<u8>) = sqlx::query_as(
                "SELECT encrypted_access_token, access_token_nonce FROM github_tokens WHERE session_id = $1",
            )
            .bind(session.id)
            .fetch_one(&pool)
            .await
            .unwrap();

            assert_ne!(row.0, plaintext);
            let decrypted = crate::crypto::decrypt_token(&key, &row.0, &row.1).unwrap();
            assert_eq!(decrypted, plaintext);

            // Wrong key fails.
            let wrong = vec![9u8; 32];
            assert!(crate::crypto::decrypt_token(&wrong, &row.0, &row.1).is_err());

            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(session.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn db_session_revocation_idempotent() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value)
                if value.starts_with("postgres://")
                    && value.contains("127.0.0.1:5434")
                    && value.contains("/repoinsight_test") =>
            {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (raw_id, record) = create_session(&pool, "user-revoke-twice", 60)
            .await
            .unwrap();
        let result = async {
            revoke_session(&pool, &raw_id).await.unwrap();
            revoke_session(&pool, &raw_id).await.unwrap();
            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn db_session_lookup_valid() {
        let url = match std::env::var("TEST_DATABASE_URL") {
            Ok(value) if value.contains("127.0.0.1:5434") && value.contains("repoinsight_test") => {
                value
            }
            _ => return,
        };
        let pool = crate::db::init_pool(Some(&url)).await.unwrap().unwrap();
        let (raw_id, record) = create_session(&pool, "user-db2", 60).await.unwrap();
        let result = async {
            let now = chrono::Utc::now();
            let session = lookup_session(&pool, &raw_id, now).await.unwrap();
            assert_eq!(session.github_user_id, "user-db2");
            Ok::<(), sqlx::Error>(())
        }
        .await;

        let _ = sqlx::query("DELETE FROM github_tokens WHERE session_id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(record.id)
            .execute(&pool)
            .await;

        result.unwrap();
    }
}
