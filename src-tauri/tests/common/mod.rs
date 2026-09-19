//! Shared helpers. Every test runs against a scratch database and a mock
//! HTTP server — nothing here ever needs a real Anthropic API key.

#![allow(dead_code)]

use openclaude_lib::db::models::*;
use openclaude_lib::db::repo;
use openclaude_lib::db::Db;

pub fn db() -> Db {
    let dir = tempfile::tempdir().unwrap();
    // Leak the temp dir for the life of the test process so the file stays
    // put; the OS cleans it up afterwards.
    let path = dir.keep().join("test.db");
    Db::open(&path).unwrap()
}

pub fn conversation(db: &Db, title: &str) -> Conversation {
    let conn = db.conn();
    repo::conversations::create(
        &conn,
        repo::conversations::NewConversation {
            title: Some(title.to_string()),
            model: "claude-sonnet-4-5".into(),
            project_id: None,
            system_prompt: None,
        },
    )
    .unwrap()
}

pub fn say(db: &Db, conversation_id: &str, role: Role, content: &str) -> Message {
    let conn = db.conn();
    repo::messages::insert(
        &conn,
        repo::messages::NewMessage {
            conversation_id,
            role,
            content,
            status: MessageStatus::Complete,
            model: None,
        },
    )
    .unwrap()
}

/// Build one Anthropic-shaped SSE body from a list of text chunks.
pub fn sse_body(chunks: &[&str], stop_reason: &str) -> String {
    let mut s = String::new();
    s.push_str(
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_test\",\
         \"model\":\"claude-sonnet-4-5\",\"usage\":{\"input_tokens\":11}}}\n\n",
    );
    s.push_str(
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0}\n\n",
    );
    for c in chunks {
        s.push_str(&format!(
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\
             \"delta\":{{\"type\":\"text_delta\",\"text\":{}}}}}\n\n",
            serde_json::to_string(c).unwrap()
        ));
    }
    s.push_str(
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    );
    s.push_str(&format!(
        "event: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"{stop_reason}\"}},\
         \"usage\":{{\"output_tokens\":7}}}}\n\n"
    ));
    s.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");
    s
}
