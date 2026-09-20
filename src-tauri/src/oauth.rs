//! Browser-based sign-in via the official Anthropic CLI (`ant`).
//!
//! # Why this, and not a `/login` like Claude Code's
//!
//! Claude Code's `/login` is a *first-party* OAuth flow: Anthropic registered
//! Claude Code as its own OAuth client, and the token it stores belongs to
//! that client. A third-party application cannot legitimately take part in
//! that flow — it would have to embed Claude Code's client id (impersonating
//! a first-party application) or read its credentials file. Both are out of
//! bounds, so neither is implemented here and nothing in this crate reads
//! `~/.claude/`.
//!
//! What *is* supported is the Anthropic CLI's own OAuth: `ant auth login`
//! opens a browser, and stores a short-lived, auto-refreshing profile under
//! `$ANTHROPIC_CONFIG_DIR` (`~/.config/anthropic` by default). The SDKs read
//! the same profile, and so can we — by asking the CLI for a token rather
//! than by parsing its files, so the storage format stays the CLI's business.
//!
//! Note this authenticates to the **API**, against the org and workspace the
//! profile is bound to. It is not a Claude Pro/Max subscription: the billing
//! is the same as an API key. What it buys is credential hygiene — no
//! long-lived secret to paste or store, and tokens that rotate.

use crate::error::{AppError, Result};
use std::process::Command;
use std::time::Duration;

/// The CLI binary. Resolved from `PATH` by the OS; never run through a shell,
/// so nothing here can be turned into command injection.
const ANT: &str = "ant";

/// `print-credentials` may refresh the token over the network first.
const TOKEN_TIMEOUT: Duration = Duration::from_secs(30);

/// OAuth tokens authenticate differently from API keys: a bearer header
/// rather than `x-api-key`, plus a beta opt-in that `/v1/messages` requires.
pub const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    /// Whether `ant` is on PATH at all.
    pub available: bool,
    /// Whether a profile is logged in and can mint a token right now.
    pub signed_in: bool,
    /// `ant auth status` output, trimmed. Safe to show — it names the
    /// credential source, org and workspace, never a token.
    pub detail: Option<String>,
    pub profile: Option<String>,
}

/// A profile name reaches us from the UI, so it is validated before becoming
/// a process argument — even though we never invoke a shell.
fn check_profile(profile: &str) -> Result<()> {
    if profile.is_empty() || profile.len() > 64 {
        return Err(AppError::invalid("That profile name is not valid."));
    }
    if !profile
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(AppError::invalid(
            "A profile name may only contain letters, digits, dashes, dots and underscores.",
        ));
    }
    Ok(())
}

fn base_command() -> Command {
    let mut cmd = Command::new(ANT);
    // The CLI resolves an API key ahead of any profile, so a stale
    // ANTHROPIC_API_KEY in the environment would silently shadow the very
    // profile we are asking about. Clear it for our child processes only.
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

pub fn is_available() -> bool {
    base_command()
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// What the UI needs to decide which sign-in options to offer.
pub fn status(profile: Option<&str>) -> CliStatus {
    if !is_available() {
        return CliStatus {
            available: false,
            signed_in: false,
            detail: None,
            profile: None,
        };
    }

    let mut cmd = base_command();
    cmd.arg("auth").arg("status");
    if let Some(p) = profile {
        if check_profile(p).is_ok() {
            cmd.arg("--profile").arg(p);
        }
    }

    let detail = cmd
        .stdin(std::process::Stdio::null())
        .output()
        .ok()
        .map(|o| {
            let mut s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                s = String::from_utf8_lossy(&o.stderr).trim().to_string();
            }
            s
        })
        .filter(|s| !s.is_empty());

    // `auth status` reports status even when signed out, and its exit code is
    // explicitly not a health check — so the real test is whether a token can
    // actually be minted.
    let signed_in = access_token(profile).is_ok();

    CliStatus {
        available: true,
        signed_in,
        detail,
        profile: profile.map(str::to_owned),
    }
}

/// Fetch a usable access token, refreshing it first if the CLI needs to.
///
/// Called once per request rather than cached: tokens are short-lived, the
/// CLI owns refresh, and one ~50ms subprocess is noise next to a multi-second
/// completion. Caching it here would mean duplicating expiry logic that
/// belongs to the CLI.
pub fn access_token(profile: Option<&str>) -> Result<String> {
    let mut cmd = base_command();
    cmd.arg("auth")
        .arg("print-credentials")
        .arg("--access-token");
    if let Some(p) = profile {
        check_profile(p)?;
        cmd.arg("--profile").arg(p);
    }

    let out = cmd
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::invalid(
                "The Anthropic CLI (`ant`) is not installed, so browser sign-in is unavailable.",
            ),
            _ => AppError::invalid(format!("Could not run the Anthropic CLI: {e}")),
        })?;

    if !out.status.success() {
        // stderr here is a CLI diagnostic ("not logged in", "token expired").
        // It never contains the token, which only ever goes to stdout.
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(AppError::invalid(if why.is_empty() {
            "Not signed in. Run `ant auth login`, or use an API key instead.".to_string()
        } else {
            format!("Anthropic CLI sign-in failed: {why}")
        }));
    }

    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    validate_token(&token)?;
    Ok(token)
}

