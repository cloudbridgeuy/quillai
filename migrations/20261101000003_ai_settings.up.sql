CREATE TABLE ai_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    base_url TEXT,
    api_key TEXT,
    model TEXT
);
