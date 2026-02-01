use std::sync::Arc;

use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

/// Trait for key-value data storage with serializable values
/// Storage is organized per-chat, with each chat having its own key-value namespace
#[async_trait::async_trait]
pub trait DataStoreTrait<V>: Send + Sync
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    /// Get a value by key for a specific user
    async fn get(&self, user_id: UserId, key: &str) -> Option<V>;

    /// Set a value for a key for a specific user (overwrites if exists)
    async fn set(&self, user_id: UserId, key: &str, value: V);

    /// Remove a value by key for a specific user, returns true if it existed
    async fn remove(&self, user_id: UserId, key: &str) -> bool;

    /// List all keys in the store for a specific user
    async fn keys(&self, user_id: UserId) -> Vec<String>;

    /// List all users who have stored data in this store
    async fn users(&self) -> Vec<UserId>;
}

/// Trait for key-value data storage scoped to a specific user
#[async_trait::async_trait]
pub trait UserProxyTrait<V>: Send + Sync
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    async fn get(&self, key: &str) -> Option<V>;
    async fn set(&self, key: &str, value: V);
    async fn remove(&self, key: &str) -> bool;
    async fn keys(&self) -> Vec<String>;
}

/// A proxy for DataStoreTrait that scopes all operations to a specific user
pub struct UserProxy<V>
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    store: Arc<dyn DataStoreTrait<V>>,
    user_id: UserId,
}

impl<V> UserProxy<V>
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    pub fn new(store: Arc<dyn DataStoreTrait<V>>, user_id: UserId) -> Self {
        Self { store, user_id }
    }
}

#[async_trait::async_trait]
impl<V> UserProxyTrait<V> for UserProxy<V>
where
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn get(&self, key: &str) -> Option<V> {
        self.store.get(self.user_id, key).await
    }

    async fn set(&self, key: &str, value: V) {
        self.store.set(self.user_id, key, value).await
    }

    async fn remove(&self, key: &str) -> bool {
        self.store.remove(self.user_id, key).await
    }

    async fn keys(&self) -> Vec<String> {
        self.store.keys(self.user_id).await
    }
}
