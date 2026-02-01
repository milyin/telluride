// Re-export all public items from submodules
pub use error::UnpackError;
pub use callback_key::{CallbackKey, MAX_CALLBACK_DATA_SIZE};
pub use button_ext::InlineKeyboardButtonPackedExt;

mod error;
mod callback_key;
mod button_ext;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::data_store::in_mem::InMemStore;
    use crate::api::data_store::util::UserProxy;

    #[tokio::test]
    async fn test_pack_unpack_inline() {
        let callback_storage = InMemStore::new();
        let user_id = teloxide::types::UserId(1);
        let user_store = UserProxy::new(callback_storage.clone(), user_id);

        let value = "hello".to_string();
        let key = CallbackKey::pack(&value, &user_store).await;
        
        assert!(key.is_inline());
        let unpacked = CallbackKey::unpack::<String, _>(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        assert_eq!(unpacked, value);
    }

    #[tokio::test]
    async fn test_pack_unpack_storage() {
        let callback_storage = InMemStore::new();
        let user_id = teloxide::types::UserId(1);
        let user_store = UserProxy::new(callback_storage.clone(), user_id);

        // Large value that should go to storage
        let value = "a".repeat(100);
        let key = CallbackKey::pack(&value, &user_store).await;
        
        assert!(key.is_storage_backed());
        let unpacked = CallbackKey::unpack::<String, _>(key.as_str(), &user_store)
            .await
            .expect("Should unpack");
        assert_eq!(unpacked, value);
    }

    #[test]
    fn test_legacy_callback_key_still_works() {
        let key1 = CallbackKey::new("legacy_key").unwrap();
        let key2 = CallbackKey::new("legacy_key").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_hash_stability() {
        let value = "test_value";
        let key1 = CallbackKey::from(&value);
        let key2 = CallbackKey::from(&value);
        assert_eq!(key1, key2);
        
        // Different values should have different keys
        let key3 = CallbackKey::from(&"different_value");
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_button_equality() {
        let key = CallbackKey::new("test").unwrap();
        let button1 = teloxide::types::InlineKeyboardButton::callback("Text", key.to_string());
        let button2 = teloxide::types::InlineKeyboardButton::callback("Text", key.to_string());

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

    #[test]
    fn test_is_packed_data() {
        assert!(CallbackKey::is_packed_data("i:{\"test\":1}"));
        assert!(CallbackKey::is_packed_data("s:12345"));
        assert!(!CallbackKey::is_packed_data("legacy_data"));
        assert!(!CallbackKey::is_packed_data(""));
    }
}
