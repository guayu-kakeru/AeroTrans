CREATE TABLE IF NOT EXISTS app_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  spotlight_shortcut TEXT NOT NULL,
  selection_shortcut TEXT NOT NULL,
  companion_enabled INTEGER NOT NULL DEFAULT 0,
  companion_opacity INTEGER NOT NULL DEFAULT 88,
  companion_mouse_through INTEGER NOT NULL DEFAULT 0,
  tts_enabled INTEGER NOT NULL DEFAULT 1,
  collection_mode TEXT NOT NULL DEFAULT 'manual_star',
  api_base_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1/chat/completions',
  api_model TEXT NOT NULL DEFAULT 'gpt-4o-mini',
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO app_settings (
  id,
  spotlight_shortcut,
  selection_shortcut,
  companion_enabled,
  companion_opacity,
  companion_mouse_through,
  tts_enabled,
  collection_mode,
  api_base_url,
  api_model
)
SELECT
  1,
  'Ctrl+Shift+Space',
  'Ctrl+Shift+D',
  0,
  88,
  0,
  1,
  'manual_star',
  'https://api.openai.com/v1/chat/completions',
  'gpt-4o-mini'
WHERE NOT EXISTS (SELECT 1 FROM app_settings WHERE id = 1);
