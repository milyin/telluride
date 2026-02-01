use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
};

use teloxide::types::InlineKeyboardButton;

use crate::api::data_store::data_store_trait::DataStoreTrait;

use serde::{Deserialize, Serialize};

/// Error type for unpacking failures
#[derive(Debug, Clone)]
pub enum UnpackError {
    /// The callback data could not be deserialized as this action type
    DeserializeError(String),
    /// The callback key was not found in storage
    NotFound,
    /// The callback query has no data
    NoData,
    /// The callback key format is invalid
    InvalidKey(String),
}

impl std::fmt::Display for UnpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnpackError::DeserializeError(msg) => write!(f, "Deserialize error: {}", msg),
            UnpackError::NotFound => write!(f, "Packed data not found in storage"),
            UnpackError::NoData => write!(f, "Callback query has no data"),
            UnpackError::InvalidKey(msg) => write!(f, "Invalid callback key: {}", msg),
        }
    }
}

impl std::error::Error for UnpackError {}

impl From<String> for UnpackError {
    fn from(s: String) -> Self {
        UnpackError::InvalidKey(s)
    }
}

/// Prefix used for inline (directly embedded) callback data
const INLINE_PREFIX: &str = "i:";
/// Prefix used for storage-backed (hash reference) callback data  
const STORAGE_PREFIX: &str = "s:";

/// Telegram's maximum allowed size for callback data (64 bytes).
/// See: https://core.telegram.org/bots/api#inlinekeyboardbutton
pub const MAX_CALLBACK_DATA_SIZE: usize = 64;

/// A wrapper around callback data to ensure it never exceeds Telegram's 64-byte limit.
/// It stores the actual data in a `DataStoreTrait` and keeps only a hash-based reference.
///
/// Rationale for 64-byte limit:
/// Telegram API specifies that `callback_data` for inline keyboard buttons must not exceed 64 bytes.
/// If the data is longer, it will cause an error when sending the message.
/// `CallbackKey` ensures compliance by hashing longer data and storing it in a backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallbackKey {
    data: [u8; MAX_CALLBACK_DATA_SIZE],
    len: usize,
}

impl serde::Serialize for CallbackKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.data[..self.len])
    }
}

impl<'de> serde::Deserialize<'de> for CallbackKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CallbackDataVisitor;

        impl<'de> serde::de::Visitor<'de> for CallbackDataVisitor {
            type Value = CallbackKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte slice up to 64 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() > MAX_CALLBACK_DATA_SIZE {
                    return Err(E::custom(format!(
                        "length {} exceeds {} bytes",
                        v.len(),
                        MAX_CALLBACK_DATA_SIZE
                    )));
                }
                let mut data = [0u8; MAX_CALLBACK_DATA_SIZE];
                data[..v.len()].copy_from_slice(v);
                Ok(CallbackKey { data, len: v.len() })
            }
        }

        deserializer.deserialize_bytes(CallbackDataVisitor)
    }
}

impl CallbackKey {
    /// Create a new CallbackKey from a string, ensuring it fits in Telegram's limit
    pub fn new(s: &str) -> Result<Self, String> {
        let bytes = s.as_bytes();
        if bytes.len() > MAX_CALLBACK_DATA_SIZE {
            return Err(format!(
                "Callback data too long: {} bytes (max {})",
                bytes.len(),
                MAX_CALLBACK_DATA_SIZE
            ));
        }
        let mut data = [0u8; MAX_CALLBACK_DATA_SIZE];
        data[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            data,
            len: bytes.len(),
        })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }

    /// Create a CallbackKey from a hash value (for storage-backed actions)
    pub fn from_hash(hash: u64) -> Self {
        let key_str = format!("{}{}", STORAGE_PREFIX, hash);
        Self::new(&key_str).expect("Hash key should always fit in 64 bytes")
    }

    /// Check if a string contains packed callback data
    pub fn is_packed_data(data: &str) -> bool {
        data.starts_with(INLINE_PREFIX) || data.starts_with(STORAGE_PREFIX)
    }

    /// Pack an action into a CallbackKey, using storage if needed.
    ///
    /// If the serialized action fits within 64 bytes (with prefix), it's embedded directly.
    /// Otherwise, it's stored in the provided storage and referenced by a hash key.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = CallbackKey::pack(&Action::ShowUser(123), &storage).await;
    /// InlineKeyboardButton::callback("Show User", key.to_string())
    /// ```
    pub fn pack<V, S>(
        value: &V,
        storage: &S,
    ) -> impl std::future::Future<Output = Self> + Send
    where
        V: Serialize + for<'de> Deserialize<'de> + Hash + Clone + Send + Sync,
        S: DataStoreTrait<CallbackKey, V>,
    {
        // Serialize the value
        let serialized = serde_json::to_string(value);
        let value_clone = value.clone();

        async move {
            match serialized {
                Ok(json) => {
                    let inline_data = format!("{}{}", INLINE_PREFIX, json);
                    if inline_data.len() <= MAX_CALLBACK_DATA_SIZE {
                        // Fits inline - embed directly with prefix
                        Self::new(&inline_data).expect("Already checked length")
                    } else {
                        // Too large - store and use hash key
                        let key = Self::from(&value_clone);
                        storage.set(&key, value_clone).await;
                        key
                    }
                }
                Err(_) => {
                    // Serialization failed, fall back to storage
                    let key = Self::from(&value_clone);
                    storage.set(&key, value_clone).await;
                    key
                }
            }
        }
    }

    /// Unpack an action from callback data string, using storage if needed.
    ///
    /// First checks if the data contains an inline-serialized action (prefix "i:").
    /// If not (prefix "s:"), looks up the action in storage.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let action: Action = CallbackKey::unpack(&callback_data, &storage).await?;
    /// ```
    pub fn unpack<V, S>(
        data: &str,
        storage: &S,
    ) -> impl std::future::Future<Output = Result<V, UnpackError>> + Send
    where
        V: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync,
        S: DataStoreTrait<CallbackKey, V>,
    {
        let data = data.to_string();

        async move {
            if let Some(json) = data.strip_prefix(INLINE_PREFIX) {
                // Inline data - deserialize directly
                serde_json::from_str(json)
                    .map_err(|e| UnpackError::DeserializeError(e.to_string()))
            } else if data.starts_with(STORAGE_PREFIX) {
                // Storage-backed - look up
                let key = Self::new(&data)?;
                storage.get(&key).await.ok_or(UnpackError::NotFound)
            } else {
                // Legacy format or unknown - try storage lookup
                let key = Self::new(&data)?;
                storage.get(&key).await.ok_or(UnpackError::NotFound)
            }
        }
    }

    /// Check if this key contains inline data (directly embedded action)
    pub fn is_inline(&self) -> bool {
        self.as_str().starts_with(INLINE_PREFIX)
    }

    /// Check if this key is storage-backed (hash reference)
    pub fn is_storage_backed(&self) -> bool {
        self.as_str().starts_with(STORAGE_PREFIX)
    }
}

