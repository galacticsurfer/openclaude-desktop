//! The Claude Code CLI as the backend.
//!
//! This app has no API credential of its own and makes no HTTP request. It
//! drives the `claude` binary in its documented headless mode (`-p` with
//! `--output-format stream-json`), which runs as the user, under the login
//! they already established with `claude`. Nothing here reads Claude Code's
//! credential files or knows how it authenticates — that stays entirely the
//! CLI's business.
//!
//! Safety choices worth stating, because a chat window is not a coding agent:
//!
//!   * every built-in tool is disabled, so a reply cannot read a file, run a
//!     command, or fetch a URL;
//!   * the process runs in an empty scratch directory unless a project names
//!     a working folder, so a stray `CLAUDE.md` cannot silently join the
//!     conversation;
//!   * the prompt goes in on stdin, not in `argv` — an inlined attachment
//!     would otherwise risk the argument-length limit, and arguments are
//!     visible to other processes.

use super::ndjson::LineDecoder;
use super::{AIProvider, ChatRequest, ContentBlock, EventStream, ModelInfo, StreamEvent};
use crate::error::{AppError, Result};
use futures_util::StreamExt;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Resolved from `PATH` by the OS; never run through a shell.
pub const CLAUDE_BIN: &str = "claude";

/// Tools denied for a chat conversation.
///
/// This is the complete set the CLI exposes, verified by asking it: with
/// this list plus `--strict-mcp-config`, a session reports **zero** tools.
///
/// It has to be exhaustive, and that is the weakness of a deny list — an
/// earlier version named only the obvious ones and left nineteen others
/// live, including `Read`, `Write`, `Edit`, `Grep`, `SendMessage` and
/// `CronDelete`, plus every MCP tool. `--restricted` is not enough either:
/// it removes only the command-running tools and WebFetch, and leaves file
/// access intact. Because a CLI upgrade can add tools this list has never
/// heard of, [`unexpected_tools`] re-checks at runtime and the app warns
/// rather than trusting this to stay complete.
const DISALLOWED_TOOLS: &[&str] = &[
    // File and shell access.
    "Bash",
    "Read",
    "Write",
    "Edit",
    "NotebookEdit",
    "Glob",
    "Grep",
    // Network.
    "WebFetch",
    "WebSearch",
    // Delegation and background work.
    "Task",
    "TaskOutput",
    "TaskStop",
    "TodoWrite",
    "Workflow",
    "ListAgents",
    "Skill",
    "ToolSearch",
    // Scheduling, messaging and anything else with an outside effect.
    "CronCreate",
    "CronDelete",
    "CronList",
    "ScheduleWakeup",
    "SendMessage",
    "PushNotification",
    "RemoteTrigger",
    "Monitor",
    "LSP",
    "DesignSync",
    "ReportFindings",
    "EnterWorktree",
    "ExitWorktree",
];

/// Tools a session reported that we did not expect to be available.
///
/// A chat window claims it cannot touch your files; this is what keeps that
/// claim honest when the CLI gains a tool after this was written.
pub fn unexpected_tools(reported: &[String]) -> Vec<String> {
    reported
        .iter()
        .filter(|t| !DISALLOWED_TOOLS.contains(&t.as_str()))
        .cloned()
        .collect()
}

/// Accepted `--effort` values. Validated rather than passed through, so a
/// stale setting cannot make every request fail.
pub const EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// Fallback aliases, used only when the CLI cannot be asked (not installed,
/// or the probe failed). The real list is discovered at runtime — see
/// [`discover_models`] — because which aliases exist changes with the CLI
/// version, and a pinned list would quietly go stale.
const FALLBACK_ALIASES: &[&str] = &["opus", "sonnet", "haiku"];

/// What `/model` reports.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalog {
    /// Human name of the model in use, e.g. "Opus 5 (1M context)".
    pub current: Option<String>,
    /// Effort level the CLI is currently applying.
    pub effort: Option<String>,
    /// Aliases that can be passed to `--model`.
    pub available: Vec<String>,
}

