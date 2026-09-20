//! End-to-end provider tests against a mock Anthropic server.
//!
//! No test in this file requires a real API key or makes a real network call.

mod common;

use common::sse_body;
use futures_util::StreamExt;
use openclaude_lib::provider::{
    anthropic::AnthropicProvider, AIProvider, ChatRequest, ContentBlock, ProviderMessage,
    StreamEvent,
};
use openclaude_lib::secrets::Credential;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn req(model: &str) -> ChatRequest {
    ChatRequest {
        model: model.into(),
        system: None,
        messages: vec![ProviderMessage {
            role: "user",
            content: vec![ContentBlock::Text {
                text: "hello".into(),
            }],
        }],
        max_tokens: 1024,
        temperature: None,
        stop_sequences: vec![],
    }
}

fn provider(server: &MockServer) -> AnthropicProvider {
    AnthropicProvider::new(
        Credential::ApiKey("sk-ant-test-0123456789abcdef".into()),
        Some(server.uri()),
    )
    .unwrap()
}

fn oauth_provider(server: &MockServer) -> AnthropicProvider {
    AnthropicProvider::new(
        Credential::Oauth("sk-ant-oat01-test-token".into()),
        Some(server.uri()),
    )
    .unwrap()
}

/// `EventStream` is not `Debug`, so `unwrap_err` is unavailable on it.
async fn expect_err(
    r: Result<openclaude_lib::provider::EventStream, openclaude_lib::error::AppError>,
) -> openclaude_lib::error::AppError {
    match r {
        Ok(_) => panic!("expected the request to fail"),
        Err(e) => e,
    }
}

async fn collect(server: &MockServer) -> Vec<StreamEvent> {
    let mut s = provider(server)
        .stream_message(req("claude-sonnet-4-5"))
        .await
        .unwrap();
    let mut out = Vec::new();
    while let Some(e) = s.next().await {
        out.push(e.unwrap());
    }
    out
}

#[tokio::test]
async fn streams_text_deltas_in_order_with_usage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("x-api-key", "sk-ant-test-0123456789abcdef"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body(
                    &["Write-ahead ", "logging ", "batches writes."],
                    "end_turn",
                )),
        )
        .mount(&server)
        .await;

    let events = collect(&server).await;

    match &events[0] {
        StreamEvent::Started {
            message_id,
            input_tokens,
            ..
        } => {
            assert_eq!(message_id, "msg_test");
            assert_eq!(*input_tokens, Some(11));
        }
        other => panic!("expected Started, got {other:?}"),
    }

    let text: String = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Write-ahead logging batches writes.");

    match events.last().unwrap() {
        StreamEvent::Completed {
            stop_reason,
            output_tokens,
        } => {
            assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            assert_eq!(*output_tokens, Some(7));
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_event_types_are_ignored_rather_than_fatal() {
    let server = MockServer::start().await;
    let body = format!(
        "event: some_future_event\ndata: {{\"whatever\":true}}\n\n{}",
        sse_body(&["ok"], "end_turn")
    );
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let events = collect(&server).await;
    assert!(events
        .iter()
        .any(|e| matches!(e, StreamEvent::TextDelta(t) if t == "ok")));
}

#[tokio::test]
async fn an_error_frame_mid_stream_becomes_a_failed_event() {
    let server = MockServer::start().await;
    let body = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"x\"}}\n\n\
                event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let events = collect(&server).await;
    match events.last().unwrap() {
        StreamEvent::Failed(d) => {
            assert_eq!(d.kind, "overloaded_error");
            assert!(d.retryable, "an overload should be retryable");
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[tokio::test]
async fn thinking_deltas_are_reported_on_their_own_channel() {
    let server = MockServer::start().await;
    let body = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\
                \"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"considering\"}}\n\n\
                event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\
                \"delta\":{\"type\":\"text_delta\",\"text\":\"answer\"}}\n\n";
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let events = collect(&server).await;
    assert_eq!(events[0], StreamEvent::ThinkingDelta("considering".into()));
    assert_eq!(events[1], StreamEvent::TextDelta("answer".into()));
}

#[tokio::test]
async fn a_rejected_key_produces_an_actionable_message_and_is_not_retryable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("request-id", "req_abc123")
                .set_body_string(r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#),
        )
        .mount(&server)
        .await;

    let err = expect_err(
        provider(&server)
            .stream_message(req("claude-sonnet-4-5"))
            .await,
    )
    .await;

    let detail = err.detail().expect("provider error carries detail");
    assert_eq!(detail.status, Some(401));
    assert_eq!(detail.request_id.as_deref(), Some("req_abc123"));
    assert!(!detail.retryable);
    // Actionable wording, and no echo of the raw upstream string.
    assert!(err.to_string().contains("Settings"), "{err}");
}

#[tokio::test]
async fn rate_limiting_is_retryable_and_carries_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "30")
                .set_body_string(
                    r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
                ),
        )
        .mount(&server)
        .await;

    let err = expect_err(provider(&server).stream_message(req("m")).await).await;
    let d = err.detail().unwrap();
    assert!(d.retryable);
    assert_eq!(d.retry_after_secs, Some(30));
}

#[tokio::test]
async fn a_server_error_is_retryable_and_never_leaks_the_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream exploded"))
        .mount(&server)
        .await;

    let err = expect_err(provider(&server).stream_message(req("m")).await).await;
    assert!(err.retryable());

    let serialised = serde_json::to_string(&err).unwrap();
    assert!(
        !serialised.contains("sk-ant"),
        "error payload must never carry the key"
    );
}

