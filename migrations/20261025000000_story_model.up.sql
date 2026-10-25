CREATE TABLE universes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE containers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('book_series', 'tv_series', 'collection'))
);

CREATE INDEX containers_universe_position ON containers(universe_id, position);

CREATE TABLE pieces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
    container_id INTEGER NULL REFERENCES containers(id) ON DELETE CASCADE,
    parent_id INTEGER NULL REFERENCES pieces(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    label TEXT NOT NULL CHECK (label IN ('piece', 'short_story', 'novella', 'novel', 'screenplay', 'tv_script', 'episode', 'prologue', 'chapter', 'epilogue', 'scene')),
    format TEXT NOT NULL CHECK (format IN ('prose', 'script')),
    shape TEXT NOT NULL CHECK (shape IN ('leaf', 'group')),
    content TEXT NULL CHECK (content IS NULL OR json_valid(content)),
    head_rev INTEGER NULL CHECK (head_rev IS NULL OR head_rev >= 0),
    snapshot_rev INTEGER NULL CHECK (snapshot_rev IS NULL OR (snapshot_rev >= 0 AND snapshot_rev <= head_rev)),
    legacy_book_id INTEGER NULL UNIQUE,
    legacy_chapter_id INTEGER NULL UNIQUE,
    CHECK (shape <> 'group' OR (parent_id IS NULL AND content IS NULL AND head_rev IS NULL AND snapshot_rev IS NULL AND legacy_chapter_id IS NULL)),
    CHECK (shape <> 'leaf' OR (content IS NOT NULL AND head_rev IS NOT NULL AND snapshot_rev IS NOT NULL AND snapshot_rev <= head_rev AND legacy_book_id IS NULL)),
    CHECK (parent_id IS NULL OR container_id IS NULL)
);

CREATE INDEX pieces_universe ON pieces(universe_id);
CREATE INDEX pieces_container_position ON pieces(container_id, position);
CREATE INDEX pieces_parent_position ON pieces(parent_id, position);

CREATE TABLE piece_revisions (
    piece_id INTEGER NOT NULL REFERENCES pieces(id) ON DELETE CASCADE,
    rev INTEGER NOT NULL CHECK (rev > 0),
    delta TEXT NOT NULL CHECK (json_valid(delta)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (piece_id, rev)
);

CREATE TRIGGER pieces_parent_insert BEFORE INSERT ON pieces
WHEN NEW.parent_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM pieces p
    WHERE p.id = NEW.parent_id AND p.shape = 'group' AND p.universe_id = NEW.universe_id AND p.format = NEW.format
)
BEGIN
    SELECT RAISE(ABORT, 'piece parent must be a group in the same universe and format');
END;

CREATE TRIGGER pieces_parent_update BEFORE UPDATE OF parent_id, universe_id, format ON pieces
WHEN NEW.parent_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM pieces p
    WHERE p.id = NEW.parent_id AND p.shape = 'group' AND p.universe_id = NEW.universe_id AND p.format = NEW.format
)
BEGIN
    SELECT RAISE(ABORT, 'piece parent must be a group in the same universe and format');
END;

CREATE TRIGGER pieces_group_update BEFORE UPDATE OF shape, universe_id ON pieces
WHEN (NEW.shape <> 'group' OR NEW.universe_id <> OLD.universe_id)
    AND EXISTS (SELECT 1 FROM pieces c WHERE c.parent_id = OLD.id)
BEGIN
    SELECT RAISE(ABORT, 'a group with children must stay a group in its universe');
END;

CREATE TRIGGER pieces_group_format AFTER UPDATE OF format ON pieces
WHEN NEW.shape = 'group' AND NEW.format <> OLD.format
BEGIN
    UPDATE pieces SET format = NEW.format WHERE parent_id = NEW.id;
END;

CREATE TRIGGER pieces_container_insert BEFORE INSERT ON pieces
WHEN NEW.container_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM containers c WHERE c.id = NEW.container_id AND c.universe_id = NEW.universe_id
)
BEGIN
    SELECT RAISE(ABORT, 'piece container must be in the same universe');
END;

CREATE TRIGGER pieces_container_update BEFORE UPDATE OF container_id, universe_id ON pieces
WHEN NEW.container_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM containers c WHERE c.id = NEW.container_id AND c.universe_id = NEW.universe_id
)
BEGIN
    SELECT RAISE(ABORT, 'piece container must be in the same universe');
END;

CREATE TRIGGER pieces_membership_insert BEFORE INSERT ON pieces
WHEN EXISTS (
    SELECT 1 FROM containers c WHERE c.id = NEW.container_id AND (
        (c.kind = 'book_series' AND NEW.label NOT IN ('novel', 'novella'))
        OR (c.kind = 'tv_series' AND NEW.label NOT IN ('tv_script', 'episode'))
    )
)
BEGIN
    SELECT RAISE(ABORT, 'container kind does not allow this label');
END;

CREATE TRIGGER pieces_membership_update BEFORE UPDATE OF container_id, label ON pieces
WHEN EXISTS (
    SELECT 1 FROM containers c WHERE c.id = NEW.container_id AND (
        (c.kind = 'book_series' AND NEW.label NOT IN ('novel', 'novella'))
        OR (c.kind = 'tv_series' AND NEW.label NOT IN ('tv_script', 'episode'))
    )
)
BEGIN
    SELECT RAISE(ABORT, 'container kind does not allow this label');
END;

CREATE TRIGGER containers_update BEFORE UPDATE OF universe_id, kind ON containers
WHEN EXISTS (
    SELECT 1 FROM pieces p WHERE p.container_id = OLD.id AND (
        p.universe_id <> NEW.universe_id
        OR (NEW.kind = 'book_series' AND p.label NOT IN ('novel', 'novella'))
        OR (NEW.kind = 'tv_series' AND p.label NOT IN ('tv_script', 'episode'))
    )
)
BEGIN
    SELECT RAISE(ABORT, 'container change does not fit its pieces');
END;

CREATE TRIGGER piece_revisions_leaf_insert BEFORE INSERT ON piece_revisions
WHEN (SELECT shape FROM pieces WHERE id = NEW.piece_id) = 'group'
BEGIN
    SELECT RAISE(ABORT, 'a group has no revisions');
END;

CREATE TRIGGER piece_revisions_leaf_update BEFORE UPDATE OF piece_id ON piece_revisions
WHEN (SELECT shape FROM pieces WHERE id = NEW.piece_id) = 'group'
BEGIN
    SELECT RAISE(ABORT, 'a group has no revisions');
END;

CREATE TRIGGER pieces_revisions_shape BEFORE UPDATE OF shape ON pieces
WHEN NEW.shape = 'group' AND EXISTS (SELECT 1 FROM piece_revisions r WHERE r.piece_id = OLD.id)
BEGIN
    SELECT RAISE(ABORT, 'a group has no revisions');
END;

ALTER TABLE lore_entries RENAME TO lore_entries_old;

CREATE TABLE lore_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('character', 'place', 'term', 'note')),
    name TEXT NOT NULL CHECK (trim(name, char(9, 10, 11, 12, 13, 32)) <> ''),
    keywords TEXT NOT NULL DEFAULT '[]' CHECK (CASE WHEN json_valid(keywords) THEN json_type(keywords) = 'array' ELSE 0 END),
    aliases TEXT NOT NULL DEFAULT '[]' CHECK (CASE WHEN json_valid(aliases) THEN json_type(aliases) = 'array' ELSE 0 END),
    visible_text TEXT NOT NULL DEFAULT '',
    hidden_notes TEXT NOT NULL DEFAULT '',
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    level TEXT NOT NULL DEFAULT 'include' CHECK (level IN ('include', 'known_dont_reveal', 'leave_out'))
);

CREATE INDEX lore_entries_universe ON lore_entries(universe_id);

INSERT INTO universes (id, title, position)
SELECT row_number() OVER (ORDER BY id), title, position FROM series;