/// Parse the output of `/model`, which looks like:
///
/// ```text
/// Current model: `Opus 5 (1M context)` (effort: high)
/// Usage: /model <name>. Available: sonnet, opus, haiku, …, or a full model ID.
/// ```
///
/// Written defensively: this is human-facing text that can change between
/// CLI versions, so anything unrecognised yields an empty field rather than
/// a wrong one.
pub fn parse_model_output(text: &str) -> ModelCatalog {
    let mut catalog = ModelCatalog::default();

    for line in text.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("Current model:") {
            // The name is backtick-quoted; effort follows in parentheses.
            if let (Some(a), Some(b)) = (rest.find('`'), rest.rfind('`')) {
                if b > a {
                    catalog.current = Some(rest[a + 1..b].trim().to_string());
                }
            }
            if let Some(i) = rest.find("(effort:") {
                let after = &rest[i + "(effort:".len()..];
                if let Some(end) = after.find(')') {
                    let effort = after[..end].trim().to_string();
                    if !effort.is_empty() {
                        catalog.effort = Some(effort);
                    }
                }
            }
        }

        if let Some(i) = line.find("Available:") {
            catalog.available = line[i + "Available:".len()..]
                .split(',')
                .map(|e| e.trim().trim_end_matches('.').trim())
                // Drops the trailing prose ("or a full model ID") and
                // anything else that is clearly not an alias.
                .filter(|e| {
                    !e.is_empty()
                        && !e.contains(' ')
                        && e.chars()
                            .all(|c| c.is_ascii_alphanumeric() || "-_.[]".contains(c))
                })
                .map(str::to_owned)
                .collect();
        }
    }

    catalog
}

/// Ask the CLI which models it accepts.
///
/// `/model` is answered locally — a recorded run reports zero input and
/// output tokens — so this is free and does not touch the user's quota.
/// Session persistence is off, so the probe leaves nothing resumable behind.
pub async fn discover_models() -> Option<ModelCatalog> {
    let out = Command::new(CLAUDE_BIN)
        .arg("--print")
        .arg("--output-format")
        .arg("text")
        .arg("--no-session-persistence")
        .arg("--tools")
        .arg("")
        .arg("--disallowed-tools")
        .arg(DISALLOWED_TOOLS.join(" "))
        .arg("--strict-mcp-config")
        .arg("--permission-mode")
        .arg("dontAsk")
        .arg("/model")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;

    if !out.status.success() {
        return None;
    }
    let catalog = parse_model_output(&String::from_utf8_lossy(&out.stdout));
    (!catalog.available.is_empty()).then_some(catalog)
}

pub struct ClaudeCodeProvider {
    /// Where the CLI process runs. An empty scratch dir unless a project
    /// supplies a working folder.
    cwd: PathBuf,
    /// Claude Code session backing this conversation, when one exists yet.
    session: SessionRef,
    /// Reasoning depth, when the user has chosen one.
    effort: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SessionRef {
    /// First turn: ask the CLI to adopt this id so later turns can resume it.
    New(String),
    /// A later turn in an established session.
    Resume(String),
    /// First turn of a branched conversation: continue from another session's
    /// history but in a session of its own, so the original is untouched.
    /// The CLI mints the new id, which is why the reported one is persisted
    /// rather than assumed.
    Fork { from: String },
}

impl ClaudeCodeProvider {
    pub fn new(cwd: PathBuf, session: SessionRef) -> Self {
        Self {
            cwd,
            session,
            effort: None,
        }
    }

    /// `--effort` trades thoroughness against speed and token spend.
    pub fn with_effort(mut self, effort: Option<String>) -> Self {
        self.effort = effort.filter(|e| EFFORT_LEVELS.contains(&e.as_str()));
        self
    }

    fn command(&self, req: &ChatRequest) -> Command {
        let mut cmd = Command::new(CLAUDE_BIN);
        cmd.arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            // Without this the CLI emits whole messages, not token deltas.
            .arg("--include-partial-messages")
            .arg("--verbose")
            // Empties the built-in tool set outright. This is the control
            // that matters: a deny list only denies the tools it was written
            // against, so a CLI upgrade adding one would silently reopen the
            // hole. Measured — baseline reports 27 tools, this reports 0.
            .arg("--tools")
            .arg("")
            // Belt and braces behind `--tools`, and still the only way to
            // name individual MCP tools.
            .arg("--disallowed-tools")
            .arg(DISALLOWED_TOOLS.join(" "))
            // Without this, every configured MCP server's tools stay live —
            // for this user that included one that can delete documents.
            .arg("--strict-mcp-config")
            // No TTY here, so a permission prompt would hang forever.
            // `none` denies anything that would have prompted instead.
            .arg("--permission-prompts")
            .arg("none")
            .arg("--permission-mode")
            .arg("dontAsk")
            // Structured input, so an attached image or PDF can travel as a
            // content block. Plain text stdin would flatten them away.
            .arg("--input-format")
            .arg("stream-json");

        if !req.model.trim().is_empty() {
            cmd.arg("--model").arg(&req.model);
        }
        if let Some(effort) = &self.effort {
            cmd.arg("--effort").arg(effort);
        }
        if let Some(system) = req.system.as_deref().filter(|s| !s.trim().is_empty()) {
            // Append rather than replace: the CLI's own prompt carries
            // behaviour we are not trying to override.
            cmd.arg("--append-system-prompt").arg(system);
        }

        match &self.session {
            SessionRef::New(id) => {
                cmd.arg("--session-id").arg(id);
            }
            SessionRef::Resume(id) => {
                cmd.arg("--resume").arg(id);
            }
            SessionRef::Fork { from } => {
                cmd.arg("--resume").arg(from).arg("--fork-session");
            }
        }

        cmd.current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd
    }
}

/// Build the JSON user message the CLI reads from stdin.
///
/// Only the trailing user turn is sent: the CLI owns conversation history
/// through its session, so replaying earlier turns would duplicate them.
///
/// Content blocks go across structurally rather than flattened to text,
/// which is what lets an attached image or PDF actually reach the model —
/// they serialise to the same shapes the Messages API uses.
pub fn input_message(req: &ChatRequest) -> Result<String> {
    let Some(last) = req.messages.last() else {
        return Err(AppError::invalid("There is nothing to send."));
    };

    let envelope = serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": last.content },
    });
    Ok(serde_json::to_string(&envelope)? + "\n")
}

