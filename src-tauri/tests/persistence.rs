//! Conversation persistence, ordering, and recovery after an abrupt exit.

mod common;

use common::*;
use openclaude_lib::db::models::*;
use openclaude_lib::db::repo;
use openclaude_lib::db::Db;

#[test]
fn conversations_and_messages_survive_reopening_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart.db");

    let (cid, ids) = {
        let db = Db::open(&path).unwrap();
        let c = conversation(&db, "Debug Mongo timeout");
        let a = say(&db, &c.id, Role::User, "Why does the driver time out?");
        let b = say(
            &db,
            &c.id,
            Role::Assistant,
            "Because the pool is exhausted.",
        );
        (c.id, vec![a.id, b.id])
    };

    // Simulate a full application restart: new process, same file.
    let db = Db::open(&path).unwrap();
    let conn = db.conn();
    let loaded = repo::conversations::get(&conn, &cid).unwrap();
    assert_eq!(loaded.title, "Debug Mongo timeout");

    let msgs = repo::messages::list(&conn, &cid).unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs.iter().map(|m| m.id.clone()).collect::<Vec<_>>(), ids);
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[1].content, "Because the pool is exhausted.");
}

#[test]
fn messages_keep_insertion_order_even_after_a_deletion() {
    let db = db();
    let c = conversation(&db, "Ordering");
    let m1 = say(&db, &c.id, Role::User, "one");
    let m2 = say(&db, &c.id, Role::Assistant, "two");
    let m3 = say(&db, &c.id, Role::User, "three");
    assert_eq!((m1.seq, m2.seq, m3.seq), (0, 1, 2));

    // Removing the middle message must not cause the next insert to collide
    // with an existing seq.
    repo::messages::delete(&db.conn(), &m2.id).unwrap();
    let m4 = say(&db, &c.id, Role::Assistant, "four");
    assert_eq!(m4.seq, 3);

    let order: Vec<&str> = {
        let conn = db.conn();
        repo::messages::list(&conn, &c.id)
            .unwrap()
            .iter()
            .map(|m| Box::leak(m.content.clone().into_boxed_str()) as &str)
            .collect()
    };
    assert_eq!(order, vec!["one", "three", "four"]);
}

#[test]
fn a_generation_killed_with_the_process_is_recovered_as_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.db");

    let (cid, partial_id, empty_id) = {
        let db = Db::open(&path).unwrap();
        let c = conversation(&db, "Crashed mid-stream");
        say(&db, &c.id, Role::User, "Explain WAL mode");

        let conn = db.conn();
        // One reply that had started producing text...
        let partial = repo::messages::insert(
            &conn,
            repo::messages::NewMessage {
                conversation_id: &c.id,
                role: Role::Assistant,
                content: "Write-ahead logging means",
                status: MessageStatus::Streaming,
                model: Some("claude-sonnet-4-5"),
            },
        )
        .unwrap();
        // ...and one that never got a single token.
        let empty = repo::messages::insert(
            &conn,
            repo::messages::NewMessage {
                conversation_id: &c.id,
                role: Role::Assistant,
                content: "",
                status: MessageStatus::Pending,
                model: Some("claude-sonnet-4-5"),
            },
        )
        .unwrap();
        (c.id, partial.id, empty.id)
    };

    // Reopen and run the startup recovery pass.
    let db = Db::open(&path).unwrap();
    let conn = db.conn();
    let recovered = repo::messages::recover_in_flight(&conn).unwrap();
    assert_eq!(recovered, 2);

    let partial = repo::messages::get(&conn, &partial_id).unwrap();
    assert_eq!(partial.status, MessageStatus::Interrupted);
    // The text that did arrive before the crash is not lost.
    assert_eq!(partial.content, "Write-ahead logging means");

    let empty = repo::messages::get(&conn, &empty_id).unwrap();
    assert_eq!(empty.status, MessageStatus::Error);
    assert!(empty.error_message.is_some());

    // Nothing is left looking live.
    assert!(repo::messages::list(&conn, &cid)
        .unwrap()
        .iter()
        .all(|m| !m.status.is_in_flight()));
}