impl Display for CallbackKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for CallbackKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<CallbackKey> for String {
    fn from(p: CallbackKey) -> Self {
        p.to_string()
    }
}

impl<T> From<&T> for CallbackKey
where
    T: Hash,
{
    fn from(value: &T) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        Self::from_hash(hash)
    }
}

/// Extension trait for InlineKeyboardButton to support packed (stored) callback data
#[async_trait::async_trait]
pub trait InlineKeyboardButtonPackedExt {
    /// Create a callback button from a value by packing it into the storage
    async fn callback_key<V>(
        text: impl Into<String> + Send,
        value: &V,
        storage: &dyn DataStoreTrait<CallbackKey, V>,
    ) -> InlineKeyboardButton
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + std::hash::Hash;
}

#[async_trait::async_trait]
impl InlineKeyboardButtonPackedExt for InlineKeyboardButton {
    async fn callback_key<V>(
        text: impl Into<String> + Send,
        value: &V,
        storage: &dyn DataStoreTrait<CallbackKey, V>,
    ) -> InlineKeyboardButton
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + std::hash::Hash,
    {
        // Create key from value using From trait
        let key = CallbackKey::from(value);

        // Store value in storage
        storage.set(&key, value.clone()).await;

        InlineKeyboardButton::callback(text, key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::data_store::data_store_trait::UserProxy;
    use crate::api::data_store::in_mem::InMemStore;
    use std::sync::Arc;
    use teloxide::types::UserId;

    #[tokio::test]
    async fn test_pack_unpack_inline() {
        // Test that small values are inlined (prefix "i:")
        let store = Arc::new(InMemStore::<CallbackKey, String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test".to_string();

        let key = CallbackKey::pack(&value, &user_store).await;
        assert!(key.is_inline(), "Small value should be inlined");

        let unpacked: String = CallbackKey::unpack(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        assert_eq!(unpacked, value);
    }

    #[tokio::test]
    async fn test_pack_unpack_storage() {
        // Test that large values use storage (prefix "s:")
        let store = Arc::new(InMemStore::<CallbackKey, String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "a".repeat(100); // Too large for inline

        let key = CallbackKey::pack(&value, &user_store).await;
        assert!(key.is_storage_backed(), "Large value should use storage");

        let unpacked: String = CallbackKey::unpack(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        assert_eq!(unpacked, value);
    }

    #[tokio::test]
    async fn test_legacy_callback_key_still_works() {
        // Test backward compatibility with old callback_key method
        let store = Arc::new(InMemStore::<CallbackKey, String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test_data".to_string();

        let button = InlineKeyboardButton::callback_key("Test", &value, &user_store).await;

        // Extract callback data from button kind
        let callback_data = match &button.kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
            _ => panic!("Expected CallbackData kind"),
        };
        assert!(callback_data.starts_with(STORAGE_PREFIX));

        let packed = CallbackKey::new(&callback_data).unwrap();
        let unpacked = user_store.get(&packed).await;
        assert_eq!(unpacked, Some(value));
    }

    #[tokio::test]
    async fn test_packed_value_hash_stability() {
        let store = Arc::new(InMemStore::<CallbackKey, String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test_data".to_string();

        let button1 = InlineKeyboardButton::callback_key("Test", &value, &user_store).await;
        let button2 = InlineKeyboardButton::callback_key("Test", &value, &user_store).await;

        // Extract and compare callback data
        let data1 = match &button1.kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(data) => data,
            _ => panic!("Expected CallbackData kind"),
        };
        let data2 = match &button2.kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(data) => data,
            _ => panic!("Expected CallbackData kind"),
        };

        assert_eq!(data1, data2);
    }

    #[test]
    fn test_packed_value_limit() {
        let long_string = "a".repeat(64);
        assert!(CallbackKey::new(&long_string).is_ok());

        let too_long_string = "a".repeat(65);
        assert!(CallbackKey::new(&too_long_string).is_err());
    }

    #[test]
    fn test_inline_detection() {
        let inline_key = CallbackKey::new("i:{\"val\":1}").unwrap();
        assert!(inline_key.is_inline());
        assert!(!inline_key.is_storage_backed());

        let storage_key = CallbackKey::from_hash(12345);
        assert!(!storage_key.is_inline());
        assert!(storage_key.is_storage_backed());
    }
}