/// Whether the turn carries anything at all.
fn has_content(req: &ChatRequest) -> bool {
    req.messages.last().is_some_and(|m| {
        m.content.iter().any(|b| match b {
            ContentBlock::Text { text } => !text.trim().is_empty(),
            ContentBlock::Image { .. } | ContentBlock::Document { .. } => true,
        })
    })
}

/// One decoded line of the CLI's NDJSON output.
#[derive(Debug, PartialEq)]
pub enum CliRecord {
    /// Session established; carries the id to resume later, the model the
    /// alias resolved to, and the slash commands available.
    Init {
        session_id: String,
        model: String,
        slash_commands: Vec<String>,
        tools: Vec<String>,
    },
    /// A wrapped Anthropic stream event.
    Stream(StreamEvent),
    /// Terminal record for the turn. `text` is the complete answer, which
    /// matters for local slash commands: those are answered by the CLI
    /// itself and never stream, so `result` is the only place their output
    /// ever appears.
    Done {
        is_error: bool,
        session_id: Option<String>,
        error: Option<String>,
        text: Option<String>,
    },
    /// Usage limits — surfaced so a block can be explained rather than
    /// looking like a generic failure.
    RateLimited { message: String },
    /// Anything we do not model; ignored.
    Ignored,
}

/// Parse one NDJSON line from the CLI.
pub fn parse_line(line: &str) -> CliRecord {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        return CliRecord::Ignored;
    };

    match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
        "system" if v.get("subtype").and_then(|s| s.as_str()) == Some("init") => {
            // Terminal-only commands (`/doctor`, `/color`) would do nothing
            // through a pipe, so they are filtered out rather than offered.
            let terminal: std::collections::HashSet<&str> = v
                .get("terminal_slash_commands")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
                .unwrap_or_default();

            let mut slash_commands: Vec<String> = v
                .get("slash_commands")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        // Internal entries are not for humans to invoke.
                        .filter(|c| !c.starts_with("__") && !terminal.contains(c))
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            slash_commands.sort();

            let tools: Vec<String> = v
                .get("tools")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();

            CliRecord::Init {
                tools,
                session_id: v
                    .get("session_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                model: v
                    .get("model")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                slash_commands,
            }
        }

        // A richer reasoning signal than the `thinking_delta` path: that one
        // carries empty text and only sometimes an estimate, while this
        // carries a real running total.
        "system" if v.get("subtype").and_then(|s| s.as_str()) == Some("thinking_tokens") => {
            match v.get("estimated_tokens").and_then(|n| n.as_i64()) {
                Some(n) => CliRecord::Stream(StreamEvent::ThinkingProgress(n)),
                None => CliRecord::Ignored,
            }
        }

        "stream_event" => {
            let Some(event) = v.get("event") else {
                return CliRecord::Ignored;
            };
            let kind = event.get("type").and_then(|t| t.as_str()).unwrap_or("");
            super::wire::map_event(kind, event)
                .map(CliRecord::Stream)
                .unwrap_or(CliRecord::Ignored)
        }

        "result" => {
            let is_error = v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false)
                || v.get("subtype").and_then(|s| s.as_str()) == Some("error");
            let result_text = v.get("result").and_then(|r| r.as_str()).map(str::to_owned);
            CliRecord::Done {
                is_error,
                session_id: v
                    .get("session_id")
                    .and_then(|s| s.as_str())
                    .map(str::to_owned),
                error: result_text.clone().filter(|_| is_error),
                text: result_text.filter(|t| !is_error && !t.trim().is_empty()),
            }
        }

        "rate_limit_event" => {
            let info = v.get("rate_limit_info");
            let status = info
                .and_then(|i| i.get("status"))
                .and_then(|s| s.as_str())
                .unwrap_or("");
            // Only the blocking states are worth interrupting the user for.
            if status == "allowed" || status.is_empty() {
                return CliRecord::Ignored;
            }
            let resets = info
                .and_then(|i| i.get("resetsAt"))
                .and_then(|r| r.as_i64())
                .map(|t| format!(" Resets at {}.", format_unix(t)))
                .unwrap_or_default();
            CliRecord::RateLimited {
                message: format!("You've reached your Claude usage limit.{resets}"),
            }
        }

        _ => CliRecord::Ignored,
    }
}

