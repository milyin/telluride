use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::{collections::HashMap, marker::PhantomData, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use teloxide::types::UserId;
use tokio::{fs, sync::Mutex};

use crate::api::data_store::{
    data_store_trait::UserDataStoreTrait,
    util::{decode_filename_to_key, encode_key_to_filename},
};

/// Internal wrapper to store both key and value in the filesystem
/// This allows us to use hash-based filenames while still being able to list keys
#[derive(Serialize, Deserialize)]
struct StorageEntry<K, V> {
    key: K,
    value: V,
}

/// Filesystem-based YAML data store
/// Creates a separate directory for each chat, with each key stored as a .yaml file
#[derive(Clone)]
pub struct FilesystemYamlStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    storage_dir: PathBuf,
    // In-memory cache for loaded values: user_id -> (Key -> Value)
    cache: Arc<Mutex<HashMap<UserId, HashMap<K, V>>>>,
    // Track which keys have been loaded from disk: user_id -> (Key -> bool)
    loaded_keys: Arc<Mutex<HashMap<UserId, HashMap<K, bool>>>>,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> FilesystemYamlStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
            loaded_keys: Arc::new(Mutex::new(HashMap::new())),
            _phantom: PhantomData,
        }
    }

    /// Get the directory path for a specific user
    fn get_user_dir(&self, user_id: UserId) -> PathBuf {
        let safe_user_dir = encode_key_to_filename(&user_id.to_string());
        self.storage_dir.join(safe_user_dir)
    }

    /// Get the file path for a key within a user's directory
    fn get_file_path(&self, user_id: UserId, key: &K) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        self.get_user_dir(user_id)
            .join(format!("{:016x}.yaml", hash))
    }

    /// Load value from disk for a specific user and key
    async fn load_from_disk(&self, user_id: UserId, key: &K) -> Option<V> {
        let file_path = self.get_file_path(user_id, key);

        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let entry: StorageEntry<K, V> = serde_yaml::from_str(&content).ok()?;
                Some(entry.value)
            }
            Err(_) => None, // File doesn't exist or can't be read
        }
    }

    /// Save value to disk for a specific user and key
    async fn save_to_disk(
        &self,
        user_id: UserId,
        key: &K,
        value: &V,
    ) -> Result<(), std::io::Error> {
        // Create user directory if it doesn't exist
        let user_dir = self.get_user_dir(user_id);
        fs::create_dir_all(&user_dir).await?;

        let file_path = self.get_file_path(user_id, key);
        let entry = StorageEntry {
            key: key.clone(),
            value: value.clone(),
        };

        match serde_yaml::to_string(&entry) {
            Ok(content) => fs::write(&file_path, content).await,
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize to YAML: {}", e),
            )),
        }
    }

    /// Ensure a value is loaded for a key (lazy loading)
    async fn ensure_loaded(&self, user_id: UserId, key: &K) {
        let loaded_guard = self.loaded_keys.lock().await;
        let is_loaded = loaded_guard
            .get(&user_id)
            .and_then(|user_keys| user_keys.get(key).copied())
            .unwrap_or(false);
        if is_loaded {
            // Already loaded
            return;
        }
        drop(loaded_guard); // Release lock while doing I/O

        // Load from disk
        if let Some(value) = self.load_from_disk(user_id, key).await {
            let mut cache_guard = self.cache.lock().await;
            let user_cache = cache_guard.entry(user_id).or_insert_with(HashMap::new);
            user_cache.insert(key.clone(), value);
        }

        // Mark as loaded (even if file didn't exist)
        let mut loaded_guard = self.loaded_keys.lock().await;
        let user_loaded = loaded_guard.entry(user_id).or_insert_with(HashMap::new);
        user_loaded.insert(key.clone(), true);
    }

    /// Delete file from disk
    async fn delete_from_disk(&self, user_id: UserId, key: &K) -> Result<(), std::io::Error> {
        let file_path = self.get_file_path(user_id, key);
        fs::remove_file(&file_path).await
    }
}

#[async_trait::async_trait]
impl<K, V> UserDataStoreTrait<K, V> for FilesystemYamlStore<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Eq + Hash,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
{
    async fn get(&self, user_id: UserId, key: &K) -> Option<V> {
        self.ensure_loaded(user_id, key).await;
        let cache_guard = self.cache.lock().await;
        cache_guard
            .get(&user_id)
            .and_then(|user_cache| user_cache.get(key).cloned())
    }

    async fn set(&self, user_id: UserId, key: &K, value: V) {
        // Update cache
        let mut cache_guard = self.cache.lock().await;
        let user_cache = cache_guard.entry(user_id).or_insert_with(HashMap::new);
        user_cache.insert(key.clone(), value.clone());
        drop(cache_guard);

        // Mark as loaded
        let mut loaded_guard = self.loaded_keys.lock().await;
        let user_loaded = loaded_guard.entry(user_id).or_insert_with(HashMap::new);
        user_loaded.insert(key.clone(), true);
        drop(loaded_guard);

        // Save to disk (ignore errors for now - could log them)
        let _ = self.save_to_disk(user_id, key, &value).await;
    }

    async fn remove(&self, user_id: UserId, key: &K) -> bool {
        self.ensure_loaded(user_id, key).await;

        // Remove from cache
        let mut cache_guard = self.cache.lock().await;
        let existed = cache_guard
            .get_mut(&user_id)
            .map(|user_cache| user_cache.remove(key).is_some())
            .unwrap_or(false);
        drop(cache_guard);

        if existed {
            // Delete from disk (ignore errors)
            let _ = self.delete_from_disk(user_id, key).await;
        }

        existed
    }

