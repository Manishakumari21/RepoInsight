use std::time::Duration;

use moka::future::Cache;

pub const CACHE_TTL: Duration = Duration::from_secs(300);
pub const CACHE_CAPACITY: usize = 64;

pub struct AnalysisCache {
    inner: Cache<String, Vec<u8>>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::with_ttl(CACHE_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(CACHE_CAPACITY as u64)
                .time_to_live(ttl)
                .build(),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: &str, body: Vec<u8>) {
        self.inner.insert(key.to_owned(), body).await;
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

        assert_eq!(cache.get("owner/repo").await, Some(b"payload".to_vec()));
    }

    #[tokio::test]
    async fn cache_expires_entries_after_ttl() {
        let cache = AnalysisCache::with_ttl(Duration::from_millis(10));

        cache.insert("owner/repo", b"payload".to_vec()).await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(cache.get("owner/repo").await.is_none());
    }
}
