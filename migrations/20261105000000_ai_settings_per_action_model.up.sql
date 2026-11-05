ALTER TABLE ai_settings ADD COLUMN generate_model TEXT;
ALTER TABLE ai_settings ADD COLUMN edit_model TEXT;
UPDATE ai_settings SET generate_model = model, edit_model = model;
ALTER TABLE ai_settings DROP COLUMN model;
