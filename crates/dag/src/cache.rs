//! Deterministic memoization for formula evaluations.
//!
//! The [`FormulaCache`] trait lets callers memoize the result of evaluating a
//! subgraph so that identical inputs produce a cache hit rather than a
//! recomputation. This is safe because every formula in CASIROS is a pure
//! function of its inputs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::graph::NodeId;

/// A key that uniquely identifies a cacheable evaluation point.
///
/// Two [`CacheKey`] values are equal when the same formula node is evaluated
/// with the same input values, guaranteeing that a cache hit returns the
/// correct result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// The formula node being evaluated.
    pub node: NodeId,

    /// The input values to that node, sorted by source node identifier.
    ///
    /// A `Vec` is used instead of a `HashMap` because `HashMap` does not
    /// implement `Hash`. The vector is sorted by `NodeId` so that two keys
    /// with the same inputs compare equal regardless of insertion order.
    pub inputs: Vec<(NodeId, Decimal)>,
}

/// Result of a cached formula evaluation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvaluationResult {
    /// The computed value.
    pub value: Decimal,
}

/// Storage backend for memoized formula results.
///
/// Implementations must be thread-safe. The default in-memory implementation
/// is suitable for single-process deployments; a `Redis`-backed implementation
/// can share a cache across multiple API server instances.
#[async_trait]
pub trait FormulaCache: Send + Sync {
    /// Returns the cached result for a key, if one exists.
    async fn get(&self, key: &CacheKey) -> Option<EvaluationResult>;

    /// Stores a result for a key, replacing any previous value.
    async fn put(&self, key: CacheKey, value: EvaluationResult);
}

/// In-memory formula cache backed by a `HashMap`.
///
/// Entries are retained until the process exits. This is suitable for
/// single-process deployments and for testing.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFormulaCache {
    /// The underlying key-value store, guarded by a mutex.
    entries: Arc<Mutex<HashMap<CacheKey, EvaluationResult>>>,
}

impl InMemoryFormulaCache {
    /// Creates an empty cache.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Returns the number of cached entries.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        return self.entries.lock().expect("cache mutex poisoned").len();
    }

    /// Returns true when the cache is empty.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        return self.len() == 0;
    }
}

#[async_trait]
impl FormulaCache for InMemoryFormulaCache {
    async fn get(&self, key: &CacheKey) -> Option<EvaluationResult> {
        let entries = self.entries.lock().ok()?;
        return entries.get(key).cloned();
    }

    async fn put(&self, key: CacheKey, value: EvaluationResult) {
        let mut entries = self.entries.lock().expect("cache mutex poisoned");
        entries.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(node: usize) -> CacheKey {
        return CacheKey {
            node: NodeId(node),
            inputs: Vec::new(),
        };
    }

    #[tokio::test]
    async fn cache_miss_returns_none() {
        let cache = InMemoryFormulaCache::new();
        let result = cache.get(&key(1)).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn cache_hit_returns_stored_value() {
        let cache = InMemoryFormulaCache::new();
        let k = key(1);
        let v = EvaluationResult {
            value: Decimal::new(42, 0),
        };

        cache.put(k.clone(), v.clone()).await;
        let result = cache.get(&k).await;

        assert_eq!(result, Some(v));
    }

    #[tokio::test]
    async fn distinct_keys_do_not_interfere() {
        let cache = InMemoryFormulaCache::new();
        let k1 = key(1);
        let k2 = key(2);
        let v1 = EvaluationResult {
            value: Decimal::new(10, 0),
        };
        let v2 = EvaluationResult {
            value: Decimal::new(20, 0),
        };

        cache.put(k1.clone(), v1).await;
        cache.put(k2.clone(), v2).await;

        assert_eq!(
            cache.get(&k1).await,
            Some(EvaluationResult {
                value: Decimal::new(10, 0)
            })
        );
        assert_eq!(
            cache.get(&k2).await,
            Some(EvaluationResult {
                value: Decimal::new(20, 0)
            })
        );
    }

    #[tokio::test]
    async fn put_overwrites_existing_entry() {
        let cache = InMemoryFormulaCache::new();
        let k = key(1);

        cache
            .put(
                k.clone(),
                EvaluationResult {
                    value: Decimal::new(1, 0),
                },
            )
            .await;
        cache
            .put(
                k.clone(),
                EvaluationResult {
                    value: Decimal::new(2, 0),
                },
            )
            .await;

        let result = cache.get(&k).await;
        assert_eq!(
            result,
            Some(EvaluationResult {
                value: Decimal::new(2, 0)
            })
        );
    }
}

/// `Redis`-backed formula cache for sharing across API server instances.
///
/// Entries are stored with a configurable TTL. The key format is
/// `cache:formula:{node_id}:{hash}` where `hash` is a deterministic hash
/// of the cache key.
///
/// This implementation is only available when the `redis` feature is enabled.
#[cfg(feature = "redis")]
#[derive(Clone)]
pub struct RedisFormulaCache {
    /// Redis connection manager for automatic reconnection.
    conn: redis::aio::ConnectionManager,

    /// Entry time-to-live in seconds.
    ttl_secs: usize,
}

#[cfg(feature = "redis")]
impl std::fmt::Debug for RedisFormulaCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f
            .debug_struct("RedisFormulaCache")
            .field("conn", &"redis::aio::ConnectionManager")
            .field("ttl_secs", &self.ttl_secs)
            .finish();
    }
}

#[cfg(feature = "redis")]
impl RedisFormulaCache {
    /// Creates a cache backed by an existing Redis connection manager.
    #[must_use]
    pub fn new(conn: redis::aio::ConnectionManager, ttl_secs: usize) -> Self {
        return Self { conn, ttl_secs };
    }

    /// Builds the Redis key for a cache entry.
    fn redis_key(key: &CacheKey) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        return format!("cache:formula:{}:{:x}", key.node.0, hasher.finish());
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl FormulaCache for RedisFormulaCache {
    async fn get(&self, key: &CacheKey) -> Option<EvaluationResult> {
        let redis_key = Self::redis_key(key);
        let value: Option<String> = redis::cmd("GET")
            .arg(&redis_key)
            .query_async(&mut self.conn.clone())
            .await
            .ok()?;
        return value.and_then(|v| serde_json::from_str(&v).ok());
    }

    async fn put(&self, key: CacheKey, value: EvaluationResult) {
        let redis_key = Self::redis_key(&key);
        let value_json = serde_json::to_string(&value).unwrap_or_default();
        let _: Result<(), _> = redis::cmd("SETEX")
            .arg(&redis_key)
            .arg(self.ttl_secs)
            .arg(&value_json)
            .query_async(&mut self.conn.clone())
            .await;
    }
}
