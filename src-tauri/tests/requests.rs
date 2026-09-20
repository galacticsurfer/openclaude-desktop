//! Turning stored history into a valid Messages API request.

mod common;

use common::*;
use openclaude_lib::chat::{
    build_messages, clean_generated_title, compose_system, fallback_title, rebuilding_context,
    render_prior_turns,
};
use openclaude_lib::db::models::*;
use openclaude_lib::db::repo;
use serde_json::json;

fn store() -> std::path::PathBuf {
    std::path::PathBuf::from("/nonexistent-attachment-store")
}

fn msg(role: Role, content: &str, status: MessageStatus) -> Message {
    Message {
        id: new_id(),
        conversation_id: "c".into(),
        seq: 0,
        role,
        content: content.into(),
        thinking: None,
        thinking_tokens: None,
        status,
        model: None,
        provider_message_id: None,
        stop_reason: None,
        input_tokens: None,
        output_tokens: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
        error_kind: None,
        error_message: None,
        created_at: 0,
        updated_at: 0,
        metadata: serde_json::json!({}),
        attachments: vec![],
    }
}

fn text_of(b: &openclaude_lib::provider::ContentBlock) -> String {
    match b {
        openclaude_lib::provider::ContentBlock::Text { text } => text.clone(),
        _ => String::new(),
    }
}

#[test]
fn a_simple_exchange_maps_one_to_one() {
    let h = vec![
        msg(Role::User, "why?", MessageStatus::Complete),
        msg(Role::Assistant, "because", MessageStatus::Complete),
        msg(Role::User, "and then?", MessageStatus::Complete),
    ];
    let out = build_messages(&h, &store()).unwrap();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].role, "user");
    assert_eq!(out[1].role, "assistant");
    assert_eq!(text_of(&out[2].content[0]), "and then?");
}

#[test]
fn consecutive_same_role_turns_are_merged_so_roles_alternate() {
    // Happens after a failed generation: two user turns in a row.
    let h = vec![
        msg(Role::User, "first", MessageStatus::Complete),
        msg(Role::User, "second", MessageStatus::Complete),
        msg(Role::Assistant, "reply", MessageStatus::Complete),
    ];
    let out = build_messages(&h, &store()).unwrap();
    assert_eq!(out.len(), 2, "the API rejects two user turns in a row");
    assert_eq!(out[0].role, "user");
    assert_eq!(out[0].content.len(), 2);
    assert_eq!(text_of(&out[0].content[1]), "second");
}

#[test]
fn a_leading_assistant_turn_is_dropped() {
    // The API requires the first message to be from the user.
    let h = vec![
        msg(
            Role::Assistant,
            "orphaned greeting",
            MessageStatus::Complete,
        ),
        msg(Role::User, "hello", MessageStatus::Complete),
    ];
    let out = build_messages(&h, &store()).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].role, "user");
}

#[test]
fn empty_failures_are_excluded_but_partial_answers_are_kept() {
    let h = vec![
        msg(Role::User, "q1", MessageStatus::Complete),
        msg(Role::Assistant, "", MessageStatus::Error),
        msg(Role::User, "q2", MessageStatus::Complete),
        msg(Role::Assistant, "half an ans", MessageStatus::Interrupted),
        msg(Role::User, "continue", MessageStatus::Complete),
    ];
    let out = build_messages(&h, &store()).unwrap();
    // q1 and q2 merge (the failed reply between them contributes nothing),
    // then the partial answer, then the follow-up.
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].role, "user");
    assert_eq!(out[0].content.len(), 2);
    assert_eq!(out[1].role, "assistant");
    assert_eq!(text_of(&out[1].content[0]), "half an ans");
}

#[test]
fn in_flight_rows_are_never_replayed() {
    let h = vec![
        msg(Role::User, "q", MessageStatus::Complete),
        // The placeholder we are about to fill must not be sent back.
        msg(Role::Assistant, "", MessageStatus::Streaming),
    ];
    let out = build_messages(&h, &store()).unwrap();
    assert_eq!(out.len(), 1);
}

#[test]
fn stored_system_rows_do_not_become_turns() {
    let h = vec![
        msg(Role::System, "be terse", MessageStatus::Complete),
        msg(Role::User, "hi", MessageStatus::Complete),
    ];
    let out = build_messages(&h, &store()).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].role, "user");
}

#[test]
fn project_instructions_precede_conversation_instructions() {
    assert_eq!(
        compose_system(Some("Use Python 3.12."), Some("Be terse.")),
        Some("Use Python 3.12.\n\nBe terse.".into())
    );
    assert_eq!(
        compose_system(None, Some("Be terse.")),
        Some("Be terse.".into())
    );
    assert_eq!(compose_system(Some("  "), None), None);
    assert_eq!(compose_system(None, None), None);
}

#[test]
fn text_attachments_are_inlined_with_their_filename() {
    let mut m = msg(Role::User, "review this", MessageStatus::Complete);
    m.attachments.push(Attachment {
        id: new_id(),
        conversation_id: None,
        message_id: None,
        project_id: None,
        filename: "app.py".into(),
        mime_type: "text/x-python".into(),
        size_bytes: 20,
        kind: AttachmentKind::Text,
        storage_path: "ab/abc".into(),
        sha256: "abc".into(),
        text_content: Some("print('hi')".into()),
        created_at: 0,
    });

    let out = build_messages(&[m], &store()).unwrap();
    let first = text_of(&out[0].content[0]);
    assert!(first.contains("name=\"app.py\""), "{first}");
    assert!(first.contains("print('hi')"));
    // The user's own instruction comes after the material it refers to.
    assert_eq!(text_of(&out[0].content[1]), "review this");
}

