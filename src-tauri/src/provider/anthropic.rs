//! Anthropic Messages API client.
//!
//! Lives in Rust rather than the webview on purpose: the API key never enters
//! the renderer process, so the CSP can forbid the frontend from making any
//! outbound request at all, and a webview reload cannot kill an in-flight
//! response.

use super::sse::SseDecoder;
use super::{AIProvider, ChatRequest, EventStream, ModelInfo, StreamEvent};
use crate::error::{AppError, ErrorDetail, Result};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
pub const API_VERSION: &str = "2023-06-01";

/// Used when `/v1/models` cannot be reached (offline first launch). Marked
/// `from_fallback` so the UI labels it rather than presenting it as truth.
/// The live list always wins once a request succeeds.
pub const FALLBACK_MODELS: &[(&str, &str)] = &[
    ("claude-opus-4-5", "Claude Opus 4.5"),
    ("claude-sonnet-4-5", "Claude Sonnet 4.5"),
    ("claude-haiku-4-5", "Claude Haiku 4.5"),
];

pub struct AnthropicProvider {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            // Generous: a long answer legitimately streams for minutes. The
            // per-read timeout below is what actually catches a dead socket.
            .connect_timeout(std::time::Duration::from_secs(15))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .user_agent(concat!("OpenClaudeDesktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AppError::internal(format!("Could not create HTTP client: {e}")))?;

        Ok(Self {
            http,
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
    }

    fn body(req: &ChatRequest, stream: bool) -> serde_json::Value {
        let mut body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "messages": req.messages,
            "stream": stream,
        });
        if let Some(s) = &req.system {
            if !s.trim().is_empty() {
                body["system"] = json!(s);
            }
        }
        if let Some(t) = req.temperature {
            body["temperature"] = json!(t);
        }
        if !req.stop_sequences.is_empty() {
            body["stop_sequences"] = json!(req.stop_sequences);
        }
        body
    }
}

/// Map a transport failure onto something a user can act on.
fn transport_error(e: reqwest::Error) -> AppError {
    if e.is_connect() || e.is_timeout() {
        AppError::Offline
    } else {
        AppError::Provider(Box::new(ErrorDetail {
            kind: "network".into(),
            // `reqwest`'s Display includes the URL but never headers, so no
            // key can leak here.
            message: "The connection to Claude was lost.".into(),
            status: None,
            request_id: None,
            retryable: true,
            retry_after_secs: None,
        }))
    }
}

fn request_id(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
}

/// Turn a non-2xx response into a user-facing error.
///
/// The body is parsed for Anthropic's `{"error":{"type","message"}}` shape but
/// the *message we show* is our own wording for known categories, so an
/// unexpected upstream string can never become the whole UI.
async fn http_error(resp: reqwest::Response) -> AppError {
    let status = resp.status();
    let rid = request_id(resp.headers());
    let ra = retry_after(resp.headers());
    let body = resp.text().await.unwrap_or_default();

    #[derive(Deserialize)]
    struct Envelope {
        error: Inner,
    }
    #[derive(Deserialize)]
    struct Inner {
        #[serde(rename = "type")]
        kind: String,
        message: String,
    }

    let (kind, upstream) = serde_json::from_str::<Envelope>(&body)
        .map(|e| (e.error.kind, e.error.message))
        .unwrap_or_else(|_| (format!("http_{}", status.as_u16()), String::new()));

    let (message, retryable) = match (status.as_u16(), kind.as_str()) {
        (401, _) | (_, "authentication_error") => (
            "Your API key was rejected. Check it in Settings → Claude.".to_string(),
            false,
        ),
        (403, _) | (_, "permission_error") => (
            "This API key is not allowed to use that model.".to_string(),
            false,
        ),
        (404, _) => (
            "That model is not available to your account.".to_string(),
            false,
        ),
        (413, _) | (_, "request_too_large") => (
            "The conversation and its attachments are too large to send.".to_string(),
            false,
        ),
        (429, _) | (_, "rate_limit_error") => (
            "Rate limit reached. Claude asked us to slow down.".to_string(),
            true,
        ),
        (529, _) | (_, "overloaded_error") => (
            "Claude is overloaded right now. Try again in a moment.".to_string(),
            true,
        ),
        (s, _) if s >= 500 => ("Claude had a server error.".to_string(), true),
        (_, "invalid_request_error") if !upstream.is_empty() => {
            // Genuinely useful to show: it names the offending field.
            (format!("Claude rejected the request: {upstream}"), false)
        }
        _ => ("Claude could not complete this request.".to_string(), false),
    };

    AppError::Provider(Box::new(ErrorDetail {
        kind,
        message,
        status: Some(status.as_u16()),
        request_id: rid,
        retryable,
        retry_after_secs: ra,
    }))
}

// --- streaming wire types -------------------------------------------------

#[derive(Deserialize)]
struct WireUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
}

