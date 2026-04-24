UPDATE app_settings
SET api_base_url = 'https://text.pollinations.ai/openai',
    api_model = 'openai'
WHERE id = 1
  AND (
    (
      LOWER(TRIM(api_base_url)) = 'https://api.openai.com/v1/chat/completions'
      AND LOWER(TRIM(api_model)) = 'gpt-4o-mini'
    )
    OR (
      LOWER(TRIM(api_base_url)) = 'https://openrouter.ai/api/v1/chat/completions'
      AND LOWER(TRIM(api_model)) = 'deepseek/deepseek-chat-v3-0324:free'
    )
  );
