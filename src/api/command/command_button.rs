use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
    str::FromStr,
};

use teloxide::types::InlineKeyboardButton;

use crate::api::data_store::data_store_trait::UserProxyTrait;

use serde::{Deserialize, Serialize};

/// A wrapper around callback data to ensure it never exceeds Telegram's 64-byte limit.
/// It stores the actual data in a `DataStoreTrait` and keeps only a hash-based reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackedValue {
    data: [u8; 64],
    len: usize,
}

impl PackedValue {
    /// Create a new PackedValue from a string, ensuring it fits in 64 bytes
    pub fn new(s: &str) -> Result<Self, String> {
        let bytes = s.as_bytes();
        if bytes.len() > 64 {
            return Err(format!(
                "Callback data too long: {} bytes (max 64)",
                bytes.len()
            ));
        }
        let mut data = [0u8; 64];
        data[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            data,
            len: bytes.len(),
        })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }

    /// Pack a value into a PackedValue by storing it in the given store and returning its hash-based reference
    pub async fn pack<V>(value: &V, store: &dyn UserProxyTrait<V>) -> Self
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Hash,
    {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();
        let key = format!("cb:{}", hash);

        store.set(&key, value.clone()).await;

        Self::new(&key).expect("Hash key should always fit in 64 bytes")
    }

    /// Unpack the value from the store using this reference
    pub async fn unpack<V>(&self, store: &dyn UserProxyTrait<V>) -> Option<V>
    where
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    {
        let key = self.as_str();
        if !key.starts_with("cb:") {
            return None;
        }
        store.get(key).await
    }
}

impl Display for PackedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for PackedValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<PackedValue> for String {
    fn from(p: PackedValue) -> Self {
        p.to_string()
    }
}

/// Extension trait for InlineKeyboardButton to support packed (stored) callback data
pub trait InlineKeyboardButtonPackedExt {
    /// Create a callback button from a PackedValue
    fn callback_packed(text: impl Into<String>, packed: PackedValue) -> InlineKeyboardButton;
}

impl InlineKeyboardButtonPackedExt for InlineKeyboardButton {
    fn callback_packed(text: impl Into<String>, packed: PackedValue) -> InlineKeyboardButton {
        InlineKeyboardButton::callback(text, packed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::data_store::data_store_trait::{UserProxy, UserProxyTrait};
    use crate::api::data_store::in_mem::InMemStore;
    use std::sync::Arc;
    use teloxide::types::UserId;

    #[tokio::test]
    async fn test_packed_value_symmetry() {
        let store = Arc::new(InMemStore::<String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test_data".to_string();

        let packed = PackedValue::pack(&value, &user_store).await;
        assert!(packed.as_str().starts_with("cb:"));

        let unpacked = packed.unpack::<String>(&user_store).await;
        assert_eq!(unpacked, Some(value));
    }

    #[tokio::test]
    async fn test_packed_value_hash_stability() {
        let store = Arc::new(InMemStore::<String>::new());
        let user_id = UserId(1);
        let user_store = UserProxy::new(store, user_id);
        let value = "test_data".to_string();

        let packed1 = PackedValue::pack(&value, &user_store).await;
        let packed2 = PackedValue::pack(&value, &user_store).await;

        assert_eq!(packed1, packed2);
    }

    #[test]
    fn test_packed_value_limit() {
        let long_string = "a".repeat(64);
        assert!(PackedValue::new(&long_string).is_ok());

        let too_long_string = "a".repeat(65);
        assert!(PackedValue::new(&too_long_string).is_err());
    }
}
