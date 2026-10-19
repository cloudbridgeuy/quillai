DROP TABLE chapter_revisions;
ALTER TABLE chapters DROP COLUMN snapshot_rev;
ALTER TABLE chapters DROP COLUMN head_rev;
