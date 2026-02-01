use teloxide::types::InlineKeyboardButton;

use super::callback_packing::CallbackKey;

/// Extension trait for InlineKeyboardButton to support packed (stored) callback data
pub trait InlineKeyboardButtonPackedExt {
    /// Create a callback button from a packed CallbackKey
    fn callback_key(text: impl Into<String>, key: &CallbackKey) -> InlineKeyboardButton;
}

impl InlineKeyboardButtonPackedExt for InlineKeyboardButton {
    fn callback_key(text: impl Into<String>, key: &CallbackKey) -> InlineKeyboardButton {
        InlineKeyboardButton::callback(text, key.to_string())
    }
}