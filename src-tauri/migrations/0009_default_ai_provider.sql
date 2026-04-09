UPDATE app_settings
SET api_base_url = 'https://openrouter.ai/api/v1/chat/completions',
    api_model = 'deepseek/deepseek-chat-v3-0324:free'
WHERE id = 1
  AND (
    LOWER(TRIM(api_base_url)) = 'https://api.openai.com/v1/chat/completions'
    OR LOWER(TRIM(api_model)) = 'gpt-4o-mini'
  );
