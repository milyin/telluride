/// Error type for callback data unpacking failures
#[derive(Debug, Clone)]
pub enum UnpackError {
    /// The callback data could not be deserialized as this value type
    DeserializeError(String),
    /// The callback key was not found in storage
    NotFound,
    /// The callback query has no data
    NoData,
    /// The callback key format is invalid
    InvalidKey(String),
}

impl std::fmt::Display for UnpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnpackError::DeserializeError(msg) => write!(f, "Deserialize error: {}", msg),
            UnpackError::NotFound => write!(f, "Packed data not found in storage"),
            UnpackError::NoData => write!(f, "Callback query has no data"),
            UnpackError::InvalidKey(msg) => write!(f, "Invalid callback key: {}", msg),
        }
    }
}

impl std::error::Error for UnpackError {}

impl From<String> for UnpackError {
    fn from(s: String) -> Self {
        UnpackError::InvalidKey(s)
    }
}