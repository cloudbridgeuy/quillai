DROP VIEW legacy_chapters;
DROP VIEW legacy_books;
DROP VIEW active_pieces;
ALTER TABLE pieces DROP COLUMN deleted_at;
CREATE VIEW legacy_books AS
SELECT p.legacy_book_id AS id, p.id AS piece_id, p.container_id AS series_id, p.universe_id, p.title, p.position
FROM pieces p JOIN containers c ON c.id = p.container_id
WHERE c.kind = 'book_series' AND p.shape = 'group' AND p.label = 'novel' AND p.format = 'prose';
CREATE VIEW legacy_chapters AS
SELECT l.legacy_chapter_id AS id, l.id AS piece_id, b.id AS book_id, b.series_id, l.title, l.position, l.label AS kind, l.content, l.head_rev
FROM pieces l JOIN legacy_books b ON b.piece_id = l.parent_id
WHERE l.shape = 'leaf' AND l.label IN ('prologue', 'chapter', 'epilogue');