/// Guard against the documented foot-gun: `print-credentials` with no flags
/// prints the whole credentials JSON, which as a bearer header yields an
/// empty response or a protocol error rather than a clear failure.
fn validate_token(token: &str) -> Result<()> {
    if token.is_empty() {
        return Err(AppError::invalid(
            "The Anthropic CLI returned an empty token.",
        ));
    }
    if token.starts_with('{') || token.contains('\n') || token.chars().any(char::is_whitespace) {
        return Err(AppError::invalid(
            "The Anthropic CLI returned something that is not an access token. \
             Check that `ant auth print-credentials --access-token` works.",
        ));
    }
    Ok(())
}

/// Start the browser sign-in. The CLI opens the browser and waits, so this is
/// spawned rather than awaited — the UI polls `status` afterwards.
pub fn begin_login(profile: Option<&str>) -> Result<()> {
    if !is_available() {
        return Err(AppError::invalid(
            "The Anthropic CLI (`ant`) is not installed. Install it to sign in with a browser, \
             or use an API key instead.",
        ));
    }

    let mut cmd = base_command();
    cmd.arg("auth").arg("login");
    if let Some(p) = profile {
        check_profile(p)?;
        cmd.arg("--profile").arg(p);
    }

    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| AppError::invalid(format!("Could not start the Anthropic CLI: {e}")))?;
    Ok(())
}

#[allow(dead_code)]
pub fn token_timeout() -> Duration {
    TOKEN_TIMEOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_names_are_restricted_to_safe_characters() {
        assert!(check_profile("default").is_ok());
        assert!(check_profile("work-2.prod_a").is_ok());

        // Nothing that could be meaningful to a shell or a path.
        assert!(check_profile("").is_err());
        assert!(check_profile("a b").is_err());
        assert!(check_profile("../../etc/passwd").is_err());
        assert!(check_profile("a;rm -rf /").is_err());
        assert!(check_profile("$(whoami)").is_err());
        assert!(check_profile(&"x".repeat(65)).is_err());
    }

    #[test]
    fn the_whole_credentials_json_is_rejected_as_a_token() {
        // `print-credentials` with no flags prints JSON; used as a bearer
        // header that fails obscurely, so catch it here instead.
        let json = r#"{"access_token":"abc","refresh_token":"def"}"#;
        let err = validate_token(json).unwrap_err().to_string();
        assert!(err.contains("not an access token"), "{err}");
    }

    #[test]
    fn empty_and_malformed_tokens_are_rejected() {
        assert!(validate_token("").is_err());
        assert!(validate_token("two words").is_err());
        assert!(validate_token("line\nbreak").is_err());
    }

    #[test]
    fn a_plausible_token_is_accepted() {
        assert!(validate_token("sk-ant-oat01-AbCdEf123456").is_ok());
    }
}
