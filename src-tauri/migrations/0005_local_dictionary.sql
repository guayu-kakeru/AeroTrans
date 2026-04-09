CREATE TABLE IF NOT EXISTS local_dictionary (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term TEXT NOT NULL,
  direction TEXT NOT NULL CHECK (direction IN ('en_to_zh', 'zh_to_en')),
  translation TEXT NOT NULL,
  glossary TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(term, direction)
);

INSERT OR IGNORE INTO local_dictionary (term, direction, translation, glossary) VALUES
  ('hello', 'en_to_zh', '你好', '["问候", "打招呼"]'),
  ('world', 'en_to_zh', '世界', '["地球", "全球"]'),
  ('architecture', 'en_to_zh', '架构', '["系统设计", "结构"]'),
  ('latency', 'en_to_zh', '延迟', '["响应时间", "时延"]'),
  ('robust', 'en_to_zh', '健壮的', '["稳定", "可靠"]'),
  ('翻译', 'zh_to_en', 'translate', '["translation", "interpret"]'),
  ('窗口', 'zh_to_en', 'window', '["pane", "dialog"]'),
  ('极简', 'zh_to_en', 'minimalist', '["minimal", "concise"]'),
  ('词典', 'zh_to_en', 'dictionary', '["lexicon", "glossary"]'),
  ('快捷键', 'zh_to_en', 'shortcut', '["hotkey", "keybinding"]');