#[test]
fn a_missing_attachment_blob_fails_with_a_clear_message() {
    let mut m = msg(Role::User, "look", MessageStatus::Complete);
    m.attachments.push(Attachment {
        id: new_id(),
        conversation_id: None,
        message_id: None,
        project_id: None,
        filename: "photo.png".into(),
        mime_type: "image/png".into(),
        size_bytes: 10,
        kind: AttachmentKind::Image,
        storage_path: "zz/missing".into(),
        sha256: "zz".into(),
        text_content: None,
        created_at: 0,
    });
    let err = build_messages(&[m], &store()).unwrap_err().to_string();
    assert!(err.contains("photo.png"), "{err}");
}

#[test]
fn fallback_titles_are_short_and_skip_code_fences() {
    assert_eq!(
        fallback_title("Why does Mongo time out?"),
        "Why does Mongo time out?"
    );
    // Everything inside the fence is skipped, not just the fence line.
    assert_eq!(
        fallback_title("```python\nprint(1)\nmore_code()\n```\nExplain this script"),
        "Explain this script"
    );
    // A quote block and a heading are also not the title.
    assert_eq!(
        fallback_title("> quoted\n# Heading\nActual question here"),
        "Actual question here"
    );
    // A message that is nothing but code still gets a usable name.
    assert_eq!(fallback_title("```\nprint(1)\n```"), "New conversation");
    assert_eq!(fallback_title("   "), "New conversation");
    let long = fallback_title(&"word ".repeat(40));
    assert!(long.chars().count() <= 50, "got {long:?}");
    assert!(long.ends_with('…'));
}

#[test]
fn generated_titles_are_stripped_of_model_decoration() {
    assert_eq!(
        clean_generated_title("\"Debug Mongo Timeout\""),
        Some("Debug Mongo Timeout".into())
    );
    assert_eq!(
        clean_generated_title("Design auth API."),
        Some("Design auth API".into())
    );
    assert_eq!(
        clean_generated_title("  `Redis caching`  "),
        Some("Redis caching".into())
    );
    // Only the first line is a title.
    assert_eq!(
        clean_generated_title("Title here\nand rambling"),
        Some("Title here".into())
    );
    assert_eq!(clean_generated_title(""), None);
    assert_eq!(clean_generated_title(&"x".repeat(200)), None);
}

#[test]
fn branching_copies_history_up_to_the_chosen_message() {
    let db = db();
    let c = conversation(&db, "Original");
    say(&db, &c.id, Role::User, "one");
    let fork_point = say(&db, &c.id, Role::Assistant, "two");
    say(&db, &c.id, Role::User, "three");

    let branch = openclaude_lib::chat::branch_from(&db, &fork_point.id).unwrap();
    assert_ne!(branch.id, c.id);
    assert_eq!(
        branch.branched_from_message_id.as_deref(),
        Some(fork_point.id.as_str())
    );

    let conn = db.conn();
    let copied = repo::messages::list(&conn, &branch.id).unwrap();
    assert_eq!(copied.len(), 2, "history stops at the fork point");
    assert_eq!(copied[1].content, "two");
    // The original is untouched.
    assert_eq!(
        repo::messages::count_in_conversation(&conn, &c.id).unwrap(),
        3
    );
}

// --- rebuilding a session after an edit -----------------------------------

fn conv(provider_session: Option<&str>, metadata: serde_json::Value) -> Conversation {
    Conversation {
        id: "c1".into(),
        title: "t".into(),
        title_locked: false,
        provider: "claude-code".into(),
        provider_conversation_id: provider_session.map(str::to_owned),
        model: "sonnet".into(),
        system_prompt: None,
        project_id: None,
        pinned: false,
        archived: false,
        deleted_at: None,
        branched_from_message_id: None,
        created_at: 0,
        updated_at: 0,
        last_message_at: None,
        metadata,
    }
}

#[test]
fn a_live_session_is_resumed_not_rebuilt() {
    // History lives in the CLI session; replaying it would duplicate it.
    assert!(!rebuilding_context(&conv(Some("sess-1"), json!({})), 6));
}

#[test]
fn a_first_turn_has_no_history_to_rebuild() {
    assert!(!rebuilding_context(&conv(None, json!({})), 1));
}

#[test]
fn a_branch_carries_its_context_by_forking() {
    // Its copied history is already in the forked session.
    assert!(!rebuilding_context(
        &conv(None, json!({"forkFrom": "sess-1"})),
        6
    ));
}

#[test]
fn a_truncated_conversation_with_no_session_is_rebuilt() {
    assert!(rebuilding_context(&conv(None, json!({})), 6));
    // An empty forkFrom is not a fork.
    assert!(rebuilding_context(&conv(None, json!({"forkFrom": ""})), 6));
}

#[test]
fn prior_turns_render_with_both_speakers_and_an_edit_notice() {
    let prior = vec![
        msg(Role::User, "what is a river", MessageStatus::Complete),
        msg(Role::Assistant, "flowing water", MessageStatus::Complete),
    ];
    let out = render_prior_turns(&prior);
    assert!(out.contains("### User"));
    assert!(out.contains("what is a river"));
    assert!(out.contains("### Claude"));
    assert!(out.contains("flowing water"));
    // The model must not answer the transcript itself.
    assert!(out.contains("answer only the message that follows"));
}

#[test]
fn an_empty_turn_is_left_out_of_the_replay() {
    let prior = vec![msg(Role::Assistant, "   ", MessageStatus::Complete)];
    assert!(!render_prior_turns(&prior).contains("### Claude"));
}
