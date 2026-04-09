ALTER TABLE app_settings ADD COLUMN translation_provider TEXT NOT NULL DEFAULT 'local';

UPDATE app_settings
SET translation_provider = 'local'
WHERE translation_provider IS NULL OR trim(translation_provider) = '';
