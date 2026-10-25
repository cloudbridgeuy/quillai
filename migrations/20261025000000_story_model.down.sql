CREATE TABLE story_model_down_guard (ok INTEGER NOT NULL);

CREATE TRIGGER story_model_down_guard_check BEFORE INSERT ON story_model_down_guard
WHEN EXISTS (
    SELECT 1 FROM universes u
    WHERE (SELECT COUNT(*) FROM containers c WHERE c.universe_id = u.id) <> 1
)
OR EXISTS (SELECT 1 FROM containers WHERE kind <> 'book_series')
OR EXISTS (SELECT 1 FROM pieces WHERE parent_id IS NULL AND container_id IS NULL)
OR EXISTS (
    SELECT 1 FROM pieces
    WHERE shape = 'group' AND (label <> 'novel' OR format <> 'prose' OR legacy_book_id IS NULL)
)
OR EXISTS (
    SELECT 1 FROM pieces
    WHERE shape = 'leaf' AND (
        parent_id IS NULL
        OR label NOT IN ('prologue', 'chapter', 'epilogue')
        OR format <> 'prose'
        OR legacy_chapter_id IS NULL
    )
)
OR EXISTS (
    SELECT 1 FROM lore_entries l
    WHERE NOT EXISTS (SELECT 1 FROM containers c WHERE c.universe_id = l.universe_id)
)
BEGIN
    SELECT RAISE(ABORT, 'story model rows do not fit series, books and chapters');
END;

INSERT INTO story_model_down_guard (ok) VALUES (1);

DROP VIEW legacy_chapters;
DROP VIEW legacy_books;
DROP VIEW legacy_series;

CREATE TABLE series (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE INDEX books_series_position ON books(series_id, position);

CREATE TABLE chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('chapter', 'prologue', 'epilogue')),
    content TEXT NOT NULL DEFAULT '{"ops":[]}' CHECK (json_valid(content)),
    head_rev INTEGER NOT NULL DEFAULT 0 CHECK (head_rev >= 0),
    snapshot_rev INTEGER NOT NULL DEFAULT 0 CHECK (snapshot_rev >= 0 AND snapshot_rev <= head_rev)
);

CREATE INDEX chapters_book_position ON chapters(book_id, position);

CREATE TABLE chapter_revisions (
    chapter_id INTEGER NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    rev INTEGER NOT NULL CHECK (rev > 0),
    delta TEXT NOT NULL CHECK (json_valid(delta)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chapter_id, rev)
);

ALTER TABLE lore_entries RENAME TO lore_entries_new;

CREATE TABLE lore_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('character', 'place', 'term', 'note')),
    name TEXT NOT NULL CHECK (trim(name, char(9, 10, 11, 12, 13, 32)) <> ''),
    keywords TEXT NOT NULL DEFAULT '[]' CHECK (CASE WHEN json_valid(keywords) THEN json_type(keywords) = 'array' ELSE 0 END),
    aliases TEXT NOT NULL DEFAULT '[]' CHECK (CASE WHEN json_valid(aliases) THEN json_type(aliases) = 'array' ELSE 0 END),
    visible_text TEXT NOT NULL DEFAULT '',
    hidden_notes TEXT NOT NULL DEFAULT '',
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    level TEXT NOT NULL DEFAULT 'include' CHECK (level IN ('include', 'known_dont_reveal', 'leave_out'))
);

CREATE INDEX lore_entries_series ON lore_entries(series_id);

INSERT INTO series (id, title, position)
SELECT id, title, position FROM containers ORDER BY id;

INSERT INTO books (id, series_id, title, position)
SELECT legacy_book_id, container_id, title, position FROM pieces
WHERE shape = 'group' ORDER BY legacy_book_id;

INSERT INTO chapters (id, book_id, title, position, kind, content, head_rev, snapshot_rev)
SELECT l.legacy_chapter_id, g.legacy_book_id, l.title, l.position, l.label, l.content, l.head_rev, l.snapshot_rev
FROM pieces l JOIN pieces g ON g.id = l.parent_id
WHERE l.shape = 'leaf' ORDER BY l.legacy_chapter_id;

INSERT INTO chapter_revisions (chapter_id, rev, delta, created_at)
SELECT p.legacy_chapter_id, r.rev, r.delta, r.created_at
FROM piece_revisions r JOIN pieces p ON p.id = r.piece_id;

INSERT INTO lore_entries (id, series_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level)
SELECT l.id, c.id, l.kind, l.name, l.keywords, l.aliases, l.visible_text, l.hidden_notes, l.pinned, l.level
FROM lore_entries_new l JOIN containers c ON c.universe_id = l.universe_id;

DELETE FROM sqlite_sequence WHERE name IN ('series', 'lore_entries');
INSERT INTO sqlite_sequence (name, seq) SELECT 'series', seq FROM sqlite_sequence WHERE name = 'containers';
INSERT INTO sqlite_sequence (name, seq) SELECT 'lore_entries', seq FROM sqlite_sequence WHERE name = 'lore_entries_new';
DELETE FROM sqlite_sequence WHERE name IN ('books', 'chapters');
INSERT INTO sqlite_sequence (name, seq) SELECT 'books', seq FROM legacy_ids WHERE kind = 'book' AND seq > 0;
INSERT INTO sqlite_sequence (name, seq) SELECT 'chapters', seq FROM legacy_ids WHERE kind = 'chapter' AND seq > 0;

DROP TABLE piece_revisions;
DROP TABLE lore_entries_new;
DROP TABLE pieces;
DROP TABLE containers;
DROP TABLE universes;
DROP TABLE legacy_ids;
DROP TABLE story_model_down_guard;
