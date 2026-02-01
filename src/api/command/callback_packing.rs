use std::{fmt::Display, hash::{Hash, Hasher}, str::FromStr};
use percent_encoding::{AsciiSet, CONTROLS};

use crate::api::data_store::data_store_trait::DataStoreTrait;
use super::callback_errors::UnpackError;

/// Trait for types that can be encoded/decoded to/from CallbackKey.
/// This trait abstracts the encoding mechanism used for callback data.
/// 
/// Most types should implement `CallbackBitcode` instead, which provides default
/// bitcode-based encoding. Implement this trait directly only when you need
/// completely custom encoding logic.
pub trait CallbackEncode: Hash + Clone + Send + Sync {
    /// Encode the value into bytes
    fn encode_callback(&self) -> Vec<u8>;
    
    /// Decode the value from bytes
    fn decode_callback(bytes: &[u8]) -> Result<Self, String>
    where
        Self: Sized;
    
    /// Returns true if encoding should be bypassed for this instance.
    /// This is useful for large instances where inline encoding would be inefficient.
    /// Default implementation returns false.
    fn bypass_encoding(&self) -> bool {
        false
    }
}

/// Trait for types that want to use bitcode encoding with CallbackKey.
/// Provides default implementations for encoding and decoding.
/// 
/// To use bitcode encoding for your type, simply implement this trait with an empty body:
/// 
/// ```ignore
/// #[derive(Clone, Hash, bitcode::Encode, bitcode::Decode)]
/// struct MyAction { ... }
/// 
/// impl CallbackBitcode for MyAction {}
/// ```
/// 
/// This will automatically implement `CallbackEncode` using bitcode.
/// You can override `encode_callback`, `decode_callback`, or `bypass_encoding` if needed.
pub trait CallbackBitcode: bitcode::Encode + for<'a> bitcode::Decode<'a> + Hash + Clone + Send + Sync {
    /// Encode the value using bitcode.
    /// Override this method to customize encoding behavior.
    fn encode_callback(&self) -> Vec<u8> {
        bitcode::encode(self)
    }
    
    /// Decode the value using bitcode.
    /// Override this method to customize decoding behavior.
    fn decode_callback(bytes: &[u8]) -> Result<Self, String>
    where
        Self: Sized,
    {
        bitcode::decode(bytes).map_err(|e| e.to_string())
    }
    
    /// Returns true if encoding should be bypassed for this instance.
    /// Override this method to implement custom bypass logic.
    fn bypass_encoding(&self) -> bool {
        false
    }
}

/// Blanket implementation of CallbackEncode for types that implement CallbackBitcode.
/// This allows types to opt-in to bitcode encoding by implementing CallbackBitcode.
impl<T> CallbackEncode for T
where
    T: CallbackBitcode,
{
    fn encode_callback(&self) -> Vec<u8> {
        <T as CallbackBitcode>::encode_callback(self)
    }
    
    fn decode_callback(bytes: &[u8]) -> Result<Self, String>
    where
        Self: Sized,
    {
        <T as CallbackBitcode>::decode_callback(bytes)
    }
    
    fn bypass_encoding(&self) -> bool {
        <T as CallbackBitcode>::bypass_encoding(self)
    }
}

// Minimal encoding set - only encode control characters and percent itself
// This allows most UTF-8 characters to pass through unchanged for maximum compactness
const MINIMAL_ENCODE: &AsciiSet = &CONTROLS.add(b'%');

/// Percent-encode raw bytes directly, preserving all byte values
fn percent_encode_bytes(bytes: &[u8]) -> String {
    percent_encoding::percent_encode(bytes, MINIMAL_ENCODE).to_string()
}

/// Decode percent-encoded string back to raw bytes
fn percent_decode_bytes(s: &str) -> Vec<u8> {
    percent_encoding::percent_decode_str(s).collect()
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
                formatter.write_str("a byte array of at most 64 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() > MAX_CALLBACK_DATA_SIZE {
                    return Err(serde::de::Error::invalid_length(
                        v.len(),
                        &format!("at most {} bytes", MAX_CALLBACK_DATA_SIZE).as_str(),
                    ));
                }
                let mut data = [0u8; MAX_CALLBACK_DATA_SIZE];
                data[..v.len()].copy_from_slice(v);
                Ok(CallbackKey { data, len: v.len() })
            }
        }

        deserializer.deserialize_bytes(CallbackDataVisitor)
    }
}

