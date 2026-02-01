use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
};

use teloxide::types::InlineKeyboardButton;

use crate::api::data_store::data_store_trait::DataStoreTrait;

use serde::{Deserialize, Serialize};

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

    /// Create a CallbackKey from a hash value
    pub fn from_hash(hash: u64) -> Self {
        let key_str = format!("cb:{}", hash);
        Self::new(&key_str).expect("Hash key should always fit in 64 bytes")
    }

    /// Unpack the value from the store using this reference
    pub async fn unpack<V>(&self, store: &dyn DataStoreTrait<Self, V>) -> Option<V>
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    {
        store.get(self).await
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
    async fn test_packed_value_symmetry() {
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
        assert!(callback_data.starts_with("cb:"));

        let packed = CallbackKey::new(&callback_data).unwrap();
        let unpacked = packed.unpack::<String>(&user_store).await;
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
}
