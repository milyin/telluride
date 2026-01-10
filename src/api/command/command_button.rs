use std::{fmt::Display, str::FromStr, sync::Arc};

use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};

use crate::api::data_store::data_store_trait::DataStoreTrait;

/// Type alias for callback data (the actual callback string)
pub type CallbackData = String;

/// Represents different types of inline keyboard buttons
#[derive(Clone)]
pub enum ButtonData {
    /// Callback button with label and callback data
    Callback(String, String),
    /// Switch inline query button with label and query text
    SwitchInlineQuery(String, String),
}

impl From<(String, String)> for ButtonData {
    fn from((label, data): (String, String)) -> Self {
        ButtonData::Callback(label, data)
    }
}

impl From<(&str, &str)> for ButtonData {
    fn from((label, data): (&str, &str)) -> Self {
        ButtonData::Callback(label.to_string(), data.to_string())
    }
}

/// Trait for callback data storage read operations (maps short references to full callback data)
/// This is used to work around Telegram's 64-byte limit on callback data
#[async_trait::async_trait]
pub trait CallbackDataStorageReadTrait: Send + Sync {
    /// Retrieve original callback data from a reference string
    async fn get_callback_data(&self, reference: &str) -> Option<CallbackData>;
}

/// Trait for callback data storage operations (maps short references to full callback data)
/// This is used to work around Telegram's 64-byte limit on callback data
#[async_trait::async_trait]
pub trait CallbackDataStorageTrait: CallbackDataStorageReadTrait + Send + Sync {
    /// Store callback data and return a short reference string
    /// The reference is based on (message_id, button_position)
    async fn store_callback_data(
        &self,
        message_id: i32,
        button_pos: usize,
        data: CallbackData,
    ) -> String;

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
pub struct CallbackDataStorage {
    store: Arc<dyn DataStoreTrait<CallbackData>>,
    chat_id: ChatId,
}

impl CallbackDataStorage {
    /// Create a new CallbackDataStorage with the given DataStore and chat ID
    pub fn new(store: Arc<dyn DataStoreTrait<CallbackData>>, chat_id: ChatId) -> Self {
        Self { store, chat_id }
    }
}

/// Implement CallbackDataStorageReadTrait for CallbackDataStorage
#[async_trait::async_trait]
impl CallbackDataStorageReadTrait for CallbackDataStorage {
    async fn get_callback_data(&self, reference: &str) -> Option<CallbackData> {
        // Reference string is already the key, just look it up
        self.store.get(self.chat_id, reference).await
    }
}

/// Implement CallbackDataStorageTrait for CallbackDataStorage
#[async_trait::async_trait]
impl CallbackDataStorageTrait for CallbackDataStorage {
    async fn store_callback_data(
        &self,
        message_id: i32,
        button_pos: usize,
        data: CallbackData,
    ) -> String {
        let key = CallbackDataKey::new(self.chat_id, message_id, button_pos);
        let reference = key.to_string();
        self.store.set(self.chat_id, &reference, data).await;
        reference
    }

    async fn clear_message_callbacks(&self, message_id: i32) {
        // Get all keys and filter out the ones for this message
        let all_keys = self.store.keys(self.chat_id).await;
        for key_str in all_keys {
            if let Ok(key) = CallbackDataKey::from_str(&key_str)
                && key.chat_id == self.chat_id
                && key.message_id == message_id
            {
                self.store.remove(self.chat_id, &key_str).await;
            }
        }
    }
}

/// Pack callback data into an InlineKeyboardMarkup, storing long data in storage
/// and replacing it with short references.
///
/// This function takes rows of button data where each row contains ButtonData enum values.
/// For callback buttons, if the callback_data is longer than 64 bytes or contains non-ASCII
/// characters, it stores the data in CallbackDataStorage and replaces it with a short reference.
/// For switch inline query buttons, the query text is used directly without storage.
///
/// **Important:** This function clears any previously stored callback data for this message
/// to prevent memory leaks when updating message buttons.
///
/// # Arguments
/// * `storage` - The callback data storage trait
/// * `message_id` - The message ID where buttons will be attached
/// * `rows` - Iterator of button rows, each row is an iterator of ButtonData values
pub async fn pack_callback_data<R, B>(
    storage: &Arc<dyn CallbackDataStorageTrait>,
    message_id: i32,
    rows: impl IntoIterator<Item = R>,
) -> InlineKeyboardMarkup
where
    R: IntoIterator<Item = B>,
    B: Into<ButtonData>,
{
    // Clear old callback data for this message to prevent memory leaks
    storage.clear_message_callbacks(message_id).await;

    let mut button_rows = Vec::new();
    let mut button_pos = 0;

    for row in rows {
        let mut button_row = Vec::new();
        for item in row {
            let button_data: ButtonData = item.into();

            match button_data {
                ButtonData::Callback(label, callback_data) => {
                    // Check if callback_data exceeds 64 bytes or contains non-ASCII
                    let needs_storage = callback_data.len() > 64 || !callback_data.is_ascii();

                    let final_callback_data = if needs_storage {
                        // Store in storage and get reference
                        storage
                            .store_callback_data(message_id, button_pos, callback_data)
                            .await
                    } else {
                        callback_data
                    };

                    button_row.push(InlineKeyboardButton::callback(label, final_callback_data));
                    button_pos += 1;
                }
                ButtonData::SwitchInlineQuery(label, query) => {
                    button_row.push(InlineKeyboardButton::switch_inline_query_current_chat(
                        label, query,
                    ));
                    // Don't increment button_pos for inline query buttons as they don't use storage
                }
            }
        }
        button_rows.push(button_row);
    }

    InlineKeyboardMarkup::new(button_rows)
}

/// Unpack callback data from a button press, retrieving the original data from storage if needed.
///
/// # Arguments
/// * `storage` - The callback data storage trait
/// * `callback_data` - The callback data string from the button press
///
/// # Returns
/// The original callback data string, or the input if it wasn't a storage reference
pub async fn unpack_callback_data(
    storage: &Arc<dyn CallbackDataStorageTrait>,
    callback_data: &str,
) -> String {
    // Check if it's a storage reference (starts with "cb:")
    if callback_data.starts_with("cb:") {
        // Try to retrieve from storage
        if let Some(original) = storage.get_callback_data(callback_data).await {
            return original;
        }
    }
    // Not a reference or not found in storage, return as-is
    callback_data.to_string()
}
