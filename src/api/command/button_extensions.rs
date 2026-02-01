use serde::{Deserialize, Serialize};
use std::hash::Hash;
use teloxide::types::InlineKeyboardButton;

use crate::api::data_store::data_store_trait::DataStoreTrait;
use super::callback_packing::CallbackKey;

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
        V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + Hash,
    {
        let key = CallbackKey::pack(value, storage).await;
        InlineKeyboardButton::callback(text, key.to_string())
    }
}