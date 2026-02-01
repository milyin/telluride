use std::hash::Hash;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

/// Trait for key-value data storage with serializable values
/// Storage is organized per-chat, with each chat having its own key-value namespace
#[async_trait::async_trait]
pub trait UserDataStoreTrait<K, V>: Send + Sync
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    /// Get a value by key for a specific user
    async fn get(&self, user_id: UserId, key: &K) -> Option<V>;

    /// Set a value for a key for a specific user (overwrites if exists)
    async fn set(&self, user_id: UserId, key: &K, value: V);

    /// Remove a value by key for a specific user, returns true if it existed
    async fn remove(&self, user_id: UserId, key: &K) -> bool;

    /// List all keys in the store for a specific user
    async fn keys(&self, user_id: UserId) -> Vec<K>;

    /// List all users who have stored data in this store
    async fn users(&self) -> Vec<UserId>;
}

#[async_trait::async_trait]
impl<K, V, T> UserDataStoreTrait<K, V> for Arc<T>
where
    T: UserDataStoreTrait<K, V> + Send + Sync + ?Sized,
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn get(&self, user_id: UserId, key: &K) -> Option<V> {
        (**self).get(user_id, key).await
    }

    async fn set(&self, user_id: UserId, key: &K, value: V) {
        (**self).set(user_id, key, value).await
    }

    async fn remove(&self, user_id: UserId, key: &K) -> bool {
        (**self).remove(user_id, key).await
    }

    async fn keys(&self, user_id: UserId) -> Vec<K> {
        (**self).keys(user_id).await
    }

    async fn users(&self) -> Vec<UserId> {
        (**self).users().await
    }
}

/// Trait for key-value data storage scoped to a specific user
#[async_trait::async_trait]
pub trait DataStoreTrait<K, V>: Send + Sync
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    async fn get(&self, key: &K) -> Option<V>;
    async fn set(&self, key: &K, value: V);
    async fn remove(&self, key: &K) -> bool;
    async fn keys(&self) -> Vec<K>;
}

/// A proxy for DataStoreTrait that scopes all operations to a specific user
#[derive(Clone)]
pub struct UserProxy<S> {
    store: S,
    user_id: UserId,
}

impl<S> UserProxy<S> {
    pub fn new(store: S, user_id: UserId) -> Self {
        Self { store, user_id }
    }
}

#[async_trait::async_trait]
impl<K, V, S> DataStoreTrait<K, V> for UserProxy<S>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    S: UserDataStoreTrait<K, V>,
{
    async fn get(&self, key: &K) -> Option<V> {
        self.store.get(self.user_id, key).await
    }

    async fn set(&self, key: &K, value: V) {
        self.store.set(self.user_id, key, value).await
    }

    async fn remove(&self, key: &K) -> bool {
        self.store.remove(self.user_id, key).await
    }

    async fn keys(&self) -> Vec<K> {
        self.store.keys(self.user_id).await
    }
}

/// A proxy for DataStoreTrait that scopes all operations to a single common user (UserId(0))
#[derive(Clone)]
pub struct CommonProxy<S> {
    store: S,
}

impl<S> CommonProxy<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl<K, V, S> DataStoreTrait<K, V> for CommonProxy<S>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    S: UserDataStoreTrait<K, V>,
{
    async fn get(&self, key: &K) -> Option<V> {
        self.store.get(UserId(0), key).await
    }

    async fn set(&self, key: &K, value: V) {
        self.store.set(UserId(0), key, value).await
    }

    async fn remove(&self, key: &K) -> bool {
        self.store.remove(UserId(0), key).await
    }

    async fn keys(&self) -> Vec<K> {
        self.store.keys(UserId(0)).await
    }
}
