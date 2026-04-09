UPDATE app_settings
SET translation_provider = 'google'
WHERE translation_provider IS NULL OR translation_provider IN ('', 'local');
