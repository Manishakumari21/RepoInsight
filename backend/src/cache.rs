use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

pub const CACHE_TTL: Duration = Duration::from_secs(300);
pub const CACHE_CAPACITY: usize = 64;

struct Entry {
    created_at: Instant,
    body: Vec<u8>,
}

pub struct AnalysisCache {
    ttl: Duration,
    inner: Mutex<HashMap<String, Entry>>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::with_ttl(CACHE_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut inner = self.inner.lock().await;

        let entry = inner.get(key)?;

        if entry.created_at.elapsed() >= self.ttl {
            inner.remove(key);
            return None;
        }

        Some(entry.body.clone())
    }

    pub async fn insert(&self, key: &str, body: Vec<u8>) {
        let mut inner = self.inner.lock().await;

        if inner.len() >= CACHE_CAPACITY {
            let stale_key = inner
                .iter()
                .find(|(_, entry)| entry.created_at.elapsed() >= self.ttl)
                .map(|(key, _)| key.clone());

            match stale_key {
                Some(stale_key) => {
                    inner.remove(&stale_key);
                }
                None => {
                    if let Some(any_key) = inner.keys().next().cloned() {
                        inner.remove(&any_key);
                    }
                }
            }
        }

        inner.insert(
            key.to_owned(),
            Entry {
                created_at: Instant::now(),
                body,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_misses_then_round_trips() {
        let cache = AnalysisCache::new();

        assert!(cache.get("owner/repo").await.is_none());

        cache.insert("owner/repo", b"payload".to_vec()).await;

        assert_eq!(
            cache.get("owner/repo").await,
            Some(b"payload".to_vec())
        );
    }

    #[tokio::test]
    async fn cache_expires_entries_after_ttl() {
        let cache = AnalysisCache::with_ttl(Duration::from_millis(10));

        cache.insert("owner/repo", b"payload".to_vec()).await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(cache.get("owner/repo").await.is_none());
    }
}