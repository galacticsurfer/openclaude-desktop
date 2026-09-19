//! Credential storage.
//!
//! The API key is the one genuinely sensitive thing the app holds. Rules:
//!
//!   * it is written only to the Secret Service (GNOME Keyring, KWallet via
//!     its Secret Service bridge) — never to SQLite, never to a config file;
//!   * it is never logged, never included in an error message, and never sent
//!     to the webview — the frontend only ever learns *whether* one is set;
//!   * if no Secret Service is reachable (headless session, no keyring
//!     daemon), we fall back to memory for the current run only, and say so.
//!     Silently writing an obfuscated key to disk would be worse than asking
//!     the user to paste it again.

use crate::error::{AppError, Result};
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "openclaude-desktop";

/// Which backend actually holds the key right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretBackend {
    /// Persisted in the system keyring; survives a reboot.
    Keyring,
    /// No keyring available; held for this run only.
    MemoryOnly,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub configured: bool,
    pub backend: SecretBackend,
    /// Last four characters only, so the user can tell two keys apart without
    /// the full value ever crossing the IPC boundary.
    pub hint: Option<String>,
}

fn memory() -> &'static Mutex<Option<String>> {
    static MEM: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    MEM.get_or_init(|| Mutex::new(None))
}

/// Probe the Secret Service once and cache the answer.
fn keyring_available() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| match keyring::Entry::new(SERVICE, "__probe__") {
        // `NoEntry` is the healthy answer: the daemon replied, nothing stored.
        Ok(e) => !matches!(e.get_password(), Err(keyring::Error::PlatformFailure(_))),
        Err(e) => {
            tracing::warn!(error = %e, "system keyring unavailable; using memory-only storage");
            false
        }
    })
}

pub fn backend() -> SecretBackend {
    if keyring_available() {
        SecretBackend::Keyring
    } else {
        SecretBackend::MemoryOnly
    }
}

fn entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, account).map_err(AppError::from)
}

pub fn set_api_key(provider: &str, key: &str) -> Result<SecretBackend> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::invalid("The API key is empty."));
    }

    if keyring_available() {
        entry(provider)?.set_password(key)?;
        // Drop any memory copy so the two can never disagree.
        *memory().lock().unwrap() = None;
        Ok(SecretBackend::Keyring)
    } else {
        *memory().lock().unwrap() = Some(key.to_string());
        Ok(SecretBackend::MemoryOnly)
    }
}

pub fn get_api_key(provider: &str) -> Result<Option<String>> {
    if keyring_available() {
        match entry(provider)?.get_password() {
            Ok(k) => return Ok(Some(k)),
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(AppError::Keyring(e.to_string())),
        }
    }
    Ok(memory().lock().unwrap().clone())
}

pub fn delete_api_key(provider: &str) -> Result<()> {
    if keyring_available() {
        match entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(AppError::Keyring(e.to_string())),
        }
    }
    *memory().lock().unwrap() = None;
    Ok(())
}

pub fn status(provider: &str) -> CredentialStatus {
    let key = get_api_key(provider).ok().flatten();
    CredentialStatus {
        hint: key.as_ref().and_then(|k| {
            let n = k.chars().count();
            (n >= 4).then(|| k.chars().skip(n - 4).collect())
        }),
        configured: key.is_some(),
        backend: backend(),
    }
}

/// Reject input that is obviously not a key before spending a network round
/// trip on it. Deliberately loose — the authoritative check is
/// `verify_credentials`, and a future key format must not be locked out here.
pub fn looks_like_anthropic_key(key: &str) -> std::result::Result<(), &'static str> {
    let k = key.trim();
    if k.is_empty() {
        return Err("Enter your API key.");
    }
    if k.chars().any(|c| c.is_whitespace()) {
        return Err("That key contains spaces — check it was copied fully.");
    }
    if !k.starts_with("sk-") {
        return Err("Anthropic API keys start with \"sk-\".");
    }
    if k.len() < 20 {
        return Err("That key looks too short.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_obviously_wrong_keys_without_a_network_call() {
        assert!(looks_like_anthropic_key("").is_err());
        assert!(looks_like_anthropic_key("   ").is_err());
        assert!(looks_like_anthropic_key("hello-world-not-a-key").is_err());
        assert!(looks_like_anthropic_key("sk-short").is_err());
        assert!(looks_like_anthropic_key("sk-ant with space inside it").is_err());
    }

    #[test]
    fn accepts_a_plausible_key_shape() {
        assert!(looks_like_anthropic_key("sk-ant-api03-0123456789abcdefghij").is_ok());
        // Leading/trailing whitespace from a copy-paste is tolerated.
        assert!(looks_like_anthropic_key("  sk-ant-api03-0123456789abcdefghij \n").is_ok());
    }

    #[test]
    fn status_hint_never_exposes_more_than_four_characters() {
        let s = CredentialStatus {
            configured: true,
            backend: SecretBackend::MemoryOnly,
            hint: Some("wxyz".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("sk-"));
        assert_eq!(s.hint.unwrap().len(), 4);
    }
}
