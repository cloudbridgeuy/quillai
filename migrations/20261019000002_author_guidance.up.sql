CREATE TABLE author_guidance (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    text TEXT NOT NULL CHECK (length(CAST(text AS BLOB)) <= 65536)
);