/// Local clock time for a unix timestamp, without pulling in a date library.
fn format_unix(ts: i64) -> String {
    let secs_of_day = ts.rem_euclid(86_400);
    format!(
        "{:02}:{:02} UTC",
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60
    )
}

impl AIProvider for ClaudeCodeProvider {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    async fn stream_message(&self, req: ChatRequest) -> Result<EventStream> {
        if !has_content(&req) {
            return Err(AppError::invalid("There is nothing to send."));
        }
        let payload = input_message(&req)?;

        std::fs::create_dir_all(&self.cwd).ok();

        let mut child = self.command(&req).spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::Provider(Box::new(super::wire::describe_failure("cli_missing", None)))
            }
            _ => AppError::internal(format!("Could not start Claude Code: {e}")),
        })?;

        // Hand over the prompt and close stdin, or the CLI waits for more.
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let bytes = payload.into_bytes();
            tokio::spawn(async move {
                let _ = stdin.write_all(&bytes).await;
                let _ = stdin.shutdown().await;
            });
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::internal("Claude Code produced no output stream."))?;
        let stderr = child.stderr.take();

        struct State {
            child: tokio::process::Child,
            stdout: tokio::process::ChildStdout,
            stderr: Option<tokio::process::ChildStderr>,
            decoder: LineDecoder,
            pending: std::collections::VecDeque<StreamEvent>,
            buf: Vec<u8>,
            finished: bool,
            saw_error: Option<String>,
            streamed_any: bool,
        }

        let state = State {
            child,
            stdout,
            stderr,
            decoder: LineDecoder::new(),
            pending: std::collections::VecDeque::new(),
            buf: vec![0u8; 16 * 1024],
            finished: false,
            saw_error: None,
            streamed_any: false,
        };

        let stream = futures_util::stream::unfold(state, |mut st| async move {
            loop {
                if let Some(ev) = st.pending.pop_front() {
                    return Some((Ok(ev), st));
                }
                if st.finished {
                    return None;
                }

                match st.stdout.read(&mut st.buf).await {
                    Ok(0) => {
                        // Stream closed: flush any unterminated last line,
                        // then decide how the turn ended.
                        if let Some(line) = st.decoder.finish() {
                            handle_line(
                                &line,
                                &mut st.pending,
                                &mut st.saw_error,
                                &mut st.streamed_any,
                            );
                        }
                        st.finished = true;

                        if st.pending.is_empty() {
                            let detail = match st.saw_error.take() {
                                Some(e) => Some(e),
                                None => read_stderr(&mut st.stderr).await,
                            };
                            let status = st.child.wait().await.ok();
                            let failed = st.saw_error.is_some()
                                || status.map(|s| !s.success()).unwrap_or(false);
                            if failed {
                                let kind = classify(detail.as_deref());
                                st.pending.push_back(StreamEvent::Failed(
                                    super::wire::describe_failure(kind, detail.as_deref()),
                                ));
                            }
                        }
                    }
                    Ok(n) => {
                        let chunk: Vec<u8> = st.buf[..n].to_vec();
                        for line in st.decoder.push(&chunk) {
                            handle_line(
                                &line,
                                &mut st.pending,
                                &mut st.saw_error,
                                &mut st.streamed_any,
                            );
                        }
                    }
                    Err(e) => {
                        st.finished = true;
                        return Some((
                            Err(AppError::internal(format!(
                                "Claude Code output failed: {e}"
                            ))),
                            st,
                        ));
                    }
                }
            }
        });

        Ok(Box::pin(stream))
    }

    async fn send_message(&self, req: ChatRequest) -> Result<String> {
        // Reuse the streaming path and collect: the CLI has a non-streaming
        // mode, but one code path means one set of failure semantics.
        let mut stream = self.stream_message(req).await?;
        let mut text = String::new();
        while let Some(item) = stream.next().await {
            match item? {
                StreamEvent::TextDelta(t) => text.push_str(&t),
                StreamEvent::Failed(d) => return Err(AppError::Provider(Box::new(d))),
                _ => {}
            }
        }
        Ok(text)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(models().await.0)
    }

    async fn verify_credentials(&self) -> Result<()> {
        // Deliberately does not send a prompt: checking sign-in must not
        // consume the user's quota. A real auth failure surfaces with a clear
        // message on the first send.
        match Command::new(CLAUDE_BIN)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .await
        {
            Ok(out) if out.status.success() => Ok(()),
            Ok(_) => Err(AppError::Provider(Box::new(super::wire::describe_failure(
                "cli_missing",
                None,
            )))),
            Err(_) => Err(AppError::Provider(Box::new(super::wire::describe_failure(
                "cli_missing",
                None,
            )))),
        }
    }
}

