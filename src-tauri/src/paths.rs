//! XDG Base Directory resolution.
//!
//! Tauri's own path APIs key off the bundle identifier
//! (`~/.local/share/dev.openclaude.desktop`). We deliberately use the plain
//! `openclaude` name instead so the locations are the ones documented in the
//! README and predictable for backup tooling.

use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "openclaude";

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn xdg(var: &str, default: &str) -> PathBuf {
    match std::env::var_os(var) {
        // XDG spec: relative paths in these variables must be ignored.
        Some(v) if Path::new(&v).is_absolute() => PathBuf::from(v),
        _ => home().join(default),
    }
}

/// `~/.local/share/openclaude` — database, attachments.
pub fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join(APP_DIR)
}

/// `~/.config/openclaude` — bootstrap config, window state.
pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join(APP_DIR)
}

/// `~/.local/state/openclaude` — logs.
pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state").join(APP_DIR)
}

/// `~/.cache/openclaude` — regenerable caches (model list, highlighter).
pub fn cache_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache").join(APP_DIR)
}

pub fn database_path() -> PathBuf {
    // An explicit override lets the user relocate the store (Settings → Advanced)
    // and lets tests run against a scratch database.
    if let Some(p) = std::env::var_os("OPENCLAUDE_DB") {
        return PathBuf::from(p);
    }
    data_dir().join("openclaude.db")
}

pub fn attachments_dir() -> PathBuf {
    data_dir().join("attachments")
}

/// An empty directory the Claude Code CLI runs in when a conversation has no
/// project working folder. Deliberately ours and empty, so an unrelated
/// `CLAUDE.md` somewhere on disk cannot silently join the conversation.
pub fn session_dir() -> PathBuf {
    data_dir().join("sessions")
}

/// Where the per-run MCP config is written. State, not config: it is
/// generated from the database, never edited by hand.
pub fn mcp_dir() -> PathBuf {
    state_dir().join("mcp")
}

pub fn log_dir() -> PathBuf {
    state_dir().join("logs")
}

/// Create every directory the app writes to, with owner-only permissions.
pub fn ensure_dirs() -> std::io::Result<()> {
    for d in [
        data_dir(),
        config_dir(),
        state_dir(),
        cache_dir(),
        attachments_dir(),
        log_dir(),
    ] {
        std::fs::create_dir_all(&d)?;
        restrict(&d)?;
    }
    Ok(())
}

/// Conversation contents are private; 0700 keeps other local users out.
fn restrict(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        if perms.mode() & 0o077 != 0 {
            perms.set_mode(0o700);
            std::fs::set_permissions(path, perms)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_xdg_values_are_ignored_per_spec() {
        // Guard against an odd environment silently redirecting the database
        // into a relative path next to the CWD.
        std::env::set_var("XDG_DATA_HOME", "relative/path");
        assert!(data_dir().is_absolute());
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn dirs_are_namespaced_under_openclaude() {
        assert!(data_dir().ends_with("openclaude"));
        assert!(config_dir().ends_with("openclaude"));
        assert!(state_dir().ends_with("openclaude"));
    }
}
