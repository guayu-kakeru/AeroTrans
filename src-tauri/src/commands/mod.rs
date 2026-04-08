use arboard::Clipboard;
use regex::Regex;
use rusqlite::{params, Connection};
use serde_json::Value;
use tauri::State;

use crate::db::migrations::run_migrations;
use crate::models::{
    ApiConfigDto, CollectionMode, SettingsDto, ShortcutCheckResultDto, TranslationRequestDto,
    TranslationResultDto, VocabularyDto,
};
use crate::state::AppState;

const API_KEY_SLOT: &str = "openai_api_key";

fn connect_db(db_path: &str) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    run_migrations(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

fn detect_direction(input: &str) -> &'static str {
    if Regex::new(r"[\u4e00-\u9fff]").unwrap().is_match(input) {
        "zh_to_en"
    } else {
        "en_to_zh"
    }
}

fn fallback_translate(text: &str) -> TranslationResultDto {
    let direction = detect_direction(text).to_string();
    let translation = if direction == "zh_to_en" {
        format!("Fallback translation: {text}")
    } else {
        format!("兜底翻译：{text}")
    };
    let glossary = text
        .split_whitespace()
        .take(3)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    TranslationResultDto {
        detected_direction: direction,
        translation,
        glossary,
        source: "fallback".to_string(),
    }
}

