use std::{fmt::Display, str::FromStr, sync::Arc};

use teloxide::types::{ChatId, InlineKeyboardButton, UserId};

use crate::api::data_store::data_store_trait::DataStoreTrait;

use serde::{Deserialize, Serialize};

/// Trait for callback data storage read operations (maps short references to full callback data)
/// This is used to work around Telegram's 64-byte limit on callback data
#[async_trait::async_trait]
pub trait CallbackDataStorageReadTrait<C>: Send + Sync
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Retrieve original callback data from a reference string
    async fn get_callback_data(&self, reference: &str) -> Option<C>;
}

/// Trait for callback data storage operations (maps short references to full callback data)
/// This is used to work around Telegram's 64-byte limit on callback data
#[async_trait::async_trait]
pub trait CallbackDataStorageTrait<C>: CallbackDataStorageReadTrait<C> + Send + Sync
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Store callback data and return a short reference string
    /// The reference is based on (message_id, button_position)
    async fn store_callback_data(&self, message_id: i32, button_pos: usize, data: C) -> String;

    /// Clear all callback data for a specific message
    async fn clear_message_callbacks(&self, message_id: i32);
}

/// The key for the callback data storage map
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallbackDataKey {
    chat_id: ChatId,
    message_id: i32,
    button_pos: usize,
}

impl CallbackDataKey {
    pub fn new(chat_id: ChatId, message_id: i32, button_pos: usize) -> Self {
        Self {
            chat_id,
            message_id,
            button_pos,
        }
    }
}

/// Implementation to string conversion for CallbackDataKey
/// This is used to create unique references in the button callback data
impl Display for CallbackDataKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cb:{}:{}:{}",
            self.chat_id.0, self.message_id, self.button_pos
        )
    }
}

/// Try to convert from string to CallbackDataKey
/// Returns None if the string is not in the expected format
/// Example format: "cb:{chat_id}:{message_id}:{button_pos}"
impl std::str::FromStr for CallbackDataKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 4 || parts[0] != "cb" {
            return Err(());
        }

        let chat_id = parts[1].parse::<i64>().map_err(|_| ())?;
        let message_id = parts[2].parse::<i32>().map_err(|_| ())?;
        let button_pos = parts[3].parse::<usize>().map_err(|_| ())?;

        Ok(CallbackDataKey::new(
            ChatId(chat_id),
            message_id,
            button_pos,
        ))
    }
}

/// The CallbackDataStorage implementation which maps short references to full callback data
/// This is used to work around Telegram's 64-byte limit on callback data
/// Stores data using the reference string as the key in DataStoreTrait
#[derive(Clone)]
pub struct CallbackDataStorage<C>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    pub store: Arc<dyn DataStoreTrait<C>>,
    pub user_id: UserId,
}

impl<C> CallbackDataStorage<C>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Create a new CallbackDataStorage with the given DataStore and user ID
    pub fn new(store: Arc<dyn DataStoreTrait<C>>, user_id: UserId) -> Self {
        Self { store, user_id }
    }

    /// Try to unpack callback data from a string, retrieving from storage if it's a reference
    pub async fn unpack(&self, callback_data: &str) -> Option<C>
    where
        C: FromStr,
    {
        if callback_data.starts_with("cb:") {
            self.get_callback_data(callback_data).await
        } else {
            C::from_str(callback_data).ok()
        }
    }
}

/// Implement CallbackDataStorageReadTrait for CallbackDataStorage
#[async_trait::async_trait]
impl<C> CallbackDataStorageReadTrait<C> for CallbackDataStorage<C>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn get_callback_data(&self, reference: &str) -> Option<C> {
        // Reference string is already the key, just look it up
        self.store.get(self.user_id, reference).await
    }
}

/// Implement CallbackDataStorageTrait for CallbackDataStorage
#[async_trait::async_trait]
impl<C> CallbackDataStorageTrait<C> for CallbackDataStorage<C>
where
    C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn store_callback_data(&self, message_id: i32, button_pos: usize, data: C) -> String {
        let key = CallbackDataKey::new(ChatId(0), message_id, button_pos);
        let reference = key.to_string();
        self.store.set(self.user_id, &reference, data).await;
        reference
    }

    async fn clear_message_callbacks(&self, message_id: i32) {
        // Get all keys and filter out the ones for this message
        let all_keys = self.store.keys(self.user_id).await;
        for key_str in all_keys {
            if let Ok(key) = CallbackDataKey::from_str(&key_str) {
                if key.message_id == message_id {
                    self.store.remove(self.user_id, &key_str).await;
                }
            }
        }
    }
}

/// Extension trait for InlineKeyboardButton to support packed (stored) callback data
#[async_trait::async_trait]
pub trait InlineKeyboardButtonPackedExt {
    /// Create a callback button that automatically stores long data in storage
    /// and replaces it with a short reference if needed.
    async fn callback_packed<C, T, S>(
        text: impl Into<String> + Send,
        data: T,
        storage: &S,
        message_id: i32,
        button_pos: usize,
    ) -> InlineKeyboardButton
    where
        C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
        T: Into<C> + Send,
        S: CallbackDataStorageTrait<C> + Sync;
}

#[async_trait::async_trait]
impl InlineKeyboardButtonPackedExt for InlineKeyboardButton {
    async fn callback_packed<C, T, S>(
        text: impl Into<String> + Send,
        data: T,
        storage: &S,
        message_id: i32,
        button_pos: usize,
    ) -> InlineKeyboardButton
    where
        C: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
        T: Into<C> + Send,
        S: CallbackDataStorageTrait<C> + Sync,
    {
        let data: C = data.into();
        let callback_data_str = storage
            .store_callback_data(message_id, button_pos, data)
            .await;

        InlineKeyboardButton::callback(text, callback_data_str)
    }
}