#[tokio::test]
async fn non_streaming_send_returns_concatenated_text_blocks() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"content":[{"type":"text","text":"Debug Mongo "},{"type":"text","text":"timeout"}]}"#,
        ))
        .mount(&server)
        .await;

    let out = provider(&server).send_message(req("m")).await.unwrap();
    assert_eq!(out, "Debug Mongo timeout");
}

#[tokio::test]
async fn model_listing_maps_ids_and_display_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(query_param("limit", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"data":[{"id":"claude-opus-4-5","display_name":"Claude Opus 4.5"},
                        {"id":"claude-haiku-4-5"}]}"#,
        ))
        .mount(&server)
        .await;

    let models = provider(&server).list_models().await.unwrap();
    assert_eq!(models[0].id, "claude-opus-4-5");
    assert_eq!(models[0].display_name, "Claude Opus 4.5");
    // Missing display names fall back to the id rather than being blank.
    assert_eq!(models[1].display_name, "claude-haiku-4-5");
    assert!(models.iter().all(|m| !m.from_fallback));
}

#[tokio::test]
async fn verify_credentials_distinguishes_a_good_key_from_a_bad_one() {
    let good = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"data":[]}"#))
        .mount(&good)
        .await;
    assert!(provider(&good).verify_credentials().await.is_ok());

    let bad = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"nope"}}"#,
        ))
        .mount(&bad)
        .await;
    assert!(provider(&bad).verify_credentials().await.is_err());
}

#[tokio::test]
async fn an_unreachable_host_reports_as_offline() {
    // Port 1 is reserved and refuses immediately.
    let p = AnthropicProvider::new(
        Credential::ApiKey("sk-ant-x0123456789abcdef".into()),
        Some("http://127.0.0.1:1".into()),
    )
    .unwrap();
    let err = expect_err(p.stream_message(req("m")).await).await;
    assert_eq!(err.kind(), "offline");
    assert!(err.retryable());
}

#[tokio::test]
async fn the_request_body_carries_system_prompt_and_limits() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse_body(&["ok"], "end_turn")))
        .mount(&server)
        .await;

    let mut r = req("claude-opus-4-5");
    r.system = Some("You are terse.".into());
    r.max_tokens = 4096;
    r.temperature = Some(0.2);
    let mut s = provider(&server).stream_message(r).await.unwrap();
    while s.next().await.is_some() {}

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(body["model"], "claude-opus-4-5");
    assert_eq!(body["system"], "You are terse.");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["stream"], true);
    // Absent fields must be omitted, not sent as null.
    assert!(body.get("stop_sequences").is_none());
}

// --- credential handling --------------------------------------------------

#[tokio::test]
async fn an_api_key_is_sent_as_x_api_key_and_nothing_else() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse_body(&["ok"], "end_turn")))
        .mount(&server)
        .await;

    let mut s = provider(&server).stream_message(req("m")).await.unwrap();
    while s.next().await.is_some() {}

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        sent.headers.get("x-api-key").unwrap(),
        "sk-ant-test-0123456789abcdef"
    );
    // Sending both credential headers is rejected by the API.
    assert!(sent.headers.get("authorization").is_none());
}

#[tokio::test]
async fn an_oauth_token_is_sent_as_a_bearer_with_the_beta_opt_in() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse_body(&["ok"], "end_turn")))
        .mount(&server)
        .await;

    let mut s = oauth_provider(&server)
        .stream_message(req("m"))
        .await
        .unwrap();
    while s.next().await.is_some() {}

    let sent = &server.received_requests().await.unwrap()[0];
    assert_eq!(
        sent.headers.get("authorization").unwrap(),
        "Bearer sk-ant-oat01-test-token"
    );
    // /v1/messages rejects an OAuth token without this opt-in.
    assert_eq!(
        sent.headers.get("anthropic-beta").unwrap(),
        "oauth-2025-04-20"
    );
    // An OAuth token in x-api-key would be rejected, and sending both is too.
    assert!(sent.headers.get("x-api-key").is_none());
}

#[tokio::test]
async fn oauth_is_used_for_plain_requests_too_not_just_streaming() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"data":[]}"#))
        .mount(&server)
        .await;

    oauth_provider(&server).verify_credentials().await.unwrap();

    let sent = &server.received_requests().await.unwrap()[0];
    assert!(sent.headers.get("authorization").is_some());
    assert_eq!(
        sent.headers.get("anthropic-beta").unwrap(),
        "oauth-2025-04-20"
    );
}