#[test]
fn recovery_is_idempotent_and_leaves_finished_messages_alone() {
    let db = db();
    let c = conversation(&db, "Stable");
    say(&db, &c.id, Role::User, "hi");
    let done = say(&db, &c.id, Role::Assistant, "hello");

    let conn = db.conn();
    assert_eq!(repo::messages::recover_in_flight(&conn).unwrap(), 0);
    assert_eq!(
        repo::messages::get(&conn, &done.id).unwrap().status,
        MessageStatus::Complete
    );
}

#[test]
fn deleting_a_conversation_cascades_to_its_messages_and_attachments() {
    let db = db();
    let c = conversation(&db, "Doomed");
    let m = say(&db, &c.id, Role::User, "attach this");
    {
        let conn = db.conn();
        repo::attachments::insert(
            &conn,
            repo::attachments::NewAttachment {
                conversation_id: Some(&c.id),
                project_id: None,
                filename: "notes.md",
                mime_type: "text/markdown",
                size_bytes: 12,
                kind: AttachmentKind::Text,
                storage_path: "ab/abc",
                sha256: "abc",
                text_content: Some("hello"),
            },
        )
        .unwrap();
        repo::attachments::attach_to_message(&conn, &[], &m.id).unwrap();
    }

    let conn = db.conn();
    repo::conversations::purge(&conn, &c.id).unwrap();
    assert_eq!(
        repo::messages::count_in_conversation(&conn, &c.id).unwrap(),
        0
    );
    assert!(repo::attachments::for_conversation(&conn, &c.id)
        .unwrap()
        .is_empty());
}

#[test]
fn trash_hides_a_conversation_without_destroying_it() {
    let db = db();
    let c = conversation(&db, "Regrettable");
    say(&db, &c.id, Role::User, "oops");
    let conn = db.conn();

    repo::conversations::trash(&conn, &c.id).unwrap();
    let active =
        repo::conversations::list(&conn, repo::conversations::ListScope::Active, None, 50, 0)
            .unwrap();
    assert!(active.iter().all(|s| s.conversation.id != c.id));

    let trashed =
        repo::conversations::list(&conn, repo::conversations::ListScope::Trash, None, 50, 0)
            .unwrap();
    assert_eq!(trashed.len(), 1);

    repo::conversations::restore(&conn, &c.id).unwrap();
    // The messages came back with it.
    assert_eq!(
        repo::messages::count_in_conversation(&conn, &c.id).unwrap(),
        1
    );
}

#[test]
fn the_sidebar_query_returns_counts_and_a_preview_in_one_pass() {
    let db = db();
    let c = conversation(&db, "With preview");
    say(&db, &c.id, Role::User, "first");
    say(&db, &c.id, Role::Assistant, "the latest reply");

    let conn = db.conn();
    let list =
        repo::conversations::list(&conn, repo::conversations::ListScope::Active, None, 50, 0)
            .unwrap();
    let row = list.iter().find(|s| s.conversation.id == c.id).unwrap();
    assert_eq!(row.message_count, 2);
    assert_eq!(row.preview.as_deref(), Some("the latest reply"));
}

#[test]
fn pinned_conversations_sort_above_more_recent_ones() {
    let db = db();
    let old = conversation(&db, "Old but pinned");
    let new = conversation(&db, "Fresh");
    {
        let conn = db.conn();
        repo::conversations::touch(&conn, &old.id, 1_000).unwrap();
        repo::conversations::touch(&conn, &new.id, 9_000).unwrap();
        repo::conversations::set_pinned(&conn, &old.id, true).unwrap();
    }
    let conn = db.conn();
    let list =
        repo::conversations::list(&conn, repo::conversations::ListScope::Active, None, 50, 0)
            .unwrap();
    assert_eq!(list[0].conversation.id, old.id);
    assert_eq!(list[1].conversation.id, new.id);
}

#[test]
fn usage_totals_aggregate_across_the_conversation() {
    let db = db();
    let c = conversation(&db, "Usage");
    let m = say(&db, &c.id, Role::Assistant, "answer");
    {
        let conn = db.conn();
        repo::messages::finalize(
            &conn,
            &m.id,
            repo::messages::Completion {
                content: "answer",
                input_tokens: Some(100),
                output_tokens: Some(40),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let conn = db.conn();
    let u = repo::conversations::usage(&conn, &c.id).unwrap();
    assert_eq!((u.input_tokens, u.output_tokens), (100, 40));
}
