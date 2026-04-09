UPDATE app_settings
SET selection_shortcut = 'Alt+Shift+L'
WHERE id = 1
  AND (
    selection_shortcut IS NULL
    OR TRIM(selection_shortcut) = ''
    OR LOWER(REPLACE(selection_shortcut, ' ', '')) = 'ctrl+shift+d'
  );

UPDATE app_settings
SET memory_prompt = 'You are a vocabulary memory coach. Return concise mnemonic content with old-word split, vivid scene, and one short sentence.'
WHERE id = 1
  AND (
    memory_prompt IS NULL
    OR TRIM(memory_prompt) = ''
    OR memory_prompt LIKE '%????%'
  );