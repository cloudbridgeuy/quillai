CREATE TABLE lore_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('character', 'place', 'term', 'note')),
    name TEXT NOT NULL CHECK (trim(name, char(9, 10, 11, 12, 13, 32)) <> ''),
    keywords TEXT NOT NULL DEFAULT '[]' CHECK (CASE WHEN json_valid(keywords) THEN json_type(keywords) = 'array' ELSE 0 END),
    visible_text TEXT NOT NULL DEFAULT '',
    hidden_notes TEXT NOT NULL DEFAULT '',
    pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    level TEXT NOT NULL DEFAULT 'include' CHECK (level IN ('include', 'known_dont_reveal', 'leave_out'))
);

CREATE INDEX lore_entries_series ON lore_entries(series_id);
