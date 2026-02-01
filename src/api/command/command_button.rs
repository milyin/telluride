use std::{
    collections::hash_map::DefaultHasher,
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

    /// Pack a value into a CallbackKey by storing it in the given store and returning its hash-based reference
    pub async fn pack<V>(value: &V, store: &dyn DataStoreTrait<Self, V>) -> Self
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        let key_str = format!("cb:{}", hash);
        let key = Self::new(&key_str).expect("Hash key should always fit in 64 bytes");

        store.set(&key, value.clone()).await;

        key
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

/// Extension trait for InlineKeyboardButton to support packed (stored) callback data
pub trait InlineKeyboardButtonPackedExt {
    /// Create a callback button from a CallbackKey
    fn callback_packed(text: impl Into<String>, packed: CallbackKey) -> InlineKeyboardButton;
}

impl InlineKeyboardButtonPackedExt for InlineKeyboardButton {
    fn callback_packed(text: impl Into<String>, packed: CallbackKey) -> InlineKeyboardButton {
        InlineKeyboardButton::callback(text, packed.to_string())
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

        let packed = CallbackKey::pack(&value, &user_store).await;
        assert!(packed.as_str().starts_with("cb:"));

        let unpacked = packed.unpack::<String>(&user_store).await;
        assert_eq!(unpacked, Some(value));
    }

    #[tokio::test]
    async fn test_packed_value_hash_stability() {
        let store = Arc::new(InMemStore::<CallbackKey, String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test_data".to_string();

        let packed1 = CallbackKey::pack(&value, &user_store).await;
        let packed2 = CallbackKey::pack(&value, &user_store).await;

        assert_eq!(packed1, packed2);
    }

    #[test]
    fn test_packed_value_limit() {
        let long_string = "a".repeat(64);
        assert!(CallbackKey::new(&long_string).is_ok());

        let too_long_string = "a".repeat(65);
        assert!(CallbackKey::new(&too_long_string).is_err());
    }
}
