//! Attachment validation and the content-addressed blob store.
//!
//! Files are hashed and stored at `attachments/<aa>/<sha256>`, so attaching
//! the same file to ten conversations costs one copy on disk. Rows reference
//! the blob; the blob is only removed once no row points at it.

use crate::db::models::AttachmentKind;
use crate::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Anthropic's documented per-request limits. Checked locally so an oversized
/// file fails instantly with a clear message instead of after a slow upload
/// and an opaque 413.
pub const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_PDF_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;
/// Total base64 payload across one request.
pub const MAX_REQUEST_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_IMAGES_PER_REQUEST: usize = 20;

const IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Extensions we treat as text even when the system reports something odd or
/// nothing at all. Source files are the common case for this app.
#[rustfmt::skip]
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "rst", "adoc", "log", "csv", "tsv", "json", "jsonl", "ndjson",
    "yaml", "yml", "toml", "ini", "cfg", "conf", "env", "properties", "xml", "html", "htm",
    "css", "scss", "sass", "less", "js", "jsx", "mjs", "cjs", "ts", "tsx", "py", "pyi", "rb",
    "go", "rs", "java", "kt", "kts", "scala", "c", "h", "cc", "cpp", "hpp", "cxx", "cs", "swift",
    "php", "pl", "pm", "lua", "r", "jl", "sh", "bash", "zsh", "fish", "ps1", "sql", "graphql",
    "gql", "proto", "tf", "tfvars", "hcl", "dockerfile", "makefile", "mk", "cmake", "gradle",
    "vue", "svelte", "astro", "elm", "ex", "exs", "erl", "hs", "ml", "clj", "cljs", "zig", "nim",
    "dart", "diff", "patch", "gitignore", "editorconfig", "lock", "sum", "mod",
];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedFile {
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub kind: AttachmentKind,
    /// Present for text files; lets us send the content inline as text rather
    /// than as an opaque base64 document.
    pub text_content: Option<String>,
    pub sha256: String,
    pub storage_path: String,
}

fn human(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= MB {
        format!("{:.1} MB", bytes as f64 / MB)
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Classify by MIME first, then by extension, then by sniffing for NUL bytes.
/// `mime_guess` reports `application/octet-stream` for plenty of source files,
/// so extension and content both get a say before we reject something.
pub fn classify(path: &Path, head: &[u8]) -> (String, Option<AttachmentKind>) {
    let guessed = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("")
        .to_string();
    let ext = extension(path);
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if IMAGE_TYPES.contains(&guessed.as_str()) {
        return (guessed, Some(AttachmentKind::Image));
    }
    if guessed == "application/pdf" || ext == "pdf" {
        return ("application/pdf".into(), Some(AttachmentKind::Document));
    }
    if TEXT_EXTENSIONS.contains(&ext.as_str())
        || TEXT_EXTENSIONS.contains(&stem.as_str())
        || guessed.starts_with("text/")
    {
        let mime = if guessed.starts_with("text/") {
            guessed
        } else {
            "text/plain".into()
        };
        return (mime, Some(AttachmentKind::Text));
    }
    // Extensionless files (LICENSE, Makefile, a script) are extremely common
    // in a code folder. Treat as text when the first block is valid UTF-8
    // with no NUL bytes.
    if !head.is_empty() && !head.contains(&0) && std::str::from_utf8(head).is_ok() {
        return ("text/plain".into(), Some(AttachmentKind::Text));
    }

    let mime = if guessed.is_empty() {
        "application/octet-stream".into()
    } else {
        guessed
    };
    (mime, None)
}

/// Validate a file on disk and copy it into the blob store.
pub fn ingest(path: &Path, store_root: &Path) -> Result<ValidatedFile> {
    let meta = std::fs::metadata(path)
        .map_err(|_| AppError::invalid(format!("Cannot read {}.", display_name(path))))?;

    if meta.is_dir() {
        return Err(AppError::invalid(format!(
            "{} is a folder. Attach individual files, or set a working folder on the project.",
            display_name(path)
        )));
    }
    if meta.len() == 0 {
        return Err(AppError::invalid(format!(
            "{} is empty.",
            display_name(path)
        )));
    }

    let bytes = std::fs::read(path)
        .map_err(|_| AppError::invalid(format!("Cannot read {}.", display_name(path))))?;

    let head = &bytes[..bytes.len().min(8192)];
    let (mime_type, kind) = classify(path, head);

    let Some(kind) = kind else {
        return Err(AppError::invalid(format!(
            "{} ({mime_type}) is not a file type Claude can read. \
             Supported: images, PDFs, and text or source files.",
            display_name(path)
        )));
    };

    let limit = match kind {
        AttachmentKind::Image => MAX_IMAGE_BYTES,
        AttachmentKind::Document => MAX_PDF_BYTES,
        AttachmentKind::Text => MAX_TEXT_BYTES,
    };
    if meta.len() > limit {
        return Err(AppError::invalid(format!(
            "{} is {} — the limit for this file type is {}.",
            display_name(path),
            human(meta.len()),
            human(limit)
        )));
    }

    let text_content = if kind == AttachmentKind::Text {
        match String::from_utf8(bytes.clone()) {
            Ok(s) => Some(s),
            Err(_) => {
                return Err(AppError::invalid(format!(
                    "{} is not valid UTF-8 text.",
                    display_name(path)
                )))
            }
        }
    } else {
        None
    };

    let sha256 = hex(&Sha256::digest(&bytes));
    let rel = blob_rel_path(&sha256);
    let dest = store_root.join(&rel);
    if !dest.exists() {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Write to a temp file then rename, so a crash mid-copy cannot leave a
        // truncated blob sitting at a hash that claims to be complete.
        let tmp = dest.with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o600));
        }
    }

    Ok(ValidatedFile {
        filename: display_name(path),
        mime_type,
        size_bytes: meta.len() as i64,
        kind,
        text_content,
        sha256,
        storage_path: rel.to_string_lossy().into_owned(),
    })
}

