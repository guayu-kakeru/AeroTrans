ALTER TABLE app_settings
ADD COLUMN memory_prompt TEXT NOT NULL DEFAULT 'You are a vocabulary memory coach. Return concise mnemonic content with old-word split, vivid scene, and one short sentence.';

UPDATE app_settings
SET memory_prompt = 'You are a vocabulary memory coach. Return concise mnemonic content with old-word split, vivid scene, and one short sentence.'
WHERE id = 1
  AND (memory_prompt IS NULL OR TRIM(memory_prompt) = '');