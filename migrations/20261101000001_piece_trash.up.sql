ALTER TABLE pieces ADD COLUMN deleted_at TEXT;

CREATE VIEW active_pieces AS
WITH RECURSIVE hidden(id) AS (
    SELECT id FROM pieces WHERE deleted_at IS NOT NULL
    UNION
    SELECT p.id FROM pieces p JOIN hidden h ON p.parent_id = h.id
)
SELECT * FROM pieces WHERE id NOT IN (SELECT id FROM hidden);

DROP VIEW legacy_chapters;
DROP VIEW legacy_books;
CREATE VIEW legacy_books AS
SELECT p.legacy_book_id AS id, p.id AS piece_id, p.container_id AS series_id, p.universe_id, p.title, p.position
FROM active_pieces p JOIN containers c ON c.id = p.container_id
WHERE c.kind = 'book_series' AND p.shape = 'group' AND p.label = 'novel' AND p.format = 'prose';
CREATE VIEW legacy_chapters AS
SELECT l.legacy_chapter_id AS id, l.id AS piece_id, b.id AS book_id, b.series_id, l.title, l.position, l.label AS kind, l.content, l.head_rev
FROM active_pieces l JOIN legacy_books b ON b.piece_id = l.parent_id
WHERE l.shape = 'leaf' AND l.label IN ('prologue', 'chapter', 'epilogue');