    async fn keys(&self, user_id: UserId) -> Vec<K> {
        // For filesystem store, list all .yaml files in the user's directory
        let user_dir = self.get_user_dir(user_id);
        match fs::read_dir(&user_dir).await {
            Ok(mut entries) => {
                let mut keys = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_name_os) = entry.file_name().into_string() {
                        if file_name_os.ends_with(".yaml") {
                            let file_path = entry.path();
                            if let Ok(content) = fs::read_to_string(&file_path).await {
                                if let Ok(entry) =
                                    serde_yaml::from_str::<StorageEntry<K, V>>(&content)
                                {
                                    keys.push(entry.key);
                                }
                            }
                        }
                    }
                }
                keys
            }
            Err(_) => Vec::new(),
        }
    }

    async fn users(&self) -> Vec<UserId> {
        match fs::read_dir(&self.storage_dir).await {
            Ok(mut entries) => {
                let mut users = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                        if let Ok(dir_name) = entry.file_name().into_string() {
                            let decoded_user_id_str = decode_filename_to_key(&dir_name);
                            if let Ok(user_id) = decoded_user_id_str.parse::<u64>() {
                                users.push(UserId(user_id));
                            }
                        }
                    }
                }
                users
            }
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    const TEST_USER_ID: UserId = UserId(12345);

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
        count: i32,
    }

    #[tokio::test]
    async fn test_filesystem_store_set_and_get() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_store");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists
        let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());

        let data = TestData {
            value: "test".to_string(),
            count: 42,
        };

        store
            .set(TEST_USER_ID, &"key1".to_string(), data.clone())
            .await;
        let retrieved = store.get(TEST_USER_ID, &"key1".to_string()).await;

        assert_eq!(retrieved, Some(data));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_persistence() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_persistence");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        let data = TestData {
            value: "test".to_string(),
            count: 42,
        };

        // Create store and set value
        {
            let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());
            store
                .set(TEST_USER_ID, &"key1".to_string(), data.clone())
                .await;
        }

        // Create new store instance and verify value persisted
        {
            let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());
            let retrieved = store.get(TEST_USER_ID, &"key1".to_string()).await;
            assert_eq!(retrieved, Some(data));
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_remove() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_remove");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists
        let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());

        let data = TestData {
            value: "test".to_string(),
            count: 42,
        };

        store
            .set(TEST_USER_ID, &"key1".to_string(), data.clone())
            .await;
        assert_eq!(
            store.get(TEST_USER_ID, &"key1".to_string()).await,
            Some(data)
        );

        let removed = store.remove(TEST_USER_ID, &"key1".to_string()).await;
        assert!(removed);
        assert_eq!(store.get(TEST_USER_ID, &"key1".to_string()).await, None);

        // Verify file was deleted
        // Note: Filename is now based on hash of the key
        let mut hasher = DefaultHasher::new();
        "key1".to_string().hash(&mut hasher);
        let hash = hasher.finish();
        let expected_filename = format!("{:016x}.yaml", hash);
        let file_path = temp_dir
            .join(encode_key_to_filename(&TEST_USER_ID.to_string()))
            .join(expected_filename);
        assert!(!file_path.exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_with_encoded_keys() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_encoded");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists
        let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());

        // Test keys with characters that need encoding
        let test_keys = vec![
            "path/to/key",
            "key:value",
            ".hidden",
            "file*.txt",
            "space key",
        ];

        for key in test_keys {
            let key_string = key.to_string();
            let data = TestData {
                value: format!("data for {}", key),
                count: 1,
            };

            // Set the value
            store.set(TEST_USER_ID, &key_string, data.clone()).await;

            // Retrieve the value
            let retrieved = store.get(TEST_USER_ID, &key_string).await;
            assert_eq!(retrieved, Some(data.clone()));

            // Verify file exists (indirectly)
            let keys = store.keys(TEST_USER_ID).await;
            assert!(keys.contains(&key_string));
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_keys_with_encoding() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_keys_encoded");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists
        let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());

        // Store values with various keys including chars needing encoding
        let keys = vec!["simple", "path/to/key", "key:value", ".hidden", "space key"];

        for key in &keys {
            store
                .set(
                    TEST_USER_ID,
                    &key.to_string(),
                    TestData {
                        value: format!("data for {}", key),
                        count: 1,
                    },
                )
                .await;
        }

        // Retrieve all keys
        let retrieved_keys = store.keys(TEST_USER_ID).await;

        // Verify all keys are decoded correctly
        assert_eq!(retrieved_keys.len(), keys.len());
        for key in keys {
            assert!(
                retrieved_keys.contains(&key.to_string()),
                "Key '{}' should be in retrieved keys",
                key
            );
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_round_trip_complex_keys() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_complex");
        let _ = fs::remove_dir_all(&temp_dir).await; // Clean up if exists

        // Test with complex keys that have multiple characters needing encoding
        let complex_key = "path/to:key*.txt with spaces".to_string();
        let data = TestData {
            value: "complex data".to_string(),
            count: 99,
        };

        // Create store and set value
        {
            let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());
            store.set(TEST_USER_ID, &complex_key, data.clone()).await;
        }

        // Create new store instance and verify value persisted with correct key
        {
            let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());
            let retrieved = store.get(TEST_USER_ID, &complex_key).await;
            assert_eq!(retrieved, Some(data.clone()));

            // Verify the key appears in keys() list
            let keys = store.keys(TEST_USER_ID).await;
            assert!(keys.contains(&complex_key));
        }

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_filesystem_store_users() {
        let temp_dir = std::env::temp_dir().join("yoroolbot_test_fs_users");
        let _ = fs::remove_dir_all(&temp_dir).await;
        let store = FilesystemYamlStore::<String, TestData>::new(temp_dir.clone());

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

        let _ = fs::remove_dir_all(&temp_dir).await;
    }
}
