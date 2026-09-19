use serde::{Serialize, Serializer};

/// Every error that can cross the IPC boundary.
///
/// The `Display` text is what reaches the UI, so it must never contain an API
/// key, a request body, or anything else the user would not want in a
/// screenshot. Provider errors carry a structured `ErrorDetail` instead, which
/// the UI shows behind a "Details" disclosure.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("{0} not found")]
    NotFound(&'static str),

    #[error("{0}")]
    Invalid(String),

    #[error("No API key is configured. Add one in Settings → Claude.")]
    MissingCredentials,

    #[error("Could not reach the system keyring: {0}")]
    Keyring(String),

    #[error("{}", .0.message)]
    Provider(Box<ErrorDetail>),

    #[error("Could not reach Claude. Check your network connection.")]
    Offline,

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Internal(String),
}

/// Structured, secret-free diagnostics for the "Details" panel of an error card.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetail {
    /// Short machine-readable category, e.g. `rate_limit`, `overloaded`.
    pub kind: String,
    /// Human-readable, already sanitised.
    pub message: String,
    pub status: Option<u16>,
    /// Anthropic `request-id` header — safe to show and useful in bug reports.
    pub request_id: Option<String>,
    /// Whether retrying the same request could plausibly succeed.
    pub retryable: bool,
    /// Seconds to wait before retrying, when the server told us.
    pub retry_after_secs: Option<u64>,
}

impl AppError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self::Invalid(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn detail(&self) -> Option<&ErrorDetail> {
        match self {
            Self::Provider(d) => Some(d),
            _ => None,
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            Self::Db(_) => "database",
            Self::NotFound(_) => "not_found",
            Self::Invalid(_) => "invalid",
            Self::MissingCredentials => "missing_credentials",
            Self::Keyring(_) => "keyring",
            Self::Provider(d) => &d.kind,
            Self::Offline => "offline",
            Self::Io(_) => "io",
            Self::Internal(_) => "internal",
        }
    }

    pub fn retryable(&self) -> bool {
        match self {
            Self::Provider(d) => d.retryable,
            Self::Offline => true,
            _ => false,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal(format!("Malformed data: {e}"))
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        match e {
            keyring::Error::NoEntry => Self::MissingCredentials,
            other => Self::Keyring(other.to_string()),
        }
    }
}

/// Wire format for an error returned from a `#[tauri::command]`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireError<'a> {
    kind: &'a str,
    message: String,
    retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a ErrorDetail>,
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        WireError {
            kind: self.kind(),
            message: self.to_string(),
            retryable: self.retryable(),
            detail: self.detail(),
        }
        .serialize(s)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
