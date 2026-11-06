CREATE TABLE lore_images (
    id INTEGER PRIMARY KEY,
    lore_entry_id INTEGER NOT NULL REFERENCES lore_entries(id) ON DELETE CASCADE,
    timeline_point_id INTEGER REFERENCES timeline_points(id) ON DELETE SET NULL,
    prompt TEXT NOT NULL,
    model TEXT NOT NULL,
    size TEXT NOT NULL,
    quality TEXT NOT NULL,
    mime TEXT NOT NULL,
    bytes BLOB NOT NULL,
    cost REAL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX lore_images_entry ON lore_images (lore_entry_id, id);
