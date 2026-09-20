//! Setting keys and their defaults, in one place so the Rust side and the
//! settings UI cannot drift apart.

use serde_json::{json, Value};

pub const DEFAULT_MODEL: &str = "claude.defaultModel";
pub const AUTO_TITLE: &str = "claude.autoTitle";
pub const TITLE_MODEL: &str = "claude.titleModel";

pub const THEME: &str = "appearance.theme";
pub const FONT_SIZE: &str = "appearance.fontSize";
pub const COMPACT: &str = "appearance.compact";
pub const CODE_WRAP: &str = "appearance.codeWrap";
pub const REDUCED_MOTION: &str = "appearance.reducedMotion";

pub const SEND_KEY: &str = "general.sendKey";
pub const RESTORE_LAST: &str = "general.restoreLastConversation";
pub const TRAY_ENABLED: &str = "general.trayEnabled";

pub const NOTIFY_ENABLED: &str = "notifications.enabled";
pub const NOTIFY_MIN_MS: &str = "notifications.minDurationMs";

pub const HISTORY_ENABLED: &str = "privacy.historyEnabled";

pub const DEBUG_LOGS: &str = "advanced.debugLogs";
pub const TRASH_RETENTION_DAYS: &str = "advanced.trashRetentionDays";

pub const LAST_CONVERSATION: &str = "ui.lastConversationId";
pub const SIDEBAR_COLLAPSED: &str = "ui.sidebarCollapsed";
pub const ONBOARDED: &str = "ui.onboarded";

/// Every default as one object. The frontend hydrates from this, then
/// overlays whatever the `settings` table holds.
pub fn defaults() -> Value {
    json!({
        DEFAULT_MODEL: Value::Null,
        AUTO_TITLE: true,
        TITLE_MODEL: Value::Null,

        THEME: "system",
        FONT_SIZE: 15,
        COMPACT: false,
        CODE_WRAP: false,
        REDUCED_MOTION: false,

        SEND_KEY: "enter",
        RESTORE_LAST: true,
        TRAY_ENABLED: false,

        NOTIFY_ENABLED: true,
        NOTIFY_MIN_MS: 8000,

        HISTORY_ENABLED: true,

        DEBUG_LOGS: false,
        TRASH_RETENTION_DAYS: 30,

        LAST_CONVERSATION: Value::Null,
        SIDEBAR_COLLAPSED: false,
        ONBOARDED: false,
    })
}
