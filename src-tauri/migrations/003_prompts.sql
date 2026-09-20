-- Saved prompts: reusable message text the user keeps to hand.
--
-- `use_count` and `last_used_at` exist so the picker can lead with what is
-- actually used rather than with whatever was added first.
CREATE TABLE prompts (
    id           TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    body         TEXT NOT NULL,
    use_count    INTEGER NOT NULL DEFAULT 0,
    last_used_at INTEGER,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE INDEX prompts_recent ON prompts(last_used_at DESC);
