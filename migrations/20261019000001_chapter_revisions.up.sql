ALTER TABLE chapters ADD COLUMN head_rev INTEGER NOT NULL DEFAULT 0 CHECK (head_rev >= 0);
ALTER TABLE chapters ADD COLUMN snapshot_rev INTEGER NOT NULL DEFAULT 0 CHECK (snapshot_rev >= 0 AND snapshot_rev <= head_rev);

CREATE TABLE chapter_revisions (
    chapter_id INTEGER NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    rev INTEGER NOT NULL CHECK (rev > 0),
    delta TEXT NOT NULL CHECK (json_valid(delta)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chapter_id, rev)
);
