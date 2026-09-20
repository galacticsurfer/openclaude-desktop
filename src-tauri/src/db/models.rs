//! Row types shared between the repositories and the IPC layer.
//!
//! These serialise straight to the frontend, so field names are camelCase and
//! nothing secret is ever a member.

use serde::{Deserialize, Serialize};

pub type Millis = i64;

pub fn now_ms() -> Millis {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => Role::User,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Pending,
    Streaming,
    Complete,
    Interrupted,
    Error,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Streaming => "streaming",
            Self::Complete => "complete",
            Self::Interrupted => "interrupted",
            Self::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "streaming" => Self::Streaming,
            "interrupted" => Self::Interrupted,
            "error" => Self::Error,
            _ => Self::Complete,
        }
    }

    /// A message that was mid-flight when the process ended.
    pub fn is_in_flight(self) -> bool {
        matches!(self, Self::Pending | Self::Streaming)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub title_locked: bool,
    pub provider: String,
    pub provider_conversation_id: Option<String>,
    pub model: String,
    pub system_prompt: Option<String>,
    pub project_id: Option<String>,
    pub pinned: bool,
    pub archived: bool,
    pub deleted_at: Option<Millis>,
    pub branched_from_message_id: Option<String>,
    pub created_at: Millis,
    pub updated_at: Millis,
    pub last_message_at: Option<Millis>,
    pub metadata: serde_json::Value,
}

/// A conversation plus the denormalised counters the sidebar needs, so the
/// list renders from one query instead of N.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub message_count: i64,
    pub preview: Option<String>,
    pub project_name: Option<String>,
}

/// A reusable prompt the user has saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub body: String,
    pub use_count: i64,
    pub last_used_at: Option<Millis>,
    pub created_at: Millis,
    pub updated_at: Millis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub seq: i64,
    pub role: Role,
    pub content: String,
    pub thinking: Option<String>,
    /// Reasoning tokens the provider estimated for this reply. Claude Code
    /// gives a count but never the text, so this may be set while
    /// `thinking` is None.
    pub thinking_tokens: Option<i64>,
    pub status: MessageStatus,
    pub model: Option<String>,
    pub provider_message_id: Option<String>,
    pub stop_reason: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub created_at: Millis,
    pub updated_at: Millis,
    pub metadata: serde_json::Value,
    /// Populated on read; attachments live in their own table.
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentKind {
    Image,
    Document,
    Text,
}

impl AttachmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Document => "document",
            Self::Text => "text",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "image" => Self::Image,
            "document" => Self::Document,
            _ => Self::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub project_id: Option<String>,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub kind: AttachmentKind,
    pub storage_path: String,
    pub sha256: String,
    /// Never serialised to the sidebar; only used when building a request.
    #[serde(skip_serializing)]
    pub text_content: Option<String>,
    pub created_at: Millis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub working_dir: Option<String>,
    pub default_model: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub archived: bool,
    pub created_at: Millis,
    pub updated_at: Millis,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    #[serde(flatten)]
    pub project: Project,
    pub conversation_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub conversation_id: String,
    pub conversation_title: String,
    pub message_id: Option<String>,
    /// `title` | `message` | `attachment` | `project`
    pub kind: String,
    pub role: Option<Role>,
    /// FTS5 snippet with `<<`/`>>` delimiters around matched terms.
    pub snippet: String,
    pub created_at: Millis,
    pub rank: f64,
}

/// Aggregate token counts for a conversation.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTotals {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub message_count: i64,
}

/// Helper for `metadata TEXT` columns: tolerate corrupt JSON rather than
/// failing the whole read.
pub fn parse_json_object(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
}