INSERT INTO containers (id, universe_id, title, position, kind)
SELECT id, row_number() OVER (ORDER BY id), title, position, 'book_series' FROM series;

INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape, legacy_book_id)
SELECT c.universe_id, c.id, b.title, b.position, 'novel', 'prose', 'group', b.id
FROM books b JOIN containers c ON c.id = b.series_id
ORDER BY b.id;

INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev, legacy_chapter_id)
SELECT g.universe_id, g.id, ch.title, ch.position, ch.kind, 'prose', 'leaf', ch.content, ch.head_rev, ch.snapshot_rev, ch.id
FROM chapters ch JOIN pieces g ON g.legacy_book_id = ch.book_id
ORDER BY ch.id;

INSERT INTO piece_revisions (piece_id, rev, delta, created_at)
SELECT p.id, r.rev, r.delta, r.created_at
FROM chapter_revisions r JOIN pieces p ON p.legacy_chapter_id = r.chapter_id;

INSERT INTO lore_entries (id, universe_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level)
SELECT l.id, c.universe_id, l.kind, l.name, l.keywords, l.aliases, l.visible_text, l.hidden_notes, l.pinned, l.level
FROM lore_entries_old l JOIN containers c ON c.id = l.series_id;

DELETE FROM sqlite_sequence WHERE name IN ('containers', 'lore_entries');
INSERT INTO sqlite_sequence (name, seq) SELECT 'containers', seq FROM sqlite_sequence WHERE name = 'series';
INSERT INTO sqlite_sequence (name, seq) SELECT 'lore_entries', seq FROM sqlite_sequence WHERE name = 'lore_entries_old';

CREATE TABLE legacy_ids (
    kind TEXT PRIMARY KEY CHECK (kind IN ('book', 'chapter')),
    seq INTEGER NOT NULL CHECK (seq >= 0)
);

INSERT INTO legacy_ids (kind, seq) VALUES
    ('book', max(COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'books'), 0), COALESCE((SELECT MAX(id) FROM books), 0))),
    ('chapter', max(COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'chapters'), 0), COALESCE((SELECT MAX(id) FROM chapters), 0)));

DROP TABLE chapter_revisions;
DROP TABLE lore_entries_old;
DROP TABLE chapters;
DROP TABLE books;
DROP TABLE series;

CREATE TRIGGER pieces_legacy_book AFTER INSERT ON pieces
WHEN NEW.shape = 'group' AND NEW.legacy_book_id IS NULL
BEGIN
    UPDATE legacy_ids SET seq = seq + 1 WHERE kind = 'book';
    UPDATE pieces SET legacy_book_id = (SELECT seq FROM legacy_ids WHERE kind = 'book') WHERE id = NEW.id;
END;

CREATE TRIGGER pieces_legacy_chapter AFTER INSERT ON pieces
WHEN NEW.shape = 'leaf' AND NEW.legacy_chapter_id IS NULL
BEGIN
    UPDATE legacy_ids SET seq = seq + 1 WHERE kind = 'chapter';
    UPDATE pieces SET legacy_chapter_id = (SELECT seq FROM legacy_ids WHERE kind = 'chapter') WHERE id = NEW.id;
END;

CREATE VIEW legacy_series AS
SELECT id, universe_id, title, position FROM containers WHERE kind = 'book_series';

CREATE VIEW legacy_books AS
SELECT p.legacy_book_id AS id, p.id AS piece_id, p.container_id AS series_id, p.universe_id, p.title, p.position
FROM pieces p JOIN containers c ON c.id = p.container_id
WHERE c.kind = 'book_series' AND p.shape = 'group' AND p.label = 'novel' AND p.format = 'prose';

CREATE VIEW legacy_chapters AS
SELECT l.legacy_chapter_id AS id, l.id AS piece_id, b.id AS book_id, b.series_id, l.title, l.position, l.label AS kind, l.content, l.head_rev
FROM pieces l JOIN legacy_books b ON b.piece_id = l.parent_id
WHERE l.shape = 'leaf' AND l.label IN ('prologue', 'chapter', 'epilogue');
