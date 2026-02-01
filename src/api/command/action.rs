use std::fmt::Debug;

/// Error type for action extraction failures
#[derive(Debug, Clone)]
pub enum ActionError {
    /// The callback data could not be deserialized as this action type
    DeserializeError(String),
    /// The callback key was not found in storage
    NotFound,
    /// The callback query has no data
    NoData,
    /// The callback key format is invalid
    InvalidKey(String),
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::DeserializeError(msg) => write!(f, "Deserialize error: {}", msg),
            ActionError::NotFound => write!(f, "Action not found in storage"),
            ActionError::NoData => write!(f, "Callback query has no data"),
            ActionError::InvalidKey(msg) => write!(f, "Invalid callback key: {}", msg),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<String> for ActionError {
    fn from(s: String) -> Self {
        ActionError::InvalidKey(s)
    }
}

/// Trait for bot actions that can be packed into callback data.
///
/// Actions are serialized using serde. If the serialized form fits within
/// Telegram's 64-byte callback data limit, it's embedded directly.
/// Otherwise, it's stored in a backend and referenced by a hash key.
///
/// # Example
///
/// ```ignore
/// use serde::{Deserialize, Serialize};
/// use telluride::command::BotAction;
///
/// #[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
/// enum Action {
///     ShowUser(u64),
///     DeleteMessage(u64),
///     Confirm { action: String, id: u64 },
/// }
///
/// // BotAction is automatically implemented for types with required traits
/// ```
///
/// The trait is auto-implemented for types that satisfy the bounds.
/// Use `CallbackKey::pack()` and `CallbackKey::unpack()` to work with actions:
///
/// ```ignore
/// // Creating a button
/// let key = CallbackKey::pack(&Action::ShowUser(123), &storage).await;
/// InlineKeyboardButton::callback("Show User", key.to_string())
///
/// // Extracting action from callback
/// let action: Action = CallbackKey::unpack(&callback_data, &storage).await?;
/// ```
pub trait BotAction:
    serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + std::hash::Hash + Send + Sync
{
}

// Auto-implement BotAction for any type that satisfies the bounds
impl<T> BotAction for T where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Clone + std::hash::Hash + Send + Sync
{
}
