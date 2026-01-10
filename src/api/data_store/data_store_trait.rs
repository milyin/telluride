use serde::{Deserialize, Serialize};
use teloxide::types::ChatId;

/// Trait for key-value data storage with serializable values
/// Storage is organized per-chat, with each chat having its own key-value namespace
#[async_trait::async_trait]
pub trait DataStoreTrait<V>: Send + Sync
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    /// Get a value by key for a specific chat
    async fn get(&self, chat_id: ChatId, key: &str) -> Option<V>;

    /// Set a value for a key for a specific chat (overwrites if exists)
    async fn set(&self, chat_id: ChatId, key: &str, value: V);

    /// Remove a value by key for a specific chat, returns true if it existed
    async fn remove(&self, chat_id: ChatId, key: &str) -> bool;

    /// List all keys in the store for a specific chat
    async fn keys(&self, chat_id: ChatId) -> Vec<String>;
}