fn handle_line(
    line: &str,
    pending: &mut std::collections::VecDeque<StreamEvent>,
    saw_error: &mut Option<String>,
    streamed_any: &mut bool,
) {
    match parse_line(line) {
        CliRecord::Stream(ev) => {
            if matches!(ev, StreamEvent::TextDelta(_)) {
                *streamed_any = true;
            }
            pending.push_back(ev)
        }
        CliRecord::RateLimited { message } => {
            pending.push_back(StreamEvent::Failed(super::wire::describe_failure(
                "rate_limit",
                Some(&message),
            )));
        }
        CliRecord::Done {
            is_error,
            error,
            text,
            ..
        } => {
            if is_error {
                *saw_error = Some(error.unwrap_or_else(|| "Claude Code reported an error.".into()));
            } else if let Some(t) = text {
                // Only when nothing streamed. On a normal turn `result`
                // repeats the whole answer, so emitting it as well would
                // duplicate every reply.
                if !*streamed_any {
                    pending.push_back(StreamEvent::TextDelta(t));
                }
            }
        }
        CliRecord::Init {
            session_id,
            model,
            slash_commands,
            tools,
        } => {
            pending.push_back(StreamEvent::SessionReady {
                session_id,
                model,
                slash_commands,
                tools,
            });
        }
        CliRecord::Ignored => {}
    }
}

async fn read_stderr(stderr: &mut Option<tokio::process::ChildStderr>) -> Option<String> {
    let mut s = stderr.take()?;
    let mut out = String::new();
    s.read_to_string(&mut out).await.ok()?;
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

/// Recognise the failures worth wording ourselves.
fn classify(detail: Option<&str>) -> &'static str {
    let d = detail.unwrap_or("").to_ascii_lowercase();
    if d.contains("not logged in") || d.contains("unauthor") || d.contains("authentication") {
        "authentication_error"
    } else if d.contains("rate limit") || d.contains("usage limit") {
        "rate_limit"
    } else if d.contains("not found") && d.contains("claude") {
        "cli_missing"
    } else {
        "cli_error"
    }
}

/// The aliases to offer, discovered from the CLI where possible.
pub async fn models() -> (Vec<ModelInfo>, ModelCatalog) {
    match discover_models().await {
        Some(catalog) => {
            let models = catalog
                .available
                .iter()
                .map(|id| ModelInfo {
                    id: id.clone(),
                    display_name: label_for(id),
                    created_at: None,
                    from_fallback: false,
                })
                .collect();
            (models, catalog)
        }
        None => (fallback_models(), ModelCatalog::default()),
    }
}

pub fn fallback_models() -> Vec<ModelInfo> {
    FALLBACK_ALIASES
        .iter()
        .map(|id| ModelInfo {
            id: (*id).to_string(),
            display_name: label_for(id),
            created_at: None,
            from_fallback: true,
        })
        .collect()
}

/// Turn an alias into something readable, without inventing detail: an
/// unrecognised alias is shown exactly as the CLI named it.
fn label_for(alias: &str) -> String {
    let (base, suffix) = match alias.strip_suffix("[1m]") {
        Some(b) => (b, " (1M context)"),
        None => (alias, ""),
    };
    let pretty = match base {
        "opus" => "Opus",
        "sonnet" => "Sonnet",
        "haiku" => "Haiku",
        "fable" => "Fable",
        "best" => "Best available",
        "default" => "Default",
        "opusplan" => "Opus (plan mode)",
        other => return format!("{other}{suffix}"),
    };
    format!("{pretty}{suffix}")
}