/// Store raw bytes that never had a path — a pasted or dropped image.
pub fn ingest_bytes(
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
    store_root: &Path,
) -> Result<ValidatedFile> {
    if bytes.is_empty() {
        return Err(AppError::invalid("The pasted image was empty."));
    }
    if !IMAGE_TYPES.contains(&mime_type) {
        return Err(AppError::invalid(format!(
            "{mime_type} images are not supported. Use PNG, JPEG, GIF or WebP."
        )));
    }
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(AppError::invalid(format!(
            "That image is {} — the limit is {}.",
            human(bytes.len() as u64),
            human(MAX_IMAGE_BYTES)
        )));
    }

    let sha256 = hex(&Sha256::digest(bytes));
    let rel = blob_rel_path(&sha256);
    let dest = store_root.join(&rel);
    if !dest.exists() {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = dest.with_extension("part");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &dest)?;
    }

    Ok(ValidatedFile {
        filename: sanitize_filename(filename),
        mime_type: mime_type.to_string(),
        size_bytes: bytes.len() as i64,
        kind: AttachmentKind::Image,
        text_content: None,
        sha256,
        storage_path: rel.to_string_lossy().into_owned(),
    })
}

/// Two-level fan-out keeps any one directory small even with many thousands
/// of blobs.
fn blob_rel_path(sha256: &str) -> PathBuf {
    PathBuf::from(&sha256[..2]).join(sha256)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn display_name(path: &Path) -> String {
    sanitize_filename(path.file_name().and_then(|n| n.to_str()).unwrap_or("file"))
}

/// Reduce an arbitrary name to a bare, display-safe filename.
///
/// Filenames reach the UI, exported Markdown, and (via the blob store) the
/// filesystem. Keeping only the final path segment means a crafted name like
/// `../../etc/passwd` can never be interpreted as a traversal, and stripping
/// control characters stops a name from garbling the interface.
pub fn sanitize_filename(name: &str) -> String {
    let last = name
        .rsplit(['/', '\\'])
        .find(|s| !s.trim().is_empty() && *s != "." && *s != "..")
        .unwrap_or("");

    let cleaned: String = last.chars().filter(|c| !c.is_control()).collect();
    // Leading dots would make the file hidden and can hide the real extension.
    let cleaned = cleaned.trim().trim_start_matches('.').trim().to_string();

    if cleaned.is_empty() {
        "file".to_string()
    } else {
        cleaned.chars().take(255).collect()
    }
}

/// Reject a request whose attachments exceed what the API will take.
pub fn check_request_budget(images: usize, total_bytes: u64) -> Result<()> {
    if images > MAX_IMAGES_PER_REQUEST {
        return Err(AppError::invalid(format!(
            "This message has {images} images; the limit is {MAX_IMAGES_PER_REQUEST}."
        )));
    }
    // base64 inflates by 4/3.
    let encoded = total_bytes.saturating_mul(4) / 3;
    if encoded > MAX_REQUEST_BYTES {
        return Err(AppError::invalid(format!(
            "The attachments total {} once encoded — the limit per request is {}.",
            human(encoded),
            human(MAX_REQUEST_BYTES)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn write(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn accepts_source_files_and_inlines_their_text() {
        let d = tmp();
        let f = write(d.path(), "main.rs", b"fn main() {}\n");
        let v = ingest(&f, &d.path().join("store")).unwrap();
        assert_eq!(v.kind, AttachmentKind::Text);
        assert_eq!(v.text_content.as_deref(), Some("fn main() {}\n"));
    }

    #[test]
    fn accepts_extensionless_text_like_a_makefile() {
        let d = tmp();
        let f = write(d.path(), "Makefile", b"all:\n\techo hi\n");
        let v = ingest(&f, &d.path().join("store")).unwrap();
        assert_eq!(v.kind, AttachmentKind::Text);
    }

    #[test]
    fn rejects_binary_files_with_an_explanation() {
        let d = tmp();
        // ELF header followed by NUL bytes.
        let f = write(d.path(), "a.out", &[0x7f, b'E', b'L', b'F', 0, 0, 0, 0, 0]);
        let err = ingest(&f, &d.path().join("store")).unwrap_err().to_string();
        assert!(err.contains("not a file type Claude can read"), "{err}");
    }

    #[test]
    fn rejects_empty_and_missing_files() {
        let d = tmp();
        let f = write(d.path(), "empty.txt", b"");
        assert!(ingest(&f, &d.path().join("store"))
            .unwrap_err()
            .to_string()
            .contains("empty"));
        assert!(ingest(&d.path().join("nope.txt"), &d.path().join("store")).is_err());
    }

    #[test]
    fn rejects_a_directory_with_actionable_advice() {
        let d = tmp();
        let sub = d.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        let err = ingest(&sub, &d.path().join("store"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("folder"), "{err}");
    }

    #[test]
    fn enforces_the_text_size_limit() {
        let d = tmp();
        let big = vec![b'a'; (MAX_TEXT_BYTES + 1) as usize];
        let f = write(d.path(), "big.txt", &big);
        let err = ingest(&f, &d.path().join("store")).unwrap_err().to_string();
        assert!(err.contains("limit for this file type"), "{err}");
    }

    #[test]
    fn rejects_non_utf8_masquerading_as_text() {
        let d = tmp();
        let f = write(d.path(), "bad.txt", &[0xff, 0xfe, 0xfd]);
        let err = ingest(&f, &d.path().join("store")).unwrap_err().to_string();
        assert!(
            err.contains("UTF-8") || err.contains("not a file type"),
            "{err}"
        );
    }

    #[test]
    fn identical_content_is_stored_once() {
        let d = tmp();
        let store = d.path().join("store");
        let a = write(d.path(), "a.txt", b"same bytes");
        let b = write(d.path(), "b.txt", b"same bytes");
        let va = ingest(&a, &store).unwrap();
        let vb = ingest(&b, &store).unwrap();
        assert_eq!(va.sha256, vb.sha256);
        assert_eq!(va.storage_path, vb.storage_path);
        // Different display names, one blob.
        assert_ne!(va.filename, vb.filename);
        let count = walk(&store);
        assert_eq!(count, 1, "expected a single deduplicated blob");
    }

    fn walk(dir: &Path) -> usize {
        let mut n = 0;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    n += walk(&e.path());
                } else {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn filenames_are_reduced_to_a_safe_bare_name() {
        // Traversal collapses to the final segment rather than being escaped.
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("a/b\\c.txt"), "c.txt");
        assert_eq!(sanitize_filename("/tmp/report.pdf"), "report.pdf");
        // Nothing usable left.
        assert_eq!(sanitize_filename("  "), "file");
        assert_eq!(sanitize_filename("../.."), "file");
        assert_eq!(sanitize_filename("/"), "file");
        // A dotfile does not stay hidden once attached.
        assert_eq!(sanitize_filename(".bashrc"), "bashrc");
        // Control characters are dropped, not substituted.
        assert_eq!(sanitize_filename("we\u{7}ird\nname.txt"), "weirdname.txt");
        assert_eq!(sanitize_filename("ok.md"), "ok.md");
    }

    #[test]
    fn request_budget_accounts_for_base64_inflation() {
        assert!(check_request_budget(1, 1024).is_ok());
        assert!(check_request_budget(MAX_IMAGES_PER_REQUEST + 1, 1024).is_err());
        // 30 MB of raw bytes becomes 40 MB encoded, over the 32 MB cap.
        assert!(check_request_budget(1, 30 * 1024 * 1024).is_err());
    }

    #[test]
    fn pasted_images_are_validated_by_mime_type() {
        let d = tmp();
        assert!(ingest_bytes("p.png", "image/png", b"\x89PNG\r\n", &d.path().join("s")).is_ok());
        assert!(ingest_bytes("p.bmp", "image/bmp", b"BM", &d.path().join("s")).is_err());
        assert!(ingest_bytes("p.png", "image/png", b"", &d.path().join("s")).is_err());
    }
}