impl FromStr for CallbackKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for CallbackKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl CallbackKey {
    /// Create a new CallbackKey from a string, ensuring it fits within the 64-byte limit.
    pub fn new(value: &str) -> Result<Self, String> {
        let bytes = value.as_bytes();
        if bytes.len() > MAX_CALLBACK_DATA_SIZE {
            return Err(format!(
                "Callback data too large: {} bytes (max {})",
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

    /// Create a CallbackKey from a hash value (for storage-backed data)
    pub fn from_hash(hash: u64) -> Self {
        let key_str = format!("{}{}", STORAGE_PREFIX, hash);
        Self::new(&key_str).expect("Hash key should always fit in 64 bytes")
    }

    /// Check if a string contains packed callback data
    pub fn is_packed_data(data: &str) -> bool {
        data.starts_with(INLINE_PREFIX) || data.starts_with(STORAGE_PREFIX)
    }

    /// Check if this key contains inline data (directly embedded)
    pub fn is_inline(&self) -> bool {
        self.as_str().starts_with(INLINE_PREFIX)
    }

    /// Check if this key references storage-backed data
    pub fn is_storage_backed(&self) -> bool {
        self.as_str().starts_with(STORAGE_PREFIX)
    }

    /// Extract the hash value from a storage-backed key
    pub fn extract_hash(&self) -> Option<u64> {
        let s = self.as_str();
        if let Some(hash_str) = s.strip_prefix(STORAGE_PREFIX) {
            hash_str.parse().ok()
        } else {
            None
        }
    }

    /// Pack a value into a CallbackKey, using storage if needed.
    ///
    /// If the serialized value fits within 64 bytes (with prefix), it's embedded directly.
    /// Otherwise, it's stored in the provided storage and referenced by a hash key.
    ///
    /// If the value's `bypass_encoding()` method returns true,
    /// encoding is skipped and the value is stored directly.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = CallbackKey::pack(Action::ShowUser(123), &storage).await;
    /// InlineKeyboardButton::callback("Show User", key.to_string())
    /// ```
    pub fn pack<V, S>(
        value: V,
        storage: &S,
    ) -> impl std::future::Future<Output = Self> + Send
    where
        V: CallbackEncode,
        S: DataStoreTrait<CallbackKey, V> + ?Sized,
    {
        // Check if encoding should be bypassed
        let should_bypass = value.bypass_encoding();
        
        // Serialize the value
        let serialized = value.encode_callback();

        async move {
            // If bypass_encoding returns true, skip inline encoding and go straight to storage
            if should_bypass {
                let key = Self::from(&value);
                storage.set(&key, value).await;
                return key;
            }
            
            // Combine prefix and serialized data as bytes
            let mut inline_data = Vec::with_capacity(INLINE_PREFIX.len() + serialized.len());
            inline_data.extend_from_slice(INLINE_PREFIX.as_bytes());
            inline_data.extend_from_slice(&serialized);
            
            if inline_data.len() <= MAX_CALLBACK_DATA_SIZE {
                // Fits inline - use minimal percent-encoding on raw bytes
                let encoded = percent_encode_bytes(&inline_data);
                match Self::new(&encoded) {
                    Ok(key) => key,
                    Err(_) => {
                        // Encoding made it too large - store instead
                        let key = Self::from(&value);
                        storage.set(&key, value).await;
                        key
                    }
                }
            } else {
                // Too large - store and use hash key
                let key = Self::from(&value);
                storage.set(&key, value).await;
                key
            }
        }
    }

    /// Unpack a value from callback data string, using storage if needed.
    ///
    /// First checks if the data contains an inline-serialized value (prefix "i:").
    /// If not (prefix "s:"), looks up the value in storage.
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
        V: CallbackEncode,
        S: DataStoreTrait<CallbackKey, V> + ?Sized,
    {
        let data = data.to_string();

        async move {
            // Decode from percent-encoding to get raw bytes
            let decoded = percent_decode_bytes(&data);
            
            if decoded.starts_with(INLINE_PREFIX.as_bytes()) {
                // Inline data - skip the prefix and decode using the trait
                let encoded_data = &decoded[INLINE_PREFIX.len()..];
                V::decode_callback(encoded_data)
                    .map_err(|e| UnpackError::DeserializeError(e))
            } else if decoded.starts_with(STORAGE_PREFIX.as_bytes()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::data_store::in_mem::InMemStore;
    use crate::api::data_store::data_store_trait::UserProxy;
    use std::sync::Arc;
    use teloxide::types::UserId;

    /// Test type that implements CallbackBitcode with custom encoding override
    #[derive(Debug, Clone, Hash, PartialEq, bitcode::Encode, bitcode::Decode)]
    struct CustomBitcodeType {
        value: u32,
    }

    /// Custom implementation that overrides default methods
    impl CallbackBitcode for CustomBitcodeType {
        fn encode_callback(&self) -> Vec<u8> {
            // Custom encoding: prefix with magic bytes
            let mut encoded = vec![0xCA, 0xFE]; // Magic prefix
            encoded.extend_from_slice(&bitcode::encode(self));
            encoded
        }

        fn decode_callback(bytes: &[u8]) -> Result<Self, String> {
            // Check for magic prefix
            if bytes.len() < 2 || &bytes[0..2] != [0xCA, 0xFE] {
                return Err("Missing magic prefix".to_string());
            }
            // Decode the rest with bitcode
            bitcode::decode(&bytes[2..]).map_err(|e| e.to_string())
        }

        fn bypass_encoding(&self) -> bool {
            // Custom bypass logic: bypass if value is large
            self.value > 1000
        }
    }

    #[tokio::test]
    async fn test_custom_callback_encode_for_bitcode_type() {
        let storage = Arc::new(InMemStore::new());
        let user_id = UserId(123);
        let user_store = UserProxy::new(storage, user_id);

        // Small value - should be inlined with custom encoding
        let small_value = CustomBitcodeType { value: 42 };
        let key = CallbackKey::pack(small_value.clone(), &user_store).await;
        
        assert!(key.is_inline(), "Small value should be inlined");
        
        let unpacked = CallbackKey::unpack::<CustomBitcodeType, _>(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        
        assert_eq!(unpacked, small_value);
    }

    #[tokio::test]
    async fn test_custom_bypass_encoding_for_bitcode_type() {
        let storage = Arc::new(InMemStore::new());
        let user_id = UserId(123);
        let user_store = UserProxy::new(storage, user_id);

        // Large value - should bypass encoding due to custom logic
        let large_value = CustomBitcodeType { value: 2000 };
        let key = CallbackKey::pack(large_value.clone(), &user_store).await;
        
        assert!(key.is_storage_backed(), "Large value should be stored, not inlined");
        
        let unpacked = CallbackKey::unpack::<CustomBitcodeType, _>(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        
        assert_eq!(unpacked, large_value);
    }
}