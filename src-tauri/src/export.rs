//! Conversation export.
//!
//! Exports contain only the conversation itself and metadata the user can
//! already see in the UI.

use crate::db::models::*;
use crate::db::repo;
use crate::db::Db;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Json,
    Text,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Json => "json",
            Self::Text => "txt",
        }
    }
}

fn iso(ms: Millis) -> String {
    // Minimal UTC ISO-8601 without pulling in a date library for one use.
    let secs = ms / 1000;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (h, m, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);

    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };

    format!("{y:04}-{mth:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn role_heading(role: Role) -> &'static str {
    match role {
        Role::User => "User",
        Role::Assistant => "Claude",
        Role::System => "System",
    }
}

pub fn export(
    db: &Db,
    conversation_id: &str,
    format: ExportFormat,
    include_metadata: bool,
) -> Result<String> {
    let conn = db.conn();
    let c = repo::conversations::get(&conn, conversation_id)?;
    let messages = repo::messages::list(&conn, conversation_id)?;
    let project = c
        .project_id
        .as_deref()
        .and_then(|p| repo::projects::get(&conn, p).ok());

    Ok(match format {
        ExportFormat::Json => {
            let value = serde_json::json!({
                "title": c.title,
                "model": c.model,
                "createdAt": iso(c.created_at),
                "updatedAt": iso(c.updated_at),
                "project": project.as_ref().map(|p| &p.name),
                "systemPrompt": c.system_prompt,
                "messages": messages.iter().map(|m| serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                    "status": m.status,
                    "model": m.model,
                    "createdAt": iso(m.created_at),
                    "inputTokens": m.input_tokens,
                    "outputTokens": m.output_tokens,
                    "attachments": m.attachments.iter()
                        .map(|a| serde_json::json!({ "filename": a.filename, "mimeType": a.mime_type }))
                        .collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "exportedBy": concat!("OpenClaude Desktop ", env!("CARGO_PKG_VERSION")),
            });
            serde_json::to_string_pretty(&value)?
        }

        ExportFormat::Markdown => {
            let mut out = format!("# {}\n\n", c.title);
            if include_metadata {
                out.push_str(&format!("- **Model:** {}\n", c.model));
                out.push_str(&format!("- **Created:** {}\n", iso(c.created_at)));
                if let Some(p) = &project {
                    out.push_str(&format!("- **Project:** {}\n", p.name));
                }
                out.push_str(&format!("- **Messages:** {}\n", messages.len()));
                out.push_str("\n---\n\n");
            }
            for m in &messages {
                out.push_str(&format!("## {}\n\n", role_heading(m.role)));
                if !m.attachments.is_empty() {
                    for a in &m.attachments {
                        out.push_str(&format!("> 📎 `{}` ({})\n", a.filename, a.mime_type));
                    }
                    out.push('\n');
                }
                out.push_str(m.content.trim());
                out.push_str("\n\n");
                match m.status {
                    MessageStatus::Interrupted => {
                        out.push_str("> _This reply was interrupted._\n\n")
                    }
                    MessageStatus::Error => out.push_str(&format!(
                        "> _Failed: {}_\n\n",
                        m.error_message.as_deref().unwrap_or("unknown error")
                    )),
                    _ => {}
                }
            }
            out
        }

        ExportFormat::Text => {
            let mut out = format!("{}\n{}\n\n", c.title, "=".repeat(c.title.chars().count()));
            if include_metadata {
                out.push_str(&format!(
                    "Model:   {}\nCreated: {}\n",
                    c.model,
                    iso(c.created_at)
                ));
                if let Some(p) = &project {
                    out.push_str(&format!("Project: {}\n", p.name));
                }
                out.push('\n');
            }
            for m in &messages {
                out.push_str(&format!("--- {} ---\n\n", role_heading(m.role)));
                for a in &m.attachments {
                    out.push_str(&format!("[attachment: {}]\n", a.filename));
                }
                out.push_str(m.content.trim());
                out.push_str("\n\n");
            }
            out
        }
    })
}

/// A safe filename for the export, derived from the title.
pub fn suggested_filename(title: &str, format: ExportFormat) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase();
    let slug: String = slug.chars().take(60).collect();
    let stem = if slug.is_empty() {
        "conversation".to_string()
    } else {
        slug
    };
    format!("{stem}.{}", format.extension())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_matches_reference_values_including_leap_days() {
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso(1_789_862_400_000), "2026-09-20T00:00:00Z");
        // Century leap year, and an ordinary one.
        assert_eq!(iso(951_782_400_000), "2000-02-29T00:00:00Z");
        assert_eq!(iso(1_709_164_800_000), "2024-02-29T00:00:00Z");
        assert_eq!(iso(4_102_444_800_000), "2100-01-01T00:00:00Z");
        // Time-of-day component.
        assert_eq!(iso(1_789_862_400_000 + 45_296_000), "2026-09-20T12:34:56Z");
    }

    #[test]
    fn filenames_are_slugified_and_bounded() {
        assert_eq!(
            suggested_filename("Debug Mongo Timeout", ExportFormat::Markdown),
            "debug-mongo-timeout.md"
        );
        assert_eq!(
            suggested_filename("  ///  ", ExportFormat::Json),
            "conversation.json"
        );
        assert!(suggested_filename(&"x".repeat(500), ExportFormat::Text).len() <= 64);
    }
}
