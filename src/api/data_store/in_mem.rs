use std::collections::HashMap;
use std::hash::Hash;

use serde::{Deserialize, Serialize};
use teloxide::types::UserId;
use tokio::sync::Mutex;

use crate::api::data_store::data_store_trait::UserDataStoreTrait;

/// In-memory data store implementation using HashMap
/// Organizes data per-chat with nested HashMaps
pub struct InMemStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    // Outer map: UserId -> Inner map: Key -> Value
    data: Mutex<HashMap<UserId, HashMap<K, V>>>,
}

impl<K, V> InMemStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl<K, V> Default for InMemStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<K, V> UserDataStoreTrait<K, V> for InMemStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    async fn get(&self, user_id: UserId, key: &K) -> Option<V> {
        let data_guard = self.data.lock().await;
        let user_data = data_guard.get(&user_id)?;
        user_data.get(key).cloned()
    }

    async fn set(&self, user_id: UserId, key: &K, value: V) {
        let mut data_guard = self.data.lock().await;
        let user_data = data_guard.entry(user_id).or_insert_with(HashMap::new);
        user_data.insert(key.clone(), value);
    }

    async fn remove(&self, user_id: UserId, key: &K) -> bool {
        let mut data_guard = self.data.lock().await;
        if let Some(user_data) = data_guard.get_mut(&user_id) {
            user_data.remove(key).is_some()
        } else {
            false
        }
    }

    async fn keys(&self, user_id: UserId) -> Vec<K> {
        let data_guard = self.data.lock().await;
        data_guard
            .get(&user_id)
            .map(|user_data| user_data.keys().cloned().collect())
            .unwrap_or_default()
    }

    async fn users(&self) -> Vec<UserId> {
        let data_guard = self.data.lock().await;
        data_guard.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    const TEST_USER: UserId = UserId(12345);

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
        count: i32,
    }

    #[tokio::test]
    async fn test_inmem_store_set_and_get() {
        let store = InMemStore::<String, TestData>::new();
        let data = TestData {
            value: "test".to_string(),
            count: 42,
        };

        store
            .set(TEST_USER, &"key1".to_string(), data.clone())
            .await;
        let retrieved = store.get(TEST_USER, &"key1".to_string()).await;

        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_inmem_store_remove() {
        let store = InMemStore::<String, TestData>::new();
        let data = TestData {
            value: "test".to_string(),
            count: 42,
        };

        store
            .set(TEST_USER, &"key1".to_string(), data.clone())
            .await;
        assert_eq!(store.get(TEST_USER, &"key1".to_string()).await, Some(data));

        let removed = store.remove(TEST_USER, &"key1".to_string()).await;
        assert!(removed);
        assert_eq!(store.get(TEST_USER, &"key1".to_string()).await, None);

        let removed_again = store.remove(TEST_USER, &"key1".to_string()).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_inmem_store_keys() {
        let store = InMemStore::<String, TestData>::new();

        store
            .set(
                TEST_USER,
                &"key1".to_string(),
                TestData {
                    value: "test1".to_string(),
                    count: 1,
                },
            )
            .await;
        store
            .set(
                TEST_USER,
                &"key2".to_string(),
                TestData {
                    value: "test2".to_string(),
                    count: 2,
                },
            )
            .await;

        let keys = store.keys(TEST_USER).await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[tokio::test]
    async fn test_inmem_store_users() {
        let store = InMemStore::<String, TestData>::new();
        let user1 = UserId(111);
        let user2 = UserId(222);
        let data = TestData {
            value: "test".to_string(),
            count: 1,
        };

        store.set(user1, &"k1".to_string(), data.clone()).await;
        store.set(user2, &"k2".to_string(), data.clone()).await;

        let users = store.users().await;
        assert_eq!(users.len(), 2);
        assert!(users.contains(&user1));
        assert!(users.contains(&user2));
    }
}
