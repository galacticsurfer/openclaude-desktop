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

/// Built-in tools are all disabled: this is a conversation, not an agent.
/// Named explicitly rather than relying on `--restricted`, which only removes
/// the command-running tools and still leaves file access.
const DISALLOWED_TOOLS: &[&str] = &[
    "Bash",
    "Read",
    "Write",
    "Edit",
    "NotebookEdit",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Task",
    "TodoWrite",
];

/// Model aliases the CLI accepts. Ids are what we store; labels are display
/// only. Kept short deliberately — the CLI resolves an alias to whatever the
/// current model behind it is, so this does not go stale the way a pinned
/// list would.
/// Accepted `--effort` values. Validated rather than passed through, so a
/// stale setting cannot make every request fail.
pub const EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

const MODEL_ALIASES: &[(&str, &str)] = &[
    ("opus", "Claude Opus"),
    ("sonnet", "Claude Sonnet"),
    ("haiku", "Claude Haiku"),
];

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
            .arg("--disallowed-tools")
            .arg(DISALLOWED_TOOLS.join(" "))
            // No TTY here, so a permission prompt would hang forever. With
            // every tool disabled there is nothing left to ask about.
            .arg("--permission-mode")
            .arg("dontAsk");

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
        }

        cmd.current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd
    }
}

/// Flatten the request into the single prompt the CLI reads from stdin.
///
/// Only the trailing user turn is sent: the CLI owns conversation history
/// through its session, so replaying earlier turns would duplicate them.
pub fn prompt_for(req: &ChatRequest) -> String {
    let Some(last) = req.messages.last() else {
        return String::new();
    };
    last.content
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => text.clone(),
            // Images and PDFs cannot cross the CLI's text stdin. They are
            // rejected before we get here; this is belt and braces.
            ContentBlock::Image { .. } => "[image omitted]".to_string(),
            ContentBlock::Document { .. } => "[document omitted]".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
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

            CliRecord::Init {
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
        let prompt = prompt_for(&req);
        if prompt.trim().is_empty() {
            return Err(AppError::invalid("There is nothing to send."));
        }

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
            let bytes = prompt.into_bytes();
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
        Ok(models())
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
        } => {
            pending.push_back(StreamEvent::SessionReady {
                session_id,
                model,
                slash_commands,
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

pub fn models() -> Vec<ModelInfo> {
    MODEL_ALIASES
        .iter()
        .map(|(id, label)| ModelInfo {
            id: (*id).to_string(),
            display_name: (*label).to_string(),
            created_at: None,
            from_fallback: false,
        })
        .collect()
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
        let p = prompt_for(&r);
        assert_eq!(p, "second");
        assert!(!p.contains("first"), "replaying history would duplicate it");
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
