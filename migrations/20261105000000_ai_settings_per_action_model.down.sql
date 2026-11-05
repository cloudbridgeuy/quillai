ALTER TABLE ai_settings ADD COLUMN model TEXT;
UPDATE ai_settings SET model = generate_model;
ALTER TABLE ai_settings DROP COLUMN edit_model;
ALTER TABLE ai_settings DROP COLUMN generate_model;
