ALTER TABLE containers ADD COLUMN language TEXT NOT NULL DEFAULT 'en' CHECK (language IN ('en', 'es'));
ALTER TABLE containers ADD COLUMN rules_config TEXT NOT NULL DEFAULT '{}';
