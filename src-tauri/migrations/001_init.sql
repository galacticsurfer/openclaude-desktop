-- OpenClaude Desktop — schema v1
-- All timestamps are INTEGER unix epoch milliseconds (UTC).
-- All ids are TEXT UUIDv4 unless noted.

CREATE TABLE projects (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    instructions  TEXT NOT NULL DEFAULT '',
    working_dir   TEXT,
    default_model TEXT,
    color         TEXT,
    sort_order    INTEGER NOT NULL DEFAULT 0,
    archived      INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    metadata      TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_projects_archived ON projects (archived, sort_order, name);

CREATE TABLE conversations (
    id                       TEXT PRIMARY KEY,
    title                    TEXT NOT NULL DEFAULT 'New conversation',
    -- 0 = placeholder/derived title, 1 = model-generated or user-set (never auto-overwrite)
    title_locked             INTEGER NOT NULL DEFAULT 0 CHECK (title_locked IN (0, 1)),
    provider                 TEXT NOT NULL DEFAULT 'anthropic',
    provider_conversation_id TEXT,
    model                    TEXT NOT NULL,
    system_prompt            TEXT,
    project_id               TEXT REFERENCES projects (id) ON DELETE SET NULL,
    pinned                   INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    archived                 INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    -- soft delete: rows with deleted_at set live in Trash until purged
    deleted_at               INTEGER,
    branched_from_message_id TEXT,
    created_at               INTEGER NOT NULL,
    updated_at               INTEGER NOT NULL,
    last_message_at          INTEGER,
    metadata                 TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_conversations_list
    ON conversations (deleted_at, archived, pinned DESC, last_message_at DESC);
CREATE INDEX idx_conversations_project ON conversations (project_id, last_message_at DESC);

CREATE TABLE messages (
    id                  TEXT PRIMARY KEY,
    conversation_id     TEXT NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    -- dense, monotonic ordering within a conversation; never reused
    seq                 INTEGER NOT NULL,
    role                TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content             TEXT NOT NULL DEFAULT '',
    -- reasoning/thinking output, kept separate so it is never replayed as context
    thinking            TEXT,
    status              TEXT NOT NULL DEFAULT 'complete'
                          CHECK (status IN ('pending', 'streaming', 'complete', 'interrupted', 'error')),
    model               TEXT,
    provider_message_id TEXT,
    stop_reason         TEXT,
    input_tokens        INTEGER,
    output_tokens       INTEGER,
    cache_read_tokens   INTEGER,
    cache_write_tokens  INTEGER,
    error_kind          TEXT,
    error_message       TEXT,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,
    metadata            TEXT NOT NULL DEFAULT '{}',
    UNIQUE (conversation_id, seq)
);
CREATE INDEX idx_messages_conversation ON messages (conversation_id, seq);
CREATE INDEX idx_messages_status ON messages (status) WHERE status IN ('pending', 'streaming');

CREATE TABLE attachments (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT REFERENCES conversations (id) ON DELETE CASCADE,
    message_id      TEXT REFERENCES messages (id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects (id) ON DELETE CASCADE,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('image', 'document', 'text')),
    -- content-addressed path relative to the attachment store root
    storage_path    TEXT NOT NULL,
    sha256          TEXT NOT NULL,
    -- inlined UTF-8 for text attachments; NULL for binary
    text_content    TEXT,
    created_at      INTEGER NOT NULL,
    metadata        TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_attachments_message ON attachments (message_id);
CREATE INDEX idx_attachments_conversation ON attachments (conversation_id);
CREATE INDEX idx_attachments_sha ON attachments (sha256);

CREATE TABLE mcp_servers (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    transport   TEXT NOT NULL DEFAULT 'stdio' CHECK (transport IN ('stdio', 'sse', 'http')),
    command     TEXT NOT NULL DEFAULT '',
    args        TEXT NOT NULL DEFAULT '[]',
    env         TEXT NOT NULL DEFAULT '{}',
    url         TEXT,
    enabled     INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    project_id  TEXT REFERENCES projects (id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    metadata    TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE mcp_permissions (
    id         TEXT PRIMARY KEY,
    server_id  TEXT NOT NULL REFERENCES mcp_servers (id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects (id) ON DELETE CASCADE,
    tool_name  TEXT NOT NULL,
    category   TEXT NOT NULL,
    decision   TEXT NOT NULL CHECK (decision IN ('allow', 'deny', 'ask')),
    scope      TEXT NOT NULL DEFAULT 'project' CHECK (scope IN ('once', 'project', 'global')),
    created_at INTEGER NOT NULL,
    UNIQUE (server_id, project_id, tool_name)
);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- ---------------------------------------------------------------------------
-- Full-text search (FTS5, external-content tables kept in sync by triggers)
-- ---------------------------------------------------------------------------

CREATE VIRTUAL TABLE messages_fts USING fts5 (
    content,
    content = 'messages',
    content_rowid = 'rowid',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER messages_fts_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts (rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER messages_fts_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts (messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;
CREATE TRIGGER messages_fts_au AFTER UPDATE OF content ON messages BEGIN
    INSERT INTO messages_fts (messages_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO messages_fts (rowid, content) VALUES (new.rowid, new.content);
END;

CREATE VIRTUAL TABLE conversations_fts USING fts5 (
    title,
    content = 'conversations',
    content_rowid = 'rowid',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER conversations_fts_ai AFTER INSERT ON conversations BEGIN
    INSERT INTO conversations_fts (rowid, title) VALUES (new.rowid, new.title);
END;
CREATE TRIGGER conversations_fts_ad AFTER DELETE ON conversations BEGIN
    INSERT INTO conversations_fts (conversations_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
END;
CREATE TRIGGER conversations_fts_au AFTER UPDATE OF title ON conversations BEGIN
    INSERT INTO conversations_fts (conversations_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
    INSERT INTO conversations_fts (rowid, title) VALUES (new.rowid, new.title);
END;

CREATE VIRTUAL TABLE attachments_fts USING fts5 (
    filename,
    content = 'attachments',
    content_rowid = 'rowid',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER attachments_fts_ai AFTER INSERT ON attachments BEGIN
    INSERT INTO attachments_fts (rowid, filename) VALUES (new.rowid, new.filename);
END;
CREATE TRIGGER attachments_fts_ad AFTER DELETE ON attachments BEGIN
    INSERT INTO attachments_fts (attachments_fts, rowid, filename) VALUES ('delete', old.rowid, old.filename);
END;
