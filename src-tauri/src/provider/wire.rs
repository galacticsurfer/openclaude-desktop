//! Mapping Anthropic message-stream events onto our normalised [`StreamEvent`].
//!
//! The Claude Code CLI wraps these verbatim inside its `stream_event` records,
//! so this is shared vocabulary rather than anything CLI-specific — and it is
//! testable from a JSON string with no process and no network.

use super::StreamEvent;
use crate::error::ErrorDetail;
use serde::Deserialize;

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
}

#[derive(Deserialize)]
struct MessageStart {
    message: StartedMessage,
}
#[derive(Deserialize)]
struct StartedMessage {
    id: String,
    model: String,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct BlockDelta {
    delta: DeltaBody,
}
#[derive(Deserialize)]
#[serde(tag = "type")]
enum DeltaBody {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "thinking_delta")]
    Thinking {
        thinking: String,
        /// Claude Code sends this instead of the reasoning text.
        estimated_tokens: Option<i64>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct BlockStart {
    content_block: StartedBlock,
}
#[derive(Deserialize)]
struct StartedBlock {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct MessageDelta {
    delta: StopInfo,
    usage: Option<Usage>,
}
#[derive(Deserialize)]
struct StopInfo {
    stop_reason: Option<String>,
}

/// Translate one Anthropic stream event into a [`StreamEvent`].
///
/// Returns `None` for events we deliberately ignore (`ping`,
/// `content_block_stop`, non-thinking block starts, anything added upstream
/// later) so a new event type is a no-op rather than a failure.
pub fn map_event(kind: &str, raw: &serde_json::Value) -> Option<StreamEvent> {
    match kind {
        "message_start" => {
            let m: MessageStart = serde_json::from_value(raw.clone()).ok()?;
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
            let d: BlockDelta = serde_json::from_value(raw.clone()).ok()?;
            match d.delta {
                DeltaBody::Text { text } => Some(StreamEvent::TextDelta(text)),
                // Claude Code opens a thinking block, then streams empty
                // `thinking` strings carrying only a token estimate. Report
                // whichever of the two the provider actually gave us.
                DeltaBody::Thinking {
                    thinking,
                    estimated_tokens,
                } => {
                    if thinking.is_empty() {
                        estimated_tokens.map(StreamEvent::ThinkingProgress)
                    } else {
                        Some(StreamEvent::ThinkingDelta(thinking))
                    }
                }
                DeltaBody::Other => None,
            }
        }
        "content_block_start" => {
            let b: BlockStart = serde_json::from_value(raw.clone()).ok()?;
            (b.content_block.kind == "thinking").then_some(StreamEvent::ThinkingStarted)
        }
        "message_delta" => {
            let d: MessageDelta = serde_json::from_value(raw.clone()).ok()?;
            Some(StreamEvent::Completed {
                stop_reason: d.delta.stop_reason,
                output_tokens: d.usage.and_then(|u| u.output_tokens),
            })
        }
        _ => None,
    }
}

/// Turn a CLI or upstream failure into a user-facing error.
///
/// The wording is ours for recognised categories, so an unexpected upstream
/// string can never become the whole message the user sees.
pub fn describe_failure(kind: &str, detail: Option<&str>) -> ErrorDetail {
    let (message, retryable) = match kind {
        "rate_limit" | "rate_limit_error" => (
            "You've hit your Claude usage limit. It will reset shortly.".to_string(),
            true,
        ),
        "overloaded_error" => (
            "Claude is overloaded right now. Try again in a moment.".to_string(),
            true,
        ),
        "authentication_error" | "not_logged_in" => (
            "Claude Code is not signed in. Run `claude` in a terminal and sign in.".to_string(),
            false,
        ),
        "cli_missing" => (
            "Claude Code is not installed, or is not on your PATH.".to_string(),
            false,
        ),
        _ => match detail.map(str::trim).filter(|d| !d.is_empty()) {
            Some(d) => (truncate_for_display(d), false),
            None => ("Claude could not complete this request.".to_string(), false),
        },
    };

    ErrorDetail {
        kind: kind.to_string(),
        message,
        status: None,
        request_id: None,
        retryable,
        retry_after_secs: None,
    }
}

/// Diagnostics can be long; an error card is not a log viewer.
fn truncate_for_display(s: &str) -> String {
    const MAX: usize = 300;
    if s.chars().count() <= MAX {
        return s.to_string();
    }
    let kept: String = s.chars().take(MAX).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_a_message_start_with_usage() {
        let v = json!({
            "type": "message_start",
            "message": { "id": "msg_1", "model": "claude-opus-5",
                         "usage": { "input_tokens": 2, "cache_read_input_tokens": 8628,
                                    "cache_creation_input_tokens": 7247 } }
        });
        match map_event("message_start", &v).unwrap() {
            StreamEvent::Started {
                message_id,
                input_tokens,
                cache_read,
                cache_write,
                ..
            } => {
                assert_eq!(message_id, "msg_1");
                assert_eq!(input_tokens, Some(2));
                assert_eq!(cache_read, Some(8628));
                assert_eq!(cache_write, Some(7247));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn maps_text_and_thinking_deltas_onto_separate_channels() {
        let text = json!({"delta": {"type": "text_delta", "text": "ok"}});
        assert_eq!(
            map_event("content_block_delta", &text),
            Some(StreamEvent::TextDelta("ok".into()))
        );
        let thinking = json!({"delta": {"type": "thinking_delta", "thinking": "hmm"}});
        assert_eq!(
            map_event("content_block_delta", &thinking),
            Some(StreamEvent::ThinkingDelta("hmm".into()))
        );
    }

    #[test]
    fn a_thinking_block_opening_is_announced() {
        let raw = json!({"content_block": {"type": "thinking", "thinking": "", "signature": ""}});
        assert_eq!(
            map_event("content_block_start", &raw),
            Some(StreamEvent::ThinkingStarted)
        );
    }

    #[test]
    fn a_text_block_opening_is_not_mistaken_for_thinking() {
        let raw = json!({"content_block": {"type": "text", "text": ""}});
        assert_eq!(map_event("content_block_start", &raw), None);
    }

    #[test]
    fn an_empty_thinking_delta_reports_the_token_estimate_instead() {
        // What Claude Code actually sends: no reasoning text, just a count.
        let raw =
            json!({"delta": {"type": "thinking_delta", "thinking": "", "estimated_tokens": 50}});
        assert_eq!(
            map_event("content_block_delta", &raw),
            Some(StreamEvent::ThinkingProgress(50))
        );
    }

    #[test]
    fn an_empty_thinking_delta_with_no_estimate_is_ignored() {
        let raw =
            json!({"delta": {"type": "thinking_delta", "thinking": "", "estimated_tokens": null}});
        assert_eq!(map_event("content_block_delta", &raw), None);
    }

    #[test]
    fn reasoning_text_still_wins_when_a_provider_sends_it() {
        let raw =
            json!({"delta": {"type": "thinking_delta", "thinking": "hmm", "estimated_tokens": 9}});
        assert_eq!(
            map_event("content_block_delta", &raw),
            Some(StreamEvent::ThinkingDelta("hmm".into()))
        );
    }

    #[test]
    fn maps_a_message_delta_to_completion() {
        let v = json!({"delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 4}});
        assert_eq!(
            map_event("message_delta", &v),
            Some(StreamEvent::Completed {
                stop_reason: Some("end_turn".into()),
                output_tokens: Some(4)
            })
        );
    }

    #[test]
    fn ignores_events_it_does_not_model() {
        // A new upstream event type must be a no-op, not a crash.
        assert!(map_event("ping", &json!({})).is_none());
        assert!(map_event("content_block_start", &json!({"index": 0})).is_none());
        assert!(map_event("some_future_event", &json!({"x": 1})).is_none());
        // An input-json delta belongs to tool use, which this client ignores.
        let tool = json!({"delta": {"type": "input_json_delta", "partial_json": "{"}});
        assert!(map_event("content_block_delta", &tool).is_none());
    }

    #[test]
    fn malformed_payloads_are_ignored_rather_than_panicking() {
        assert!(map_event("message_start", &json!({"message": "not an object"})).is_none());
        assert!(map_event("message_delta", &json!(null)).is_none());
    }

    #[test]
    fn known_failures_get_our_wording_and_the_right_retryability() {
        let rl = describe_failure("rate_limit", None);
        assert!(rl.retryable);
        assert!(rl.message.contains("usage limit"), "{}", rl.message);

        let auth = describe_failure("authentication_error", None);
        assert!(!auth.retryable);
        assert!(auth.message.contains("sign in"), "{}", auth.message);
    }

    #[test]
    fn unknown_failures_fall_back_to_the_detail_but_bounded() {
        let d = describe_failure("weird", Some("something specific went wrong"));
        assert_eq!(d.message, "something specific went wrong");

        let long = "x".repeat(1000);
        let d = describe_failure("weird", Some(&long));
        assert!(
            d.message.chars().count() <= 301,
            "error text must stay bounded"
        );

        let empty = describe_failure("weird", Some("   "));
        assert!(empty.message.contains("could not complete"));
    }
}
