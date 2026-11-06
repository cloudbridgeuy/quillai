ALTER TABLE lore_images ADD COLUMN purpose TEXT NOT NULL DEFAULT 'portrait' CHECK (purpose IN ('portrait', 'profile', 'map', 'render'));
ALTER TABLE lore_images ADD COLUMN time_of_day TEXT CHECK (time_of_day IN ('dawn', 'day', 'dusk', 'night'));
