use std::fmt::Debug;
use std::str::FromStr;

use super::command_button::{CallbackKey, MAX_CALLBACK_DATA_SIZE};
use crate::api::data_store::data_store_trait::DataStoreTrait;
use serde::{Deserialize, Serialize};
use teloxide::types::{CallbackQuery, InlineKeyboardButton};

/// Error type for action parsing failures
#[derive(Debug, Clone)]
pub enum ActionError {
    /// The callback data string could not be parsed as this action type
    ParseError(String),
    /// The callback key was not found in storage
    NotFound,
    /// The callback query has no data
    NoData,
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ActionError::NotFound => write!(f, "Action not found in storage"),
            ActionError::NoData => write!(f, "Callback query has no data"),
        }
    }
}

impl std::error::Error for ActionError {}

/// Trait for bot actions, similar to teloxide's `BotCommands` but for callback actions.
///
/// Actions can be stored inline in the callback data (if small enough) or
/// stored in a backend and referenced by a hash key.
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
/// impl BotAction for Action {
///     fn to_callback_data(&self) -> String {
///         match self {
///             Action::ShowUser(uid) => format!("show_user:{}", uid),
///             Action::DeleteMessage(msg_id) => format!("delete:{}", msg_id),
///             Action::Confirm { action, id } => format!("confirm:{}:{}", action, id),
///         }
///     }
///
///     fn from_callback_data(data: &str) -> Result<Self, String> {
///         let parts: Vec<&str> = data.splitn(3, ':').collect();
///         match parts.get(0).copied() {
///             Some("show_user") => {
///                 let uid = parts.get(1)
///                     .ok_or("Missing user ID")?
///                     .parse::<u64>()
///                     .map_err(|e| e.to_string())?;
///                 Ok(Action::ShowUser(uid))
///             }
///             Some("delete") => {
///                 let msg_id = parts.get(1)
///                     .ok_or("Missing message ID")?
///                     .parse::<u64>()
///                     .map_err(|e| e.to_string())?;
///                 Ok(Action::DeleteMessage(msg_id))
///             }
///             Some("confirm") => {
///                 let action = parts.get(1).ok_or("Missing action")?.to_string();
///                 let id = parts.get(2)
///                     .ok_or("Missing ID")?
///                     .parse::<u64>()
///                     .map_err(|e| e.to_string())?;
///                 Ok(Action::Confirm { action, id })
///             }
///             _ => Err(format!("Unknown action: {}", data)),
///         }
///     }
/// }
/// ```
pub trait BotAction: Sized + Clone + std::hash::Hash + Send + Sync {
    /// Serialize this action to a callback data string.
    /// This string will be embedded directly in the callback if it fits within 64 bytes.
    fn to_callback_data(&self) -> String;

    /// Parse an action from a callback data string.
    /// Returns `Err` if the string doesn't match this action type's format.
    fn from_callback_data(data: &str) -> Result<Self, String>;

    /// Check if this action's callback data fits within Telegram's 64-byte limit.
    /// If true, the action can be embedded directly without storage.
    fn fits_inline(&self) -> bool {
        self.to_callback_data().len() <= MAX_CALLBACK_DATA_SIZE
    }

    /// Create an inline keyboard button for this action.
    ///
    /// If the action fits within 64 bytes, it's embedded directly in the callback data.
    /// Otherwise, it's stored in the provided storage and referenced by a hash key.
    fn to_button<S>(
        &self,
        text: impl Into<String> + Send,
        storage: &S,
    ) -> impl std::future::Future<Output = InlineKeyboardButton> + Send
    where
        S: DataStoreTrait<CallbackKey, Self>,
        Self: Serialize + for<'de> Deserialize<'de>,
    {
        let callback_data = self.to_callback_data();
        let text = text.into();
        let action = self.clone();

        async move {
            if callback_data.len() <= MAX_CALLBACK_DATA_SIZE {
                // Fits inline - embed directly
                InlineKeyboardButton::callback(text, callback_data)
            } else {
                // Too large - store and use hash key
                let key = CallbackKey::from(&action);
                storage.set(&key, action).await;
                InlineKeyboardButton::callback(text, key.to_string())
            }
        }
    }

    /// Extract an action from a callback query.
    ///
    /// First attempts to parse the callback data directly as an action.
    /// If that fails (e.g., it's a hash key), looks up the action in storage.
    fn from_callback_query<S>(
        query: &CallbackQuery,
        storage: &S,
    ) -> impl std::future::Future<Output = Result<Self, ActionError>> + Send
    where
        S: DataStoreTrait<CallbackKey, Self>,
        Self: Serialize + for<'de> Deserialize<'de>,
    {
        let data = query.data.clone();

        async move {
            let data = data.as_ref().ok_or(ActionError::NoData)?;

            // First, try to parse directly as an action
            if let Ok(action) = Self::from_callback_data(data) {
                return Ok(action);
            }

            // If direct parsing failed, try to look up in storage
            let key = CallbackKey::new(data).map_err(ActionError::ParseError)?;

            storage.get(&key).await.ok_or(ActionError::NotFound)
        }
    }
}

// Blanket implementation for types that implement FromStr and Display
// This allows simple actions to just implement FromStr/Display
impl<T> BotAction for T
where
    T: FromStr + std::fmt::Display + Clone + std::hash::Hash + Send + Sync,
    T::Err: std::fmt::Display,
{
    fn to_callback_data(&self) -> String {
        self.to_string()
    }

    fn from_callback_data(data: &str) -> Result<Self, String> {
        data.parse().map_err(|e: T::Err| e.to_string())
    }
}