async fn call_ai_translation(
    app: &AppState,
    text: &str,
    context: Option<&str>,
    for_memory: bool,
) -> Result<Option<String>, String> {
    let conn = connect_db(&app.db_path)?;
    let api = conn
        .query_row(
            "SELECT api_base_url, api_model FROM app_settings WHERE id = 1",
            [],
            |row| {
                Ok(ApiConfigDto {
                    base_url: row.get::<_, String>(0)?,
                    model: row.get::<_, String>(1)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let Some(api_key) = app.secrets.get_secret(API_KEY_SLOT).map_err(|e| e.to_string())? else {
        return Ok(None);
    };

    let system = if for_memory {
        "You are an English vocabulary coach. Return concise Chinese memory tricks including homophonic clues, roots, and a mini story."
    } else {
        "You are a bilingual translator. Return concise translation and short glossary."
    };

    let user_prompt = if for_memory {
        format!("Word: {text}\nGive memory tips in Chinese.")
    } else {
        match context {
            Some(ctx) if !ctx.trim().is_empty() => format!("Text: {text}\nContext: {ctx}"),
            _ => format!("Text: {text}"),
        }
    };

    let body = serde_json::json!({
        "model": api.model,
        "temperature": 0.2,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt}
        ]
    });

    let response = app
        .http_client
        .post(api.base_url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let json: Value = response.json().await.map_err(|e| e.to_string())?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok(content)
}

#[tauri::command]
pub fn health_check() -> String {
    "AeroTrans backend ready".to_string()
}

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> Result<SettingsDto, String> {
    let conn = connect_db(&state.db_path)?;
    let settings = conn
        .query_row(
            "SELECT spotlight_shortcut, selection_shortcut, companion_enabled, companion_opacity, companion_mouse_through, tts_enabled, collection_mode FROM app_settings WHERE id = 1",
            [],
            |row| {
                let mode_raw: String = row.get(6)?;
                let collection_mode = match mode_raw.as_str() {
                    "silent_all" => CollectionMode::SilentAll,
                    _ => CollectionMode::ManualStar,
                };
                Ok(SettingsDto {
                    spotlight_shortcut: row.get(0)?,
                    selection_shortcut: row.get(1)?,
                    companion_enabled: row.get::<_, i64>(2)? == 1,
                    companion_opacity: row.get(3)?,
                    companion_mouse_through: row.get::<_, i64>(4)? == 1,
                    tts_enabled: row.get::<_, i64>(5)? == 1,
                    collection_mode,
                })
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn save_settings(settings: SettingsDto, state: State<'_, AppState>) -> Result<(), String> {
    let conn = connect_db(&state.db_path)?;
    let mode = match settings.collection_mode {
        CollectionMode::SilentAll => "silent_all",
        CollectionMode::ManualStar => "manual_star",
    };
    conn.execute(
        "UPDATE app_settings SET spotlight_shortcut=?, selection_shortcut=?, companion_enabled=?, companion_opacity=?, companion_mouse_through=?, tts_enabled=?, collection_mode=?, updated_at=CURRENT_TIMESTAMP WHERE id = 1",
        params![
            settings.spotlight_shortcut,
            settings.selection_shortcut,
            if settings.companion_enabled { 1 } else { 0 },
            settings.companion_opacity,
            if settings.companion_mouse_through { 1 } else { 0 },
            if settings.tts_enabled { 1 } else { 0 },
            mode
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn load_api_config(state: State<'_, AppState>) -> Result<ApiConfigDto, String> {
    let conn = connect_db(&state.db_path)?;
    let config = conn
        .query_row(
            "SELECT api_base_url, api_model FROM app_settings WHERE id = 1",
            [],
            |row| {
                Ok(ApiConfigDto {
                    base_url: row.get(0)?,
                    model: row.get(1)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn save_api_config(config: ApiConfigDto, state: State<'_, AppState>) -> Result<(), String> {
    if config.base_url.trim().is_empty() || config.model.trim().is_empty() {
        return Err("API URL 与模型名不能为空".to_string());
    }
    let conn = connect_db(&state.db_path)?;
    conn.execute(
        "UPDATE app_settings SET api_base_url=?, api_model=?, updated_at=CURRENT_TIMESTAMP WHERE id=1",
        params![config.base_url, config.model],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("API Key 不能为空".to_string());
    }
    state
        .secrets
        .set_secret(API_KEY_SLOT, api_key.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_api_key(state: State<'_, AppState>) -> Result<bool, String> {
    let key = state
        .secrets
        .get_secret(API_KEY_SLOT)
        .map_err(|e| e.to_string())?;
    Ok(key.is_some())
}

#[tauri::command]
pub fn check_shortcut_conflict(shortcut: String) -> ShortcutCheckResultDto {
    let normalized = shortcut.trim().to_ascii_lowercase();
    let reserved = [
        "alt+f4",
        "ctrl+c",
        "ctrl+v",
        "ctrl+alt+del",
        "win+l",
        "cmd+q",
    ];
    if reserved.iter().any(|item| *item == normalized) {
        return ShortcutCheckResultDto {
            conflict: true,
            reason: Some("快捷键与系统保留键冲突".to_string()),
        };
    }
    ShortcutCheckResultDto {
        conflict: false,
        reason: None,
    }
}

#[tauri::command]
pub async fn translate_text(
    request: TranslationRequestDto,
    state: State<'_, AppState>,
) -> Result<TranslationResultDto, String> {
    let fallback = fallback_translate(&request.text);
    let ai_content = call_ai_translation(
        &state,
        &request.text,
        request.context.as_deref(),
        false,
    )
    .await?;

    let Some(ai_text) = ai_content else {
        return Ok(fallback);
    };

    let glossary = request
        .text
        .split_whitespace()
        .take(3)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Ok(TranslationResultDto {
        detected_direction: detect_direction(&request.text).to_string(),
        translation: ai_text,
        glossary,
        source: "ai".to_string(),
    })
}

#[tauri::command]
pub fn add_vocabulary(item: VocabularyDto, state: State<'_, AppState>) -> Result<i64, String> {
    let conn = connect_db(&state.db_path)?;
    conn.execute(
        "INSERT INTO vocabulary (term, translation, context_text, starred, ai_memory, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        params![item.term, item.translation, item.context_text, if item.starred { 1 } else { 0 }, item.ai_memory],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn list_vocabulary(state: State<'_, AppState>) -> Result<Vec<VocabularyDto>, String> {
    let conn = connect_db(&state.db_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, term, translation, context_text, starred, ai_memory, created_at FROM vocabulary ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(VocabularyDto {
                id: Some(row.get(0)?),
                term: row.get(1)?,
                translation: row.get(2)?,
                context_text: row.get(3)?,
                starred: row.get::<_, i64>(4)? == 1,
                ai_memory: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let result = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn delete_vocabulary(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let conn = connect_db(&state.db_path)?;
    conn.execute("DELETE FROM vocabulary WHERE id = ?", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn generate_memory_for_word(id: i64, state: State<'_, AppState>) -> Result<String, String> {
    let conn = connect_db(&state.db_path)?;
    let term: String = conn
        .query_row("SELECT term FROM vocabulary WHERE id = ?", params![id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let generated = call_ai_translation(&state, &term, None, true)
        .await?
        .unwrap_or_else(|| format!("词根联想：{term} -> demo memory"));

    conn.execute(
        "UPDATE vocabulary SET ai_memory = ?, updated_at=CURRENT_TIMESTAMP WHERE id = ?",
        params![generated, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(generated)
}

#[tauri::command]
pub async fn regenerate_memory_for_word(
    id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    generate_memory_for_word(id, state).await
}

#[tauri::command]
pub fn read_clipboard_text() -> Result<Option<String>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    match clipboard.get_text() {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}
