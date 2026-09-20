//! An embedded terminal, driven by the person using the app.
//!
//! **This is not a tool.** Claude cannot read from it, write to it, or know
//! it exists. The tool lockdown is unchanged: sessions still run with
//! `--tools ""`, and MCP tools are still approved one at a time. What runs
//! here runs because a human typed it, which is the same authority they
//! already have in any terminal emulator — and is a categorically different
//! thing from a model deciding to run something.
//!
//! The separation is structural rather than a matter of discipline: no IPC
//! command carries shell output into a prompt, and nothing in the provider
//! path can reach a [`ShellSession`]. Moving output into a conversation
//! requires the user to select and copy it.

use crate::error::{AppError, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// One running shell.
pub struct ShellSession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    /// Dropping this kills the child.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct Shells {
    sessions: Mutex<HashMap<String, ShellSession>>,
}

/// The user's login shell, or a sane default.
///
/// `$SHELL` rather than a hardcoded `/bin/bash`: someone using fish or zsh
/// expects their own shell, with their own prompt and history.
fn login_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

impl Shells {
    /// Start a shell and stream its output through `on_output`.
    pub fn open<F>(&self, id: &str, cwd: &std::path::Path, on_output: F) -> Result<()>
    where
        F: Fn(String) + Send + 'static,
    {
        let pty = NativePtySystem::default();
        let pair = pty
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::internal(format!("Could not open a terminal: {e}")))?;

        let mut cmd = CommandBuilder::new(login_shell());
        cmd.cwd(cwd);
        // Without this the shell announces itself as dumb and many programs
        // disable colour and line editing.
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AppError::internal(format!("Could not start a shell: {e}")))?;
        // The slave is held by the child now; keeping our copy would stop
        // the reader ever seeing EOF when the shell exits.
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AppError::internal(format!("Could not read the terminal: {e}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AppError::internal(format!("Could not write to the terminal: {e}")))?;

        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    // Output is bytes, not text: a multi-byte character can
                    // straddle two reads, so decode lossily rather than
                    // dropping a chunk that is not valid UTF-8 on its own.
                    Ok(n) => on_output(String::from_utf8_lossy(&buf[..n]).into_owned()),
                }
            }
        });

        self.sessions.lock().unwrap().insert(
            id.to_string(),
            ShellSession {
                writer,
                master: pair.master,
                _child: child,
            },
        );
        Ok(())
    }

    pub fn write(&self, id: &str, data: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get_mut(id)
            .ok_or(AppError::NotFound("That terminal is no longer open."))?;
        s.writer
            .write_all(data.as_bytes())
            .and_then(|()| s.writer.flush())
            .map_err(|e| AppError::internal(format!("Could not write to the terminal: {e}")))
    }

    pub fn resize(&self, id: &str, rows: u16, cols: u16) -> Result<()> {
        let sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get(id)
            .ok_or(AppError::NotFound("That terminal is no longer open."))?;
        s.master
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| AppError::internal(format!("Could not resize the terminal: {e}")))
    }

    /// Close one shell. Dropping the session kills the child process.
    pub fn close(&self, id: &str) {
        self.sessions.lock().unwrap().remove(id);
    }

    /// Close every shell — on quit, so no orphan processes are left behind.
    pub fn close_all(&self) {
        self.sessions.lock().unwrap().clear();
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.sessions.lock().unwrap().contains_key(id)
    }
}

pub type SharedShells = Arc<Shells>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_users_own_shell_is_preferred_over_a_hardcoded_one() {
        // Someone using fish or zsh should get their prompt, not bash's.
        let shell = login_shell();
        assert!(!shell.is_empty());
        assert!(shell.starts_with('/'), "expected a path, got {shell}");
    }

    #[test]
    fn writing_to_a_terminal_that_is_gone_is_an_error_not_a_panic() {
        let shells = Shells::default();
        assert!(shells.write("nope", "ls\n").is_err());
        assert!(shells.resize("nope", 10, 10).is_err());
        // Closing something already closed is fine.
        shells.close("nope");
        assert!(!shells.is_open("nope"));
    }

    #[test]
    fn a_shell_runs_a_command_and_reports_its_output() {
        use std::sync::mpsc;
        let shells = Shells::default();
        let (tx, rx) = mpsc::channel();
        shells
            .open("t1", &std::env::temp_dir(), move |chunk| {
                let _ = tx.send(chunk);
            })
            .unwrap();
        shells.write("t1", "echo openclaude-shell-works\n").unwrap();

        let mut seen = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if let Ok(chunk) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
                seen.push_str(&chunk);
                if seen.contains("openclaude-shell-works") {
                    break;
                }
            }
        }
        assert!(
            seen.contains("openclaude-shell-works"),
            "expected the command's output, saw: {seen:?}"
        );
        shells.close("t1");
    }
}
