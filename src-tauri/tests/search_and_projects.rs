//! Full-text search and project management.

mod common;

use common::*;
use openclaude_lib::db::models::*;
use openclaude_lib::db::repo;

#[test]
fn search_finds_matches_in_titles_and_message_bodies() {
    let db = db();
    let c = conversation(&db, "Debug Mongo timeout");
    say(
        &db,
        &c.id,
        Role::User,
        "The connection pool keeps exhausting itself",
    );
    say(
        &db,
        &c.id,
        Role::Assistant,
        "Raise maxPoolSize and check for leaked cursors",
    );

    let other = conversation(&db, "Interview preparation");
    say(&db, &other.id, Role::User, "Explain consistent hashing");

    let conn = db.conn();

    let hits = repo::search::search(&conn, "mongo", 20).unwrap();
    assert!(hits
        .iter()
        .any(|h| h.kind == "title" && h.conversation_id == c.id));

    let hits = repo::search::search(&conn, "cursors", 20).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, "message");
    assert!(
        hits[0].snippet.contains("<<cursors>>"),
        "{}",
        hits[0].snippet
    );

    // Scoped to the other conversation only.
    let hits = repo::search::search(&conn, "hashing", 20).unwrap();
    assert_eq!(hits[0].conversation_id, other.id);
}

#[test]
fn search_is_prefix_matched_so_it_feels_live_while_typing() {
    let db = db();
    let c = conversation(&db, "Redis caching strategy");
    say(&db, &c.id, Role::User, "Discuss invalidation");

    let conn = db.conn();
    assert!(!repo::search::search(&conn, "cach", 20).unwrap().is_empty());
    assert!(!repo::search::search(&conn, "invalid", 20)
        .unwrap()
        .is_empty());
}

#[test]
fn fts_metacharacters_in_a_query_do_not_raise_an_error() {
    let db = db();
    let c = conversation(&db, "Odd query");
    say(&db, &c.id, Role::User, "a NOT b and c");

    let conn = db.conn();
    // Each of these would be a syntax error if passed to MATCH unescaped.
    for q in ["\"", "NOT", "a AND", "*", "-x", "()", "a:b", "^", "a OR"] {
        let r = repo::search::search(&conn, q, 10);
        assert!(r.is_ok(), "query {q:?} should not error: {:?}", r.err());
    }
}