#[derive(Deserialize)]
struct WireMessageStart {
    message: WireMessage,
}
#[derive(Deserialize)]
struct WireMessage {
    id: String,
    model: String,
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct WireDelta {
    delta: DeltaBody,
}
#[derive(Deserialize)]
#[serde(tag = "type")]
enum DeltaBody {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct WireMessageDelta {
    delta: StopInfo,
    usage: Option<WireUsage>,
}
#[derive(Deserialize)]
struct StopInfo {
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireError {
    error: WireErrorInner,
}
#[derive(Deserialize)]
struct WireErrorInner {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

/// Translate one decoded SSE frame into a normalised [`StreamEvent`].
///
/// Returns `None` for frames we deliberately ignore (`ping`,
/// `content_block_start/stop`, unknown future event types) so that a new
/// upstream event type is a no-op rather than a crash.
pub fn map_sse(event: &str, data: &str) -> Option<StreamEvent> {
    match event {
        "message_start" => {
            let m: WireMessageStart = serde_json::from_str(data).ok()?;
            let u = m.message.usage;
            Some(StreamEvent::Started {
                message_id: m.message.id,
                model: m.message.model,
                input_tokens: u.as_ref().and_then(|u| u.input_tokens),
                cache_read: u.as_ref().and_then(|u| u.cache_read_input_tokens),
                cache_write: u.as_ref().and_then(|u| u.cache_creation_input_tokens),
            })
        }
        "content_block_delta" => {
            let d: WireDelta = serde_json::from_str(data).ok()?;
            match d.delta {
                DeltaBody::Text { text } => Some(StreamEvent::TextDelta(text)),
                DeltaBody::Thinking { thinking } => Some(StreamEvent::ThinkingDelta(thinking)),
                DeltaBody::Other => None,
            }
        }
        "message_delta" => {
            let d: WireMessageDelta = serde_json::from_str(data).ok()?;
            Some(StreamEvent::Completed {
                stop_reason: d.delta.stop_reason,
                output_tokens: d.usage.and_then(|u| u.output_tokens),
            })
        }
        "error" => {
            let e: WireError = serde_json::from_str(data).ok()?;
            let retryable = matches!(
                e.error.kind.as_str(),
                "overloaded_error" | "api_error" | "rate_limit_error"
            );
            Some(StreamEvent::Failed(ErrorDetail {
                message: match e.error.kind.as_str() {
                    "overloaded_error" => {
                        "Claude is overloaded right now. Try again in a moment.".into()
                    }
                    "rate_limit_error" => "Rate limit reached mid-response.".into(),
                    _ => format!("Claude stopped early: {}", e.error.message),
                },
                kind: e.error.kind,
                status: None,
                request_id: None,
                retryable,
                retry_after_secs: None,
            }))
        }
        // `message_stop`, `ping`, `content_block_start`, `content_block_stop`
        // and anything added upstream later.
        _ => None,
    }
}

#[derive(Deserialize)]
struct ModelsPage {
    data: Vec<WireModel>,
}
#[derive(Deserialize)]
struct WireModel {
    id: String,
    display_name: Option<String>,
    created_at: Option<String>,
}

#[derive(Deserialize)]
struct NonStreamResponse {
    content: Vec<NonStreamBlock>,
}
#[derive(Deserialize)]
struct NonStreamBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

/// Read timeout *between chunks*, not for the whole response. Anthropic sends
/// `ping` frames during long thinking pauses, so silence this long means the
/// socket is dead rather than the model being slow.
const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Drive a byte stream through the SSE decoder into normalised events.
fn event_stream<S>(bytes: S) -> impl futures_util::Stream<Item = Result<StreamEvent>> + Send
where
    S: futures_util::Stream<Item = reqwest::Result<bytes::Bytes>> + Send + 'static,
{
    struct State<S> {
        bytes: std::pin::Pin<Box<S>>,
        decoder: SseDecoder,
        pending: std::collections::VecDeque<StreamEvent>,
        done: bool,
    }

    let state = State {
        bytes: Box::pin(bytes),
        decoder: SseDecoder::new(),
        pending: std::collections::VecDeque::new(),
        done: false,
    };

    futures_util::stream::unfold(state, |mut st| async move {
        loop {
            // Always drain already-decoded events before touching the socket,
            // so one TCP chunk holding several frames yields all of them.
            if let Some(ev) = st.pending.pop_front() {
                return Some((Ok(ev), st));
            }
            if st.done {
                return None;
            }

            match tokio::time::timeout(IDLE_TIMEOUT, st.bytes.next()).await {
                Err(_) => {
                    st.done = true;
                    return Some((
                        Err(AppError::Provider(Box::new(ErrorDetail {
                            kind: "timeout".into(),
                            message: "Claude stopped sending data.".into(),
                            status: None,
                            request_id: None,
                            retryable: true,
                            retry_after_secs: None,
                        }))),
                        st,
                    ));
                }
                Ok(Some(Ok(chunk))) => {
                    for frame in st.decoder.push(&chunk) {
                        if let Some(ev) = map_sse(&frame.event, &frame.data) {
                            st.pending.push_back(ev);
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    st.done = true;
                    return Some((Err(transport_error(e)), st));
                }
                // Clean end of body: loop once more to flush `pending`, then stop.
                Ok(None) => st.done = true,
            }
        }
    })
}

impl AIProvider for AnthropicProvider {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    async fn stream_message(&self, req: ChatRequest) -> Result<EventStream> {
        let resp = self
            .request(reqwest::Method::POST, "/v1/messages")
            .json(&Self::body(&req, true))
            .send()
            .await
            .map_err(transport_error)?;

        if !resp.status().is_success() {
            return Err(http_error(resp).await);
        }

        Ok(Box::pin(event_stream(resp.bytes_stream())))
    }

    async fn send_message(&self, req: ChatRequest) -> Result<String> {
        let resp = self
            .request(reqwest::Method::POST, "/v1/messages")
            .json(&Self::body(&req, false))
            .send()
            .await
            .map_err(transport_error)?;

        if !resp.status().is_success() {
            return Err(http_error(resp).await);
        }

        let parsed: NonStreamResponse = resp.json().await.map_err(transport_error)?;
        Ok(parsed
            .content
            .into_iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join(""))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .request(reqwest::Method::GET, "/v1/models?limit=100")
            .send()
            .await
            .map_err(transport_error)?;

        if !resp.status().is_success() {
            return Err(http_error(resp).await);
        }

        let page: ModelsPage = resp.json().await.map_err(transport_error)?;
        Ok(page
            .data
            .into_iter()
            .map(|m| ModelInfo {
                display_name: m.display_name.unwrap_or_else(|| m.id.clone()),
                id: m.id,
                created_at: m.created_at,
                from_fallback: false,
            })
            .collect())
    }

    async fn verify_credentials(&self) -> Result<()> {
        // `/v1/models` is free and proves the key is live, which a zero-token
        // Messages call would not do without being billed.
        let resp = self
            .request(reqwest::Method::GET, "/v1/models?limit=1")
            .send()
            .await
            .map_err(transport_error)?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(http_error(resp).await)
        }
    }
}

pub fn fallback_models() -> Vec<ModelInfo> {
    FALLBACK_MODELS
        .iter()
        .map(|(id, name)| ModelInfo {
            id: (*id).to_string(),
            display_name: (*name).to_string(),
            created_at: None,
            from_fallback: true,
        })
        .collect()
}
