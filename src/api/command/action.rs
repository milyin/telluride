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


