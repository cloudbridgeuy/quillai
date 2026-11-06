CREATE TABLE lore_entry_states (
    id INTEGER PRIMARY KEY,
    lore_entry_id INTEGER NOT NULL REFERENCES lore_entries(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    point_id INTEGER NOT NULL REFERENCES timeline_points(id) ON DELETE CASCADE,
    visible_text TEXT NOT NULL DEFAULT '',
    hidden_notes TEXT NOT NULL DEFAULT '',
    deleted_at TEXT
);

CREATE UNIQUE INDEX lore_entry_states_entry_point ON lore_entry_states (lore_entry_id, point_id) WHERE deleted_at IS NULL;
