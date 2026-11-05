DROP VIEW legacy_chapters;
DROP VIEW legacy_books;
DROP VIEW legacy_series;
DROP VIEW active_pieces;
ALTER TABLE pieces DROP COLUMN timeline_point_id;
DROP TABLE timeline_points;
CREATE VIEW active_pieces AS
WITH RECURSIVE hidden(id) AS (
    SELECT p.id FROM pieces p JOIN universes u ON u.id = p.universe_id
    LEFT JOIN containers c ON c.id = p.container_id
    WHERE p.deleted_at IS NOT NULL OR u.deleted_at IS NOT NULL OR c.deleted_at IS NOT NULL
    UNION
    SELECT p.id FROM pieces p JOIN hidden h ON p.parent_id = h.id
)
SELECT * FROM pieces WHERE id NOT IN (SELECT id FROM hidden);
CREATE VIEW legacy_series AS
SELECT c.id, c.universe_id, c.title, c.position FROM containers c JOIN universes u ON u.id = c.universe_id
WHERE c.kind = 'book_series' AND c.deleted_at IS NULL AND u.deleted_at IS NULL;
CREATE VIEW legacy_books AS
SELECT p.legacy_book_id AS id, p.id AS piece_id, p.container_id AS series_id, p.universe_id, p.title, p.position
FROM active_pieces p JOIN legacy_series c ON c.id = p.container_id
WHERE p.shape = 'group' AND p.label = 'novel' AND p.format = 'prose';
CREATE VIEW legacy_chapters AS
SELECT l.legacy_chapter_id AS id, l.id AS piece_id, b.id AS book_id, b.series_id, l.title, l.position, l.label AS kind, l.content, l.head_rev
FROM active_pieces l JOIN legacy_books b ON b.piece_id = l.parent_id
WHERE l.shape = 'leaf' AND l.label IN ('prologue', 'chapter', 'epilogue');
