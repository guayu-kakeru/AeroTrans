use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionMode {
    SilentAll,
    ManualStar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsDto {
    pub spotlight_shortcut: String,
    pub selection_shortcut: String,
    pub companion_enabled: bool,
    pub companion_opacity: i64,
    pub companion_mouse_through: bool,
    pub tts_enabled: bool,
    pub collection_mode: CollectionMode,
}

impl Default for SettingsDto {
    fn default() -> Self {
        Self {
            spotlight_shortcut: "Ctrl+Shift+Space".to_string(),
            selection_shortcut: "Ctrl+Shift+D".to_string(),
            companion_enabled: false,
            companion_opacity: 88,
            companion_mouse_through: false,
            tts_enabled: true,
            collection_mode: CollectionMode::ManualStar,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfigDto {
    pub base_url: String,
    pub model: String,
}

impl Default for ApiConfigDto {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4o-mini".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequestDto {
    pub text: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResultDto {
    pub detected_direction: String,
    pub translation: String,
    pub glossary: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
