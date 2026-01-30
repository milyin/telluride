use serde::{Deserialize, Serialize};

/// Trait for key-value data storage with serializable values
/// Storage is organized per-chat, with each chat having its own key-value namespace
#[async_trait::async_trait]
pub trait DataStoreTrait<V>: Send + Sync
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    /// Get a value by key for a specific user
    async fn get(&self, user_name: &str, key: &str) -> Option<V>;

    /// Set a value for a key for a specific user (overwrites if exists)
    async fn set(&self, user_name: &str, key: &str, value: V);

    /// Remove a value by key for a specific user, returns true if it existed
    async fn remove(&self, user_name: &str, key: &str) -> bool;

    /// List all keys in the store for a specific user
    async fn keys(&self, user_name: &str) -> Vec<String>;
}