#[test]
fn trashed_conversations_are_excluded_from_search() {
    let db = db();
    let c = conversation(&db, "Secret plans");
    say(&db, &c.id, Role::User, "confidential widget");

    {
        let conn = db.conn();
        assert!(!repo::search::search(&conn, "widget", 10)
            .unwrap()
            .is_empty());
        repo::conversations::trash(&conn, &c.id).unwrap();
    }
    let conn = db.conn();
    assert!(repo::search::search(&conn, "widget", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn the_index_follows_edits_and_deletions() {
    let db = db();
    let c = conversation(&db, "Editable");
    let m = say(&db, &c.id, Role::User, "original wording");
    let conn = db.conn();

    assert!(!repo::search::search(&conn, "original", 10)
        .unwrap()
        .is_empty());

    repo::messages::set_content(&conn, &m.id, "replacement wording").unwrap();
    assert!(repo::search::search(&conn, "original", 10)
        .unwrap()
        .is_empty());
    assert!(!repo::search::search(&conn, "replacement", 10)
        .unwrap()
        .is_empty());

    repo::messages::delete(&conn, &m.id).unwrap();
    assert!(repo::search::search(&conn, "replacement", 10)
        .unwrap()
        .is_empty());

    // Renaming re-indexes the title too.
    repo::conversations::rename(&conn, &c.id, "Renamed thread").unwrap();
    assert!(repo::search::search(&conn, "Editable", 10)
        .unwrap()
        .is_empty());
    assert!(!repo::search::search(&conn, "Renamed", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn attachment_filenames_are_searchable() {
    let db = db();
    let c = conversation(&db, "With files");
    let conn = db.conn();
    repo::attachments::insert(
        &conn,
        repo::attachments::NewAttachment {
            conversation_id: Some(&c.id),
            project_id: None,
            filename: "payments_service.py",
            mime_type: "text/x-python",
            size_bytes: 100,
            kind: AttachmentKind::Text,
            storage_path: "aa/aaa",
            sha256: "aaa",
            text_content: Some("x = 1"),
        },
    )
    .unwrap();

    let hits = repo::search::search(&conn, "payments", 10).unwrap();
    assert!(hits.iter().any(|h| h.kind == "attachment"));
}

#[test]
fn rebuilding_the_index_restores_search_after_corruption() {
    let db = db();
    let c = conversation(&db, "Rebuildable");
    say(&db, &c.id, Role::User, "findable content");

    let conn = db.conn();
    // Simulate an FTS table that drifted out of sync with its content table.
    conn.execute("DELETE FROM messages_fts", []).unwrap();
    assert!(repo::search::search(&conn, "findable", 10)
        .unwrap()
        .is_empty());

    repo::search::rebuild_indexes(&conn).unwrap();
    assert!(!repo::search::search(&conn, "findable", 10)
        .unwrap()
        .is_empty());
}

#[test]
fn search_scales_to_a_large_history() {
    let db = db();
    // 400 conversations x 6 messages ~ 2,400 indexed rows; enough to catch a
    // query plan that degrades into a scan.
    {
        let conn = db.conn();
        for i in 0..400 {
            let c = repo::conversations::create(
                &conn,
                repo::conversations::NewConversation {
                    title: Some(format!("Conversation number {i}")),
                    model: "claude-sonnet-4-5".into(),
                    project_id: None,
                    system_prompt: None,
                },
            )
            .unwrap();
            for j in 0..6 {
                repo::messages::insert(
                    &conn,
                    repo::messages::NewMessage {
                        conversation_id: &c.id,
                        role: if j % 2 == 0 {
                            Role::User
                        } else {
                            Role::Assistant
                        },
                        content: &format!("filler text body {i}-{j} about databases and indexing"),
                        status: MessageStatus::Complete,
                        model: None,
                    },
                )
                .unwrap();
            }
        }
        repo::messages::insert(
            &conn,
            repo::messages::NewMessage {
                conversation_id: &repo::conversations::create(
                    &conn,
                    repo::conversations::NewConversation {
                        title: Some("Needle".into()),
                        model: "m".into(),
                        project_id: None,
                        system_prompt: None,
                    },
                )
                .unwrap()
                .id,
                role: Role::User,
                content: "a very distinctive phrase: ornithopter",
                status: MessageStatus::Complete,
                model: None,
            },
        )
        .unwrap();
    }

    let conn = db.conn();
    let started = std::time::Instant::now();
    let hits = repo::search::search(&conn, "ornithopter", 20).unwrap();
    let elapsed = started.elapsed();

    assert_eq!(hits.len(), 1);
    assert!(
        elapsed.as_millis() < 250,
        "search took {elapsed:?}; expected to feel instant"
    );

    // The sidebar query must stay fast at this size too.
    let started = std::time::Instant::now();
    let list =
        repo::conversations::list(&conn, repo::conversations::ListScope::Active, None, 200, 0)
            .unwrap();
    assert_eq!(list.len(), 200);
    assert!(
        started.elapsed().as_millis() < 400,
        "sidebar query too slow"
    );
}

#[test]
fn a_project_groups_conversations_and_survives_its_own_deletion() {
    let db = db();
    let conn = db.conn();

    let p = repo::projects::create(
        &conn,
        repo::projects::ProjectInput {
            name: "Backend API".into(),
            description: "FastAPI + Mongo".into(),
            instructions: "Prefer async APIs. Use Python 3.12.".into(),
            working_dir: None,
            default_model: None,
            color: None,
        },
    )
    .unwrap();

    let c = repo::conversations::create(
        &conn,
        repo::conversations::NewConversation {
            title: Some("Mongo performance".into()),
            model: "claude-sonnet-4-5".into(),
            project_id: Some(p.id.clone()),
            system_prompt: None,
        },
    )
    .unwrap();

    let summary = repo::projects::list(&conn, false).unwrap();
    assert_eq!(summary[0].conversation_count, 1);

    let scoped = repo::conversations::list(
        &conn,
        repo::conversations::ListScope::Active,
        Some(&p.id),
        50,
        0,
    )
    .unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].project_name.as_deref(), Some("Backend API"));

    // Deleting the project must not take the conversation with it.
    repo::projects::delete(&conn, &p.id).unwrap();
    let still_there = repo::conversations::get(&conn, &c.id).unwrap();
    assert!(still_there.project_id.is_none());
}

#[test]
fn settings_round_trip_and_tolerate_corruption() {
    let db = db();
    let conn = db.conn();

    repo::settings::set(&conn, "appearance.theme", &"dark").unwrap();
    let theme: String = repo::settings::get_or(&conn, "appearance.theme", "system".into());
    assert_eq!(theme, "dark");

    // An absent key falls back.
    let missing: i64 = repo::settings::get_or(&conn, "nope", 42);
    assert_eq!(missing, 42);

    // A malformed value must not panic or poison the launch.
    repo::settings::set_raw(&conn, "claude.maxTokens", "{not json").unwrap();
    let n: i64 = repo::settings::get_or(&conn, "claude.maxTokens", 8192);
    assert_eq!(n, 8192);
}
