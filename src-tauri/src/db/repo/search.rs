use crate::db::models::*;
use crate::error::Result;
use rusqlite::{params, Connection, Row};

/// Turn arbitrary user input into a safe FTS5 MATCH expression.
///
/// FTS5 treats `"`, `*`, `:`, `^`, `-`, `(`, `)`, `AND`/`OR`/`NOT` as syntax.
/// Feeding raw input straight to MATCH means a stray quote raises
/// `fts5: syntax error`, and a bare `-` silently flips a term to a negation.
/// So every token is emitted as a double-quoted string (with `"` doubled),
/// which FTS5 parses as a literal. The final token additionally gets a `*`
/// so search feels live while the user is still typing a word.
pub fn build_match_query(input: &str) -> Option<String> {
    let tokens: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| t.replace('"', "\"\""))
        .collect();

    if tokens.is_empty() {
        return None;
    }

    let last = tokens.len() - 1;
    let expr = tokens
        .iter()
        .enumerate()
        .map(|(i, t)| {
            // Prefix-match the trailing token, but only when it is long enough
            // that the prefix scan stays cheap.
            if i == last && t.chars().count() >= 2 {
                format!("\"{t}\"*")
            } else {
                format!("\"{t}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    Some(expr)
}

fn hit(row: &Row<'_>) -> rusqlite::Result<SearchHit> {
    Ok(SearchHit {
        conversation_id: row.get(0)?,
        conversation_title: row.get(1)?,
        message_id: row.get(2)?,
        kind: row.get(3)?,
        role: row.get::<_, Option<String>>(4)?.map(|r| Role::parse(&r)),
        snippet: row.get(5)?,
        created_at: row.get(6)?,
        rank: row.get(7)?,
    })
}

/// Global search across titles, message bodies and attachment filenames.
///
/// Trashed conversations are excluded; archived ones are included but rank
/// slightly lower so active work surfaces first.
pub fn search(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
    let Some(m) = build_match_query(query) else {
        return Ok(Vec::new());
    };

    let sql = "
        WITH title_hits AS (
            SELECT c.id AS conversation_id, c.title AS conversation_title,
                   NULL AS message_id, 'title' AS kind, NULL AS role,
                   snippet(conversations_fts, 0, '<<', '>>', '…', 12) AS snippet,
                   COALESCE(c.last_message_at, c.updated_at) AS created_at,
                   -- Title matches are the strongest signal; bias them up.
                   bm25(conversations_fts) - 4.0 AS rank
              FROM conversations_fts
              JOIN conversations c ON c.rowid = conversations_fts.rowid
             WHERE conversations_fts MATCH ?1 AND c.deleted_at IS NULL
        ),
        message_hits AS (
            SELECT c.id, c.title, m.id, 'message', m.role,
                   snippet(messages_fts, 0, '<<', '>>', '…', 18),
                   m.created_at,
                   bm25(messages_fts) + (CASE WHEN c.archived = 1 THEN 2.0 ELSE 0.0 END)
              FROM messages_fts
              JOIN messages m ON m.rowid = messages_fts.rowid
              JOIN conversations c ON c.id = m.conversation_id
             WHERE messages_fts MATCH ?1 AND c.deleted_at IS NULL
        ),
        file_hits AS (
            SELECT c.id, c.title, a.message_id, 'attachment', NULL,
                   a.filename, a.created_at,
                   bm25(attachments_fts) - 1.0
              FROM attachments_fts
              JOIN attachments a ON a.rowid = attachments_fts.rowid
              JOIN conversations c ON c.id = a.conversation_id
             WHERE attachments_fts MATCH ?1 AND c.deleted_at IS NULL
        )
        SELECT * FROM (
            SELECT * FROM title_hits
            UNION ALL SELECT * FROM message_hits
            UNION ALL SELECT * FROM file_hits
        )
        ORDER BY rank, created_at DESC
        LIMIT ?2";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params![m, limit], hit)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Matches within a single conversation, for in-conversation find.
pub fn search_in_conversation(
    conn: &Connection,
    conversation_id: &str,
    query: &str,
    limit: i64,
) -> Result<Vec<SearchHit>> {
    let Some(m) = build_match_query(query) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title, m.id, 'message', m.role,
                snippet(messages_fts, 0, '<<', '>>', '…', 18),
                m.created_at, bm25(messages_fts)
           FROM messages_fts
           JOIN messages m ON m.rowid = messages_fts.rowid
           JOIN conversations c ON c.id = m.conversation_id
          WHERE messages_fts MATCH ?1 AND m.conversation_id = ?2
          ORDER BY m.seq
          LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![m, conversation_id, limit], hit)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Rebuild the FTS indexes from their content tables.
/// Offered in Settings → Advanced if search ever looks stale.
pub fn rebuild_indexes(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "INSERT INTO messages_fts(messages_fts) VALUES('rebuild');
         INSERT INTO conversations_fts(conversations_fts) VALUES('rebuild');
         INSERT INTO attachments_fts(attachments_fts) VALUES('rebuild');",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_punctuation_only_queries_yield_nothing() {
        assert!(build_match_query("").is_none());
        assert!(build_match_query("   ").is_none());
        assert!(build_match_query("*** --- \"\"").is_none());
    }

    #[test]
    fn fts_operators_in_user_input_are_neutralised() {
        // A user typing `NOT` or `-foo` means the literal words, not operators.
        let q = build_match_query("NOT foo").unwrap();
        assert_eq!(q, "\"NOT\" AND \"foo\"*");
        let q = build_match_query("-mongo").unwrap();
        assert_eq!(q, "\"mongo\"*");
    }

    #[test]
    fn embedded_quotes_are_escaped_not_dropped() {
        let q = build_match_query("say \"hi\" now").unwrap();
        // Quotes are stripped as separators, leaving safe literal tokens.
        assert_eq!(q, "\"say\" AND \"hi\" AND \"now\"*");
    }

    #[test]
    fn last_token_is_prefix_matched_for_type_ahead() {
        assert_eq!(
            build_match_query("mongo tim").unwrap(),
            "\"mongo\" AND \"tim\"*"
        );
        // A single leftover character would scan too much of the index.
        assert_eq!(build_match_query("mongo t").unwrap(), "\"mongo\" AND \"t\"");
    }
}
