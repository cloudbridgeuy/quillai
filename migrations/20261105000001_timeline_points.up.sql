CREATE TABLE timeline_points (
    id INTEGER PRIMARY KEY,
    universe_id INTEGER NOT NULL REFERENCES universes(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    position INTEGER NOT NULL,
    deleted_at TEXT
);

CREATE INDEX timeline_points_universe_position ON timeline_points (universe_id, position);

ALTER TABLE pieces ADD COLUMN timeline_point_id INTEGER REFERENCES timeline_points(id) ON DELETE SET NULL;
