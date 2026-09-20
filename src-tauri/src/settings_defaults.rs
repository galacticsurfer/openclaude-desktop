//! Setting keys and their defaults, in one place so the Rust side and the
//! settings UI cannot drift apart.

use serde_json::{json, Value};

pub const DEFAULT_MODEL: &str = "claude.defaultModel";
pub const MAX_TOKENS: &str = "claude.maxTokens";
pub const TEMPERATURE: &str = "claude.temperature";
pub const AUTO_TITLE: &str = "claude.autoTitle";
pub const TITLE_MODEL: &str = "claude.titleModel";
pub const BASE_URL: &str = "claude.baseUrl";
/// `apiKey` or `oauth` — see `secrets::AuthMode`.
pub const AUTH_MODE: &str = "claude.authMode";
/// Which Anthropic CLI profile to use when `AUTH_MODE` is `oauth`.
pub const OAUTH_PROFILE: &str = "claude.oauthProfile";

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
pub const SHOW_COST: &str = "privacy.showCost";
pub const PRICING: &str = "privacy.pricing";

pub const DEBUG_LOGS: &str = "advanced.debugLogs";
pub const TRASH_RETENTION_DAYS: &str = "advanced.trashRetentionDays";

pub const LAST_CONVERSATION: &str = "ui.lastConversationId";
pub const SIDEBAR_COLLAPSED: &str = "ui.sidebarCollapsed";
pub const ONBOARDED: &str = "ui.onboarded";

/// Per-million-token prices used only when cost display is switched on.
///
/// Shipped with an `asOf` date and fully editable in Settings → Privacy,
/// because a hardcoded price silently going stale is worse than no price.
/// Cost display defaults to off for exactly that reason.
pub fn default_pricing() -> Value {
    json!({
        "asOf": "2026-09-01",
        "currency": "USD",
        "note": "Per million tokens. Edit these to match your account's actual pricing.",
        "models": {
            "claude-opus-4-5":   { "input": 5.00,  "output": 25.00 },
            "claude-sonnet-4-5": { "input": 3.00,  "output": 15.00 },
            "claude-haiku-4-5":  { "input": 1.00,  "output": 5.00  }
        }
    })
}

/// Every default as one object. The frontend hydrates from this, then
/// overlays whatever the `settings` table holds.
pub fn defaults() -> Value {
    json!({
        DEFAULT_MODEL: Value::Null,
        MAX_TOKENS: 8192,
        TEMPERATURE: Value::Null,
        AUTO_TITLE: true,
        TITLE_MODEL: Value::Null,
        BASE_URL: Value::Null,
        AUTH_MODE: "apiKey",
        OAUTH_PROFILE: Value::Null,

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
        SHOW_COST: false,
        PRICING: default_pricing(),

        DEBUG_LOGS: false,
        TRASH_RETENTION_DAYS: 30,

        LAST_CONVERSATION: Value::Null,
        SIDEBAR_COLLAPSED: false,
        ONBOARDED: false,
    })
}
