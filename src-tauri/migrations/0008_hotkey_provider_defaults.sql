UPDATE app_settings
SET spotlight_shortcut = 'Alt+Shift+F'
WHERE id = 1
  AND (
    spotlight_shortcut IS NULL
    OR TRIM(spotlight_shortcut) = ''
    OR LOWER(REPLACE(spotlight_shortcut, ' ', '')) = 'ctrl+shift+space'
  );

UPDATE app_settings
SET translation_provider = 'youdao'
WHERE id = 1
  AND (
    translation_provider IS NULL
    OR LOWER(TRIM(translation_provider)) IN ('', 'google', 'local')
  );
