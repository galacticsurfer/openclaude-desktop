-- Claude Code reports reasoning as a token estimate rather than text, so the
-- count is the only record of it we can keep. Nullable: every row written
-- before this migration, and every reply that did no reasoning, has none.
ALTER TABLE messages ADD COLUMN thinking_tokens INTEGER;