/// Is the CLI installed and runnable?
pub async fn is_installed() -> Option<String> {
    let out = Command::new(CLAUDE_BIN)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .await
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderMessage;

    fn req(text: &str) -> ChatRequest {
        ChatRequest {
            model: "sonnet".into(),
            system: None,
            messages: vec![ProviderMessage {
                role: "user",
                content: vec![ContentBlock::Text { text: text.into() }],
            }],
            max_tokens: 0,
            temperature: None,
            stop_sequences: vec![],
        }
    }

    #[test]
    fn a_thinking_tokens_system_record_reports_the_running_total() {
        let line = r#"{"type":"system","subtype":"thinking_tokens",
            "estimated_tokens":162,"estimated_tokens_delta":112,"session_id":"s"}"#;
        assert_eq!(
            parse_line(line),
            CliRecord::Stream(StreamEvent::ThinkingProgress(162))
        );
    }

    #[test]
    fn a_thinking_tokens_record_without_a_total_is_ignored() {
        let line = r#"{"type":"system","subtype":"thinking_tokens","session_id":"s"}"#;
        assert_eq!(parse_line(line), CliRecord::Ignored);
    }

    #[test]
    fn only_the_last_turn_is_sent_since_the_cli_owns_history() {
        let mut r = req("second");
        r.messages.insert(
            0,
            ProviderMessage {
                role: "user",
                content: vec![ContentBlock::Text {
                    text: "first".into(),
                }],
            },
        );
        let payload = input_message(&r).unwrap();
        assert!(payload.contains("second"));
        assert!(
            !payload.contains("first"),
            "replaying history would duplicate it"
        );
    }

    #[test]
    fn an_attached_image_survives_as_a_content_block() {
        // Flattening the turn to text would silently drop the image; it has
        // to cross as structured content for the model to see it.
        let mut r = req("what is this?");
        r.messages[0].content.insert(
            0,
            ContentBlock::Image {
                source: crate::provider::MediaSource::Base64 {
                    media_type: "image/png".into(),
                    data: "aGVsbG8=".into(),
                },
            },
        );

        let payload = input_message(&r).unwrap();
        let v: serde_json::Value = serde_json::from_str(payload.trim()).unwrap();
        assert_eq!(v["type"], "user");
        assert_eq!(v["message"]["role"], "user");

        let blocks = v["message"]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["type"], "image");
        assert_eq!(blocks[0]["source"]["type"], "base64");
        assert_eq!(blocks[0]["source"]["media_type"], "image/png");
        assert_eq!(blocks[0]["source"]["data"], "aGVsbG8=");
        assert_eq!(blocks[1]["type"], "text");
    }

    #[test]
    fn a_pdf_crosses_as_a_document_block() {
        let mut r = req("summarise this");
        r.messages[0].content.insert(
            0,
            ContentBlock::Document {
                source: crate::provider::MediaSource::Base64 {
                    media_type: "application/pdf".into(),
                    data: "JVBERi0=".into(),
                },
            },
        );
        let v: serde_json::Value = serde_json::from_str(input_message(&r).unwrap().trim()).unwrap();
        assert_eq!(v["message"]["content"][0]["type"], "document");
    }

    #[test]
    fn an_empty_turn_is_refused() {
        let mut r = req("   ");
        assert!(!has_content(&r));
        r.messages[0].content.clear();
        assert!(!has_content(&r));
    }

    #[test]
    fn structured_input_is_requested_from_the_cli() {
        let p = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let args: Vec<String> = p
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        // Without this the CLI reads stdin as plain text and attachments die.
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--input-format" && w[1] == "stream-json"));
    }

    #[test]
    fn parses_the_init_record_including_available_commands() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc",
                       "model":"claude-opus-5",
                       "slash_commands":["model","context","__internal","doctor","compact"],
                       "terminal_slash_commands":["doctor"]}"#;
        match parse_line(line) {
            CliRecord::Init {
                session_id,
                model,
                slash_commands,
                ..
            } => {
                assert_eq!(session_id, "abc");
                assert_eq!(model, "claude-opus-5");
                // Sorted, with internal and terminal-only entries dropped —
                // neither would do anything through a pipe.
                assert_eq!(slash_commands, vec!["compact", "context", "model"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_init_record_without_commands_is_still_fine() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc","model":"m"}"#;
        match parse_line(line) {
            CliRecord::Init { slash_commands, .. } => assert!(slash_commands.is_empty()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_lockdown_is_checked_against_what_the_session_reports() {
        // Everything we deny is expected; anything else is a hole a CLI
        // upgrade opened, and must be reported rather than ignored.
        assert!(unexpected_tools(&[]).is_empty());
        assert!(unexpected_tools(&["Bash".into(), "Read".into(), "Skill".into()]).is_empty());

        let holes = unexpected_tools(&[
            "Read".into(),
            "SomeNewTool".into(),
            "mcp__server__do_thing".into(),
        ]);
        assert_eq!(holes, vec!["SomeNewTool", "mcp__server__do_thing"]);
    }

    #[test]
    fn mcp_servers_are_excluded_from_chat_sessions() {
        // A configured MCP server would otherwise stay live — this user had
        // one that can delete documents.
        let p = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let args: Vec<String> = p
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == "--strict-mcp-config"));
    }

    #[test]
    fn the_built_in_tool_set_is_emptied_not_merely_denied() {
        // A deny list only denies what it was written against; a CLI upgrade
        // adding a tool would reopen the hole. `--tools ""` closes the set.
        // Measured against the real CLI: baseline 27 tools, this 0.
        let p = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let args: Vec<String> = p
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let at = args.iter().position(|a| a == "--tools").expect("--tools");
        assert_eq!(args[at + 1], "");
        // Anything that would have raised a prompt is denied, not hung:
        // there is no TTY to answer one.
        let at = args
            .iter()
            .position(|a| a == "--permission-prompts")
            .expect("--permission-prompts");
        assert_eq!(args[at + 1], "none");
    }

    #[test]
    fn effort_is_validated_before_becoming_a_flag() {
        let base = || ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let args = |p: ClaudeCodeProvider| -> Vec<String> {
            p.command(&req("hi"))
                .as_std()
                .get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        };

        let good = args(base().with_effort(Some("xhigh".into())));
        assert!(good
            .windows(2)
            .any(|w| w[0] == "--effort" && w[1] == "xhigh"));

        // A stale or bogus value must not make every request fail.
        let bogus = args(base().with_effort(Some("turbo".into())));
        assert!(!bogus.iter().any(|a| a == "--effort"));

        let none = args(base().with_effort(None));
        assert!(!none.iter().any(|a| a == "--effort"));
    }

    #[test]
    fn unwraps_a_stream_event_into_a_delta() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta",
                       "delta":{"type":"text_delta","text":"ok"}}}"#;
        assert_eq!(
            parse_line(line),
            CliRecord::Stream(StreamEvent::TextDelta("ok".into()))
        );
    }

    #[test]
    fn a_successful_result_is_not_an_error() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,
                       "result":"ok","session_id":"abc"}"#;
        match parse_line(line) {
            CliRecord::Done {
                is_error,
                session_id,
                ..
            } => {
                assert!(!is_error);
                assert_eq!(session_id.as_deref(), Some("abc"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_result_carries_its_reason() {
        let line = r#"{"type":"result","subtype":"error","is_error":true,
                       "result":"something broke","session_id":"abc"}"#;
        match parse_line(line) {
            CliRecord::Done {
                is_error, error, ..
            } => {
                assert!(is_error);
                assert_eq!(error.as_deref(), Some("something broke"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_allowed_rate_limit_event_is_not_surfaced() {
        // The CLI reports limit status on every turn; only blocking matters.
        let line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#;
        assert_eq!(parse_line(line), CliRecord::Ignored);
    }

    #[test]
    fn a_blocking_rate_limit_is_surfaced_with_a_reset_time() {
        let line = r#"{"type":"rate_limit_event","rate_limit_info":
                       {"status":"blocked","resetsAt":1789896600}}"#;
        match parse_line(line) {
            CliRecord::RateLimited { message } => {
                assert!(message.contains("usage limit"), "{message}");
                assert!(message.contains("Resets at"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_local_command_answer_arrives_only_in_the_result_record() {
        // `/context` and friends are answered by the CLI itself and never
        // stream, so `result` is the only place their output shows up.
        // r###: the payload contains `"##` (a quote then a markdown
        // heading), which would terminate a shorter raw-string literal.
        let line = r###"{"type":"result","subtype":"success","is_error":false,
                         "result":"## Context Usage\n\n12.3k / 1m","session_id":"abc"}"###;
        match parse_line(line) {
            CliRecord::Done { text, error, .. } => {
                assert_eq!(text.as_deref(), Some("## Context Usage\n\n12.3k / 1m"));
                assert!(error.is_none());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn result_text_is_only_emitted_when_nothing_streamed() {
        use std::collections::VecDeque;
        let result =
            r#"{"type":"result","subtype":"success","is_error":false,"result":"full answer"}"#;

        // Nothing streamed (a local command): the result text is the answer.
        let mut pending = VecDeque::new();
        let mut err = None;
        let mut streamed = false;
        handle_line(result, &mut pending, &mut err, &mut streamed);
        assert_eq!(
            pending.pop_front(),
            Some(StreamEvent::TextDelta("full answer".into()))
        );

        // Something streamed: `result` repeats the whole reply, so emitting
        // it again would duplicate every answer.
        let mut pending = VecDeque::new();
        let mut err = None;
        let mut streamed = true;
        handle_line(result, &mut pending, &mut err, &mut streamed);
        assert!(pending.is_empty());
    }

    #[test]
    fn parses_the_model_command_output() {
        let out = "Current model: `Opus 5 (1M context)` (effort: high)\n\
                   Usage: /model <name>. Available: sonnet, opus, haiku, fable, best, \
                   sonnet[1m], opus[1m], opusplan, default, or a full model ID.";
        let c = parse_model_output(out);
        assert_eq!(c.current.as_deref(), Some("Opus 5 (1M context)"));
        assert_eq!(c.effort.as_deref(), Some("high"));
        // The trailing prose is not an alias and must not become an option.
        assert!(!c.available.iter().any(|a| a.contains(' ')));
        assert!(c.available.contains(&"sonnet".to_string()));
        assert!(c.available.contains(&"opus[1m]".to_string()));
        assert!(c.available.contains(&"opusplan".to_string()));
        assert_eq!(c.available.len(), 9);
    }

    #[test]
    fn model_output_that_changed_shape_yields_empty_rather_than_wrong() {
        // Human-facing text can change between CLI versions; a bad parse
        // must not invent a model list.
        let c = parse_model_output("something entirely different");
        assert!(c.current.is_none());
        assert!(c.effort.is_none());
        assert!(c.available.is_empty());

        // Current model present but no list, and vice versa.
        let c = parse_model_output("Current model: `Sonnet 5` (effort: low)");
        assert_eq!(c.current.as_deref(), Some("Sonnet 5"));
        assert!(c.available.is_empty());
    }

    #[test]
    fn aliases_get_readable_labels_without_inventing_detail() {
        assert_eq!(label_for("opus"), "Opus");
        assert_eq!(label_for("sonnet[1m]"), "Sonnet (1M context)");
        assert_eq!(label_for("opusplan"), "Opus (plan mode)");
        // An alias added by a newer CLI is shown exactly as given.
        assert_eq!(label_for("claude-fable-9"), "claude-fable-9");
        assert_eq!(label_for("mystery[1m]"), "mystery (1M context)");
    }

    #[test]
    fn junk_and_unknown_records_are_ignored() {
        assert_eq!(parse_line("not json"), CliRecord::Ignored);
        assert_eq!(parse_line("{}"), CliRecord::Ignored);
        assert_eq!(
            parse_line(r#"{"type":"whatever_is_new"}"#),
            CliRecord::Ignored
        );
    }

    #[test]
    fn every_built_in_tool_is_disabled() {
        // A chat window must not be able to read files or run commands.
        let p = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let cmd = p.command(&req("hi"));
        let args: Vec<String> = cmd
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        let joined = args.join(" ");
        assert!(joined.contains("--disallowed-tools"));
        for tool in ["Bash", "Read", "Write", "Edit", "WebFetch"] {
            assert!(joined.contains(tool), "{tool} should be disallowed");
        }
        // No TTY, so a prompt would hang.
        assert!(joined.contains("dontAsk"));
        // --bare would force API-key auth and never read the user's login.
        assert!(!joined.contains("--bare"));
    }

    #[test]
    fn a_branch_forks_the_source_session_rather_than_resuming_it() {
        // Resuming the source directly would write the branch's turns into
        // the original conversation's history.
        let p = ClaudeCodeProvider::new(
            PathBuf::from("/tmp"),
            SessionRef::Fork {
                from: "source-1".into(),
            },
        );
        let args: Vec<String> = p
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--resume" && w[1] == "source-1"));
        assert!(args.iter().any(|a| a == "--fork-session"));
    }

    #[test]
    fn a_first_turn_claims_a_session_and_later_turns_resume_it() {
        let new = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("sess-1".into()));
        let args: Vec<String> = new
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--session-id" && w[1] == "sess-1"));

        let resumed =
            ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::Resume("sess-1".into()));
        let args: Vec<String> = resumed
            .command(&req("hi"))
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--resume" && w[1] == "sess-1"));
    }

    #[test]
    fn project_instructions_are_appended_not_substituted() {
        let mut r = req("hi");
        r.system = Some("Prefer async APIs.".into());
        let p = ClaudeCodeProvider::new(PathBuf::from("/tmp"), SessionRef::New("s".into()));
        let args: Vec<String> = p
            .command(&r)
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--append-system-prompt" && w[1] == "Prefer async APIs."));
        // Replacing it would discard the CLI's own behaviour.
        assert!(!args.iter().any(|a| a == "--system-prompt"));
    }
}
