use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionMode {
    SilentAll,
    ManualStar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationProvider {
    Google,
    Youdao,
    Mymemory,
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub spotlight_shortcut: String,
    pub selection_shortcut: String,
    pub companion_enabled: bool,
    pub companion_opacity: i64,
    pub companion_mouse_through: bool,
    pub tts_enabled: bool,
    pub collection_mode: CollectionMode,
    pub translation_provider: TranslationProvider,
    pub memory_prompt: String,
}

impl Default for SettingsDto {
    fn default() -> Self {
        Self {
            spotlight_shortcut: "Alt+Shift+F".to_string(),
            selection_shortcut: "Alt+Shift+L".to_string(),
            companion_enabled: false,
            companion_opacity: 88,
            companion_mouse_through: false,
            tts_enabled: true,
            collection_mode: CollectionMode::ManualStar,
            translation_provider: TranslationProvider::Youdao,
            memory_prompt: "You are a vocabulary memory coach for Chinese learners. Create short, vivid memory notes that feel tailored to the word. Vary the structure naturally between requests, avoid boilerplate labels, and explain the word with the strongest memory hook instead of a fixed template.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfigDto {
    pub base_url: String,
    pub model: String,
}

impl Default for ApiConfigDto {
    fn default() -> Self {
        Self {
            base_url: "https://text.pollinations.ai/openai".to_string(),
            model: "openai".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRequestDto {
    pub text: String,
    pub context: Option<String>,
    pub provider: Option<TranslationProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResultDto {
    pub detected_direction: String,
    pub translation: String,
    pub glossary: Vec<String>,
    pub phonetics: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabularyDto {
    pub id: Option<i64>,
    pub term: String,
    pub translation: String,
    pub context_text: Option<String>,
    pub starred: bool,
    pub ai_memory: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutCheckResultDto {
    pub conflict: bool,
    pub reason: Option<String>,
}
