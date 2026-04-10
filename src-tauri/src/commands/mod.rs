use arboard::Clipboard;
use regex::Regex;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::db::migrations::run_migrations;
use crate::models::{
    ApiConfigDto, CollectionMode, SettingsDto, ShortcutCheckResultDto, TranslationProvider,
    TranslationRequestDto, TranslationResultDto, VocabularyDto,
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

fn normalize_translation_input(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut normalized = input.replace("\r\n", "\n");
    normalized = Regex::new(r"([A-Za-z])-\n([A-Za-z])")
        .unwrap()
        .replace_all(&normalized, "$1$2")
        .into_owned();
    normalized = Regex::new(r"\n+")
        .unwrap()
        .replace_all(&normalized, " ")
        .into_owned();
    normalized = Regex::new(r"[ \t\u{00A0}]+")
        .unwrap()
        .replace_all(&normalized, " ")
        .into_owned();
    normalized.trim().to_string()
}

fn split_glossary_terms(text: &str) -> Vec<String> {
    Regex::new(r"[A-Za-z\u4e00-\u9fff]+")
        .unwrap()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .take(3)
        .collect()
}

fn provider_to_db(provider: &TranslationProvider) -> &'static str {
    match provider {
        TranslationProvider::Google => "google",
        TranslationProvider::Youdao => "youdao",
        TranslationProvider::Mymemory => "mymemory",
        TranslationProvider::Ai => "ai",
    }
}

fn provider_from_db(raw: &str) -> TranslationProvider {
    match raw {
        "youdao" => TranslationProvider::Youdao,
        "mymemory" => TranslationProvider::Mymemory,
        "google" => TranslationProvider::Google,
        "ai" => TranslationProvider::Ai,
        "local" => TranslationProvider::Youdao,
        _ => TranslationProvider::Youdao,
    }
}

fn fallback_translate(text: &str) -> TranslationResultDto {
    let direction = detect_direction(text).to_string();
    let translation = if direction == "zh_to_en" {
        format!("Fallback translation: {text}")
    } else {
        format!("兜底翻译：{text}")
    };

    let glossary = split_glossary_terms(text);

    TranslationResultDto {
        detected_direction: direction,
        translation,
        glossary,
        phonetics: vec![],
        source: "fallback".to_string(),
    }
}

fn parse_collection_mode(raw: &str) -> CollectionMode {
    match raw {
        "silent_all" => CollectionMode::SilentAll,
        _ => CollectionMode::ManualStar,
    }
}

fn load_settings_row(conn: &Connection) -> Result<SettingsDto, String> {
    conn.query_row(
        "SELECT spotlight_shortcut, selection_shortcut, companion_enabled, companion_opacity, companion_mouse_through, tts_enabled, collection_mode, translation_provider, memory_prompt FROM app_settings WHERE id = 1",
        [],
        |row| {
            let mode_raw: String = row.get(6)?;
            let provider_raw: String = row.get(7)?;
            Ok(SettingsDto {
                spotlight_shortcut: row.get(0)?,
                selection_shortcut: row.get(1)?,
                companion_enabled: row.get::<_, i64>(2)? == 1,
                companion_opacity: row.get(3)?,
                companion_mouse_through: row.get::<_, i64>(4)? == 1,
                tts_enabled: row.get::<_, i64>(5)? == 1,
                collection_mode: parse_collection_mode(&mode_raw),
                translation_provider: provider_from_db(&provider_raw),
                memory_prompt: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

async fn call_ai_translation(
    app: &AppState,
    text: &str,
    translation_hint: Option<&str>,
    context: Option<&str>,
    for_memory: bool,
) -> Result<Option<String>, String> {
    let conn = connect_db(&app.db_path)?;
    let api = conn
        .query_row(
            "SELECT api_base_url, api_model, memory_prompt FROM app_settings WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    let Some(api_key) = app
        .secrets
        .get_secret(API_KEY_SLOT)
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    let (api_base_url, api_model, memory_prompt) = api;
    let chat_url = resolve_chat_completions_url(&api_base_url);
    if chat_url.is_empty() {
        return Ok(None);
    }
    let memory_guard = "\
你是严格格式化输出器，必须遵守以下协议。\n\
1) 只输出 4 行中文，不要任何额外说明。\n\
2) 每行不超过 32 个字。\n\
3) 不要编号，不要项目符号，不要解释方法。\n\
固定格式：\n\
第1行：{word} /音标可选/ 中文释义\n\
第2行：旧词联想：...\n\
第3行：记忆场景：...\n\
第4行：场景短句：...\n\
若不符合上述格式，输出将被程序丢弃。";

    let system = if for_memory {
        let base = if memory_prompt.trim().is_empty() {
            default_memory_prompt()
        } else {
            memory_prompt.as_str()
        };
        format!("{base}\n\n{memory_guard}")
    } else {
        "You are a bilingual translator. Return concise translation and short glossary.".to_string()
    };

    let user_prompt = if for_memory {
        let hint = translation_hint
            .map(sanitize_translation_hint)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "（未提供）".to_string());
        format!(
            "目标单词：{text}\n中文释义参考：{hint}\n请生成“能直接记住”的联想内容，不要解释原理。"
        )
    } else {
        match context {
            Some(ctx) if !ctx.trim().is_empty() => format!("Text: {text}\nContext: {ctx}"),
            _ => format!("Text: {text}"),
        }
    };

    let temperature = if for_memory { 0.0 } else { 0.2 };
    let max_tokens = if for_memory { 220 } else { 512 };

    let body = serde_json::json!({
        "model": api_model,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt}
        ]
    });

    let response = app
        .http_client
        .post(chat_url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response
            .text()
            .await
            .unwrap_or_else(|_| "<response unreadable>".to_string());
        eprintln!("[AeroTrans] AI request failed: status={status}, detail={detail}");
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

fn resolve_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("/chat/completions") {
        return trimmed.to_string();
    }

    format!("{trimmed}/chat/completions")
}

fn is_local_api_base_url(base_url: &str) -> bool {
    let normalized = base_url.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }

    let local_keywords = ["localhost", "127.0.0.1", "0.0.0.0", "::1", "ollama"];
    local_keywords
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

fn now_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn default_memory_prompt() -> &'static str {
    "你是专业单词记忆导师，只用联想记忆法 + 简单旧词拆分法帮我高效记单词，拒绝枯燥死记，每条都给具体场景和记忆方式，清晰好记、一看就会。请按以下规则输出：把陌生单词拆成熟悉旧单词 / 拼音 / 音节，不造复杂结构；搭配生活化、夸张搞笑的场景联想，画面感极强；标注中文释义 + 记忆逻辑 + 场景短句，不空谈方法，直接给可记内容；单词长度不限，优先高频词汇，记忆方式通俗不绕弯，看完就能立刻记住。现在请你针对我给出的单词，生成专属记忆方案。"
}

fn sanitize_translation_hint(hint: &str) -> String {
    let compact = hint
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('；', ";")
        .replace('，', ",");
    let part = compact
        .split(&[';', ',', '。', '.'][..])
        .map(str::trim)
        .find(|item| !item.is_empty())
        .unwrap_or("");
    let mut result = part.to_string();
    if result.chars().count() > 18 {
        result = result.chars().take(18).collect::<String>();
    }
    result
}

fn split_memory_chunks(term: &str) -> (String, String) {
    let compact = term.trim().to_ascii_lowercase();
    if compact.chars().count() <= 3 {
        return (compact.clone(), "音".to_string());
    }

    let chars: Vec<char> = compact.chars().collect();
    let cut = (chars.len() / 2).max(1).min(chars.len() - 1);
    let left: String = chars[..cut].iter().collect();
    let right: String = chars[cut..].iter().collect();
    (left, right)
}

fn build_local_memory_tip(term: &str, translation: &str, seed: u64) -> String {
    let meaning = sanitize_translation_hint(translation);
    let meaning = if meaning.is_empty() {
        translation.trim().to_string()
    } else {
        meaning
    };
    let (left, right) = split_memory_chunks(term);
    let scenes = [
        format!("地铁上看见 {term}，立刻想到“{meaning}”。"),
        format!("做题遇到 {term}，你秒答“{meaning}”。"),
        format!("读文章划到 {term}，脑中蹦出“{meaning}”。"),
    ];
    let short_lines = [
        format!("I see {term}, I think {meaning}."),
        format!("When {term} appears, it means {meaning}."),
        format!("{term} pops up, {meaning} pops up."),
    ];
    let idx = (seed as usize) % scenes.len();

    format!(
        "{term} /{term}/ {meaning}\n旧词联想：{left} + {right}\n记忆场景：{scene}\n场景短句：{line}",
        scene = scenes[idx],
        line = short_lines[idx]
    )
}

fn normalize_memory_output(raw: &str, term: &str, translation: &str, seed: u64) -> String {
    let lines: Vec<String> = raw
        .replace('\r', "")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('-').trim().to_string())
        .collect();

    let compact = lines.join("\n");
    let has_link = compact.contains("联想");
    let has_scene = compact.contains("场景");
    let too_long = compact.chars().count() > 260;
    let talks_about_method = compact.contains("记忆方法") || compact.contains("学习建议");

    if compact.is_empty() || too_long || talks_about_method || !has_link || !has_scene {
        return build_local_memory_tip(term, translation, seed);
    }

    lines.into_iter().take(4).collect::<Vec<_>>().join("\n")
}

async fn build_memory_content(
    state: &AppState,
    term: &str,
    translation: &str,
    seed: u64,
) -> Result<String, String> {
    let conn = connect_db(&state.db_path)?;
    let (api_base_url, api_model): (String, String) = conn
        .query_row(
            "SELECT api_base_url, api_model FROM app_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    if is_local_api_base_url(&api_base_url) || api_model.trim().to_ascii_lowercase().contains("ollama") {
        return Err("检测到本地模型接口。联想记忆仅支持已配置 API Key 的云端 OpenAI 兼容模型。".to_string());
    }

    let has_api_key = state
        .secrets
        .get_secret(API_KEY_SLOT)
        .map_err(|e| e.to_string())?
        .is_some();
    if !has_api_key {
        return Err("未检测到 API Key。请先在设置中配置 API Key 后再生成联想记忆。".to_string());
    }

    if let Some(ai_text) = call_ai_translation(state, term, Some(translation), None, true).await? {
        return Ok(normalize_memory_output(&ai_text, term, translation, seed));
    }

    Err("AI 联想记忆生成失败，请检查 API 配置或网络连接。".to_string())
}

fn maybe_silent_collect(
    state: &AppState,
    settings: &SettingsDto,
    term: &str,
    context: Option<&str>,
    result: &TranslationResultDto,
) -> Result<(), String> {
    if !matches!(settings.collection_mode, CollectionMode::SilentAll) {
        return Ok(());
    }

    let conn = connect_db(&state.db_path)?;
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM vocabulary WHERE lower(term)=lower(?) AND translation=?",
            params![term.trim(), result.translation.trim()],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO vocabulary (term, translation, context_text, starred, ai_memory, updated_at) VALUES (?, ?, ?, 0, NULL, CURRENT_TIMESTAMP)",
        params![term.trim(), result.translation.trim(), context],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn call_google_translate(
    app: &AppState,
    text: &str,
    direction: &str,
) -> Result<Option<String>, String> {
    let target = if direction == "zh_to_en" {
        "en"
    } else {
        "zh-CN"
    };

    let response = app
        .http_client
        .get("https://translate.googleapis.com/translate_a/single")
        .header("User-Agent", "AeroTrans/0.1")
        .query(&[
            ("client", "gtx"),
            ("sl", "auto"),
            ("tl", target),
            ("dt", "t"),
            ("q", text),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    let segments = value.get(0).and_then(Value::as_array);
    let Some(segments) = segments else {
        return Ok(None);
    };

    let translation = segments
        .iter()
        .filter_map(|segment| segment.as_array())
        .filter_map(|part| part.first().and_then(Value::as_str))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<String>();

    if translation.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(translation))
    }
}

async fn call_public_translate(
    app: &AppState,
    text: &str,
    direction: &str,
) -> Result<Option<String>, String> {
    let target = if direction == "zh_to_en" {
        "en"
    } else {
        "zh-CN"
    };
    let langpair = format!("auto|{target}");

    let response = app
        .http_client
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text), ("langpair", langpair.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    let translation = value["responseData"]["translatedText"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok(translation)
}

fn cleanup_youdao_translation(raw: &str) -> String {
    let cleaned = Regex::new(r"@\{[^}]+\}")
        .unwrap()
        .replace_all(raw, "")
        .into_owned();
    cleaned
        .replace('；', "; ")
        .replace('\n', " ")
        .trim()
        .trim_matches(';')
        .trim()
        .to_string()
}

fn parse_youdao_translation(value: &Value, direction: &str) -> Option<(String, Vec<String>)> {
    if direction == "en_to_zh" {
        let word = value
            .get("ec")
            .and_then(|v| v.get("word"))
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())?;

        let mut phonetics: Vec<String> = Vec::new();
        if let Some(us) = word.get("usphone").and_then(Value::as_str) {
            if !us.trim().is_empty() {
                phonetics.push(format!("US /{}/", us.trim()));
            }
        }
        if let Some(uk) = word.get("ukphone").and_then(Value::as_str) {
            if !uk.trim().is_empty() {
                phonetics.push(format!("UK /{}/", uk.trim()));
            }
        }

        let translation = word
            .get("trs")
            .and_then(Value::as_array)
            .and_then(|trs| trs.first())
            .and_then(|tr| tr.get("tr"))
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("l"))
            .and_then(|l| l.get("i"))
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(Value::as_str)
            .map(cleanup_youdao_translation)?;

        return Some((translation, phonetics));
    }

    let word = value
        .get("ce_new")
        .and_then(|v| v.get("word"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())?;

    let mut phonetics: Vec<String> = Vec::new();
    if let Some(phone) = word.get("phone").and_then(Value::as_str) {
        if !phone.trim().is_empty() {
            phonetics.push(phone.trim().to_string());
        }
    }

    let translation = word
        .get("trs")
        .and_then(Value::as_array)
        .and_then(|trs| trs.first())
        .and_then(|tr| tr.get("tr"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("l"))
        .and_then(|l| l.get("i"))
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .map(cleanup_youdao_translation)?;

    Some((translation, phonetics))
}

async fn call_youdao_translate(
    app: &AppState,
    text: &str,
    direction: &str,
) -> Result<Option<(String, Vec<String>)>, String> {
    let response = app
        .http_client
        .get("https://dict.youdao.com/jsonapi")
        .query(&[("q", text)])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(parse_youdao_translation(&value, direction))
}

async fn lookup_youdao_phonetics(app: &AppState, text: &str, direction: &str) -> Vec<String> {
    if direction != "en_to_zh" {
        return vec![];
    }

    match call_youdao_translate(app, text, direction).await {
        Ok(Some((_, phonetics))) => phonetics,
        _ => vec![],
    }
}

fn build_translation_result(
    direction: &str,
    text: &str,
    translation: String,
    source: &str,
    glossary: Vec<String>,
    phonetics: Vec<String>,
) -> TranslationResultDto {
    let normalized_glossary = if glossary.is_empty() {
        split_glossary_terms(text)
    } else {
        glossary
    };

    TranslationResultDto {
        detected_direction: direction.to_string(),
        translation,
        glossary: normalized_glossary,
        phonetics,
        source: source.to_string(),
    }
}

fn parse_shortcut(shortcut: &str, label: &str) -> Result<Shortcut, String> {
    let parse_result: Result<Shortcut, _> = shortcut.trim().to_string().try_into();
    parse_result.map_err(|_| {
        if label.trim().is_empty() {
            "快捷键格式无效".to_string()
        } else {
            format!("{label} 快捷键格式无效")
        }
    })
}

fn register_shortcut_internal(
    app: &tauri::AppHandle,
    shortcut: &str,
    shortcut_label: &str,
    id_slot: &std::sync::Mutex<Option<u32>>,
    str_slot: &std::sync::Mutex<Option<String>>,
) -> Result<(), String> {
    let shortcut_trimmed = shortcut.trim();
    let parsed = parse_shortcut(shortcut_trimmed, shortcut_label)?;
    let manager = app.global_shortcut();

    let previous = str_slot
        .lock()
        .map_err(|_| "Shortcut state lock unavailable".to_string())?
        .clone();

    if let Some(previous_shortcut) = previous {
        if previous_shortcut.eq_ignore_ascii_case(shortcut_trimmed)
            && manager.is_registered(parsed.clone())
        {
            *id_slot
                .lock()
                .map_err(|_| "Shortcut state lock unavailable".to_string())? = Some(parsed.id);
            return Ok(());
        }

        if let Ok(previous_parsed) = parse_shortcut(&previous_shortcut, shortcut_label) {
            let _ = manager.unregister(previous_parsed);
        }
    }

    manager
        .register(parsed.clone())
        .map_err(|e| format!("Failed to register {shortcut_label} shortcut: {e}"))?;

    *id_slot
        .lock()
        .map_err(|_| "Shortcut state lock unavailable".to_string())? = Some(parsed.id);
    *str_slot
        .lock()
        .map_err(|_| "Shortcut state lock unavailable".to_string())? =
        Some(shortcut_trimmed.to_string());

    Ok(())
}

pub fn register_spotlight_shortcut_internal(
    app: &tauri::AppHandle,
    state: &AppState,
    shortcut: &str,
) -> Result<(), String> {
    register_shortcut_internal(
        app,
        shortcut,
        "Spotlight",
        &state.spotlight_hotkey_id,
        &state.spotlight_hotkey_str,
    )
}

pub fn register_selection_shortcut_internal(
    app: &tauri::AppHandle,
    state: &AppState,
    shortcut: &str,
) -> Result<(), String> {
    register_shortcut_internal(
        app,
        shortcut,
        "划词翻译",
        &state.selection_hotkey_id,
        &state.selection_hotkey_str,
    )
}

pub fn toggle_spotlight_window_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "未找到 Spotlight 窗口".to_string())?;

    let visible = window.is_visible().map_err(|e| e.to_string())?;

    if visible {
        window.hide().map_err(|e| e.to_string())?;
    } else {
        let _ = window.center();
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        let _ = window.emit("spotlight_shown", ());
    }

    Ok(())
}

pub fn show_spotlight_window_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "未找到 Spotlight 窗口".to_string())?;

    let visible = window.is_visible().map_err(|e| e.to_string())?;
    if !visible {
        let _ = window.center();
        window.show().map_err(|e| e.to_string())?;
    }
    window.set_focus().map_err(|e| e.to_string())?;
    let _ = window.emit("spotlight_shown", ());
    Ok(())
}

pub fn hide_spotlight_window_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("spotlight")
        .ok_or_else(|| "未找到 Spotlight 窗口".to_string())?;
    window.hide().map_err(|e| e.to_string())
}

pub fn show_main_window_inner(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "未找到主窗口".to_string())?;

    if window.is_minimized().map_err(|e| e.to_string())? {
        window.unminimize().map_err(|e| e.to_string())?;
    }

    if !window.is_visible().map_err(|e| e.to_string())? {
        window.show().map_err(|e| e.to_string())?;
    }

    window.set_focus().map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn trigger_copy_shortcut_once() {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYEVENTF_KEYUP, VK_CONTROL,
    };

    unsafe {
        keybd_event(VK_CONTROL as u8, 0, 0, 0);
        keybd_event(b'C', 0, 0, 0);
        keybd_event(b'C', 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

#[cfg(not(target_os = "windows"))]
fn trigger_copy_shortcut_once() {}

fn capture_selected_text_after_copy() -> Result<String, String> {
    let before = read_clipboard_text()?.unwrap_or_default();
    let before_trimmed = before.trim().to_string();
    let mut latest_non_empty = String::new();

    for _ in 0..8 {
        thread::sleep(Duration::from_millis(45));
        let current = read_clipboard_text()?.unwrap_or_default();
        let trimmed = current.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed != before_trimmed {
            return Ok(trimmed);
        }

        latest_non_empty = trimmed;
    }

    if latest_non_empty.is_empty() {
        return Ok(String::new());
    }

    let word_count = latest_non_empty.split_whitespace().count();
    if latest_non_empty == before_trimmed && (latest_non_empty.chars().count() > 120 || word_count > 18) {
        return Ok(String::new());
    }

    Ok(latest_non_empty)
}

pub fn handle_selection_hotkey_inner(app: &tauri::AppHandle) -> Result<(), String> {
    trigger_copy_shortcut_once();
    let payload = capture_selected_text_after_copy()?;
    show_main_window_inner(app)?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "未找到主窗口".to_string())?;
    window
        .emit("selection_translate_triggered", payload)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn health_check() -> String {
    "AeroTrans backend ready".to_string()
}

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> Result<SettingsDto, String> {
    let conn = connect_db(&state.db_path)?;
    load_settings_row(&conn)
}

#[tauri::command]
pub fn save_settings(
    settings: SettingsDto,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let conn = connect_db(&state.db_path)?;
    let mode = match settings.collection_mode {
        CollectionMode::SilentAll => "silent_all",
        CollectionMode::ManualStar => "manual_star",
    };

    conn.execute(
        "UPDATE app_settings SET spotlight_shortcut=?, selection_shortcut=?, companion_enabled=?, companion_opacity=?, companion_mouse_through=?, tts_enabled=?, collection_mode=?, translation_provider=?, memory_prompt=?, updated_at=CURRENT_TIMESTAMP WHERE id = 1",
        params![
            settings.spotlight_shortcut,
            settings.selection_shortcut,
            if settings.companion_enabled { 1 } else { 0 },
            settings.companion_opacity,
            if settings.companion_mouse_through { 1 } else { 0 },
            if settings.tts_enabled { 1 } else { 0 },
            mode,
            provider_to_db(&settings.translation_provider),
            settings.memory_prompt,
        ],
    )
    .map_err(|e| e.to_string())?;

    register_spotlight_shortcut_internal(&app, &state, &settings.spotlight_shortcut)?;
    register_selection_shortcut_internal(&app, &state, &settings.selection_shortcut)?;

    Ok(())
}

#[tauri::command]
pub fn sync_spotlight_shortcut(
    shortcut: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    register_spotlight_shortcut_internal(&app, &state, &shortcut)
}

#[tauri::command]
pub fn sync_selection_shortcut(
    shortcut: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    register_selection_shortcut_internal(&app, &state, &shortcut)
}

#[tauri::command]
pub fn toggle_spotlight_window(app: tauri::AppHandle) -> Result<(), String> {
    toggle_spotlight_window_inner(&app)
}

#[tauri::command]
pub fn show_spotlight_window(app: tauri::AppHandle) -> Result<(), String> {
    show_spotlight_window_inner(&app)
}

#[tauri::command]
pub fn hide_spotlight_window(app: tauri::AppHandle) -> Result<(), String> {
    hide_spotlight_window_inner(&app)
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    let _ = hide_spotlight_window_inner(&app);
    app.exit(0);
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
pub fn check_shortcut_conflict(
    shortcut: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> ShortcutCheckResultDto {
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

    let parsed = parse_shortcut(&shortcut, "");
    let Ok(parsed_shortcut) = parsed else {
        return ShortcutCheckResultDto {
            conflict: true,
            reason: Some("快捷键格式无效".to_string()),
        };
    };

    let trimmed = shortcut.trim().to_string();
    let current_spotlight = state
        .spotlight_hotkey_str
        .lock()
        .ok()
        .and_then(|value| value.clone());
    if current_spotlight
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case(trimmed.as_str()))
        .unwrap_or(false)
    {
        return ShortcutCheckResultDto {
            conflict: false,
            reason: None,
        };
    }

    let current_selection = state
        .selection_hotkey_str
        .lock()
        .ok()
        .and_then(|value| value.clone());
    if current_selection
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case(trimmed.as_str()))
        .unwrap_or(false)
    {
        return ShortcutCheckResultDto {
            conflict: false,
            reason: None,
        };
    }

    let manager = app.global_shortcut();

    match manager.register(parsed_shortcut.clone()) {
        Ok(_) => {
            let _ = manager.unregister(parsed_shortcut);
            ShortcutCheckResultDto {
                conflict: false,
                reason: None,
            }
        }
        Err(err) => ShortcutCheckResultDto {
            conflict: true,
            reason: Some(format!("系统级冲突：{err}")),
        },
    }
}

#[tauri::command]
pub async fn translate_text(
    request: TranslationRequestDto,
    state: State<'_, AppState>,
) -> Result<TranslationResultDto, String> {
    let settings = {
        let conn = connect_db(&state.db_path)?;
        load_settings_row(&conn)?
    };
    let provider = request
        .provider
        .clone()
        .unwrap_or(settings.translation_provider.clone());
    let normalized_text = normalize_translation_input(&request.text);
    if normalized_text.is_empty() {
        return Err("请输入要翻译的内容".to_string());
    }
    let normalized_context = request.context.as_deref().map(normalize_translation_input);
    let direction = detect_direction(&normalized_text).to_string();
    let collect_and_return =
        |result: TranslationResultDto| -> Result<TranslationResultDto, String> {
            let _ = maybe_silent_collect(
                &state,
                &settings,
                &normalized_text,
                normalized_context.as_deref(),
                &result,
            );
            Ok(result)
        };

    match provider {
        TranslationProvider::Google => {
            if let Some(google_text) =
                call_google_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    google_text,
                    "google",
                    vec![],
                    phonetics,
                ));
            }
            if let Some((youdao_text, phonetics)) =
                call_youdao_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    youdao_text,
                    "youdao",
                    vec![],
                    phonetics,
                ));
            }
            if let Some(public_text) =
                call_public_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    public_text,
                    "mymemory",
                    vec![],
                    phonetics,
                ));
            }

            collect_and_return(fallback_translate(&normalized_text))
        }
        TranslationProvider::Youdao => {
            if let Some((translation, phonetics)) =
                call_youdao_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    translation,
                    "youdao",
                    vec![],
                    phonetics,
                ));
            }

            if let Some(google_text) =
                call_google_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    google_text,
                    "google",
                    vec![],
                    vec![],
                ));
            }

            if let Some(public_text) =
                call_public_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    public_text,
                    "mymemory",
                    vec![],
                    vec![],
                ));
            }

            collect_and_return(fallback_translate(&normalized_text))
        }
        TranslationProvider::Mymemory => {
            if let Some(public_text) =
                call_public_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    public_text,
                    "mymemory",
                    vec![],
                    phonetics,
                ));
            }

            if let Some(google_text) =
                call_google_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    google_text,
                    "google",
                    vec![],
                    phonetics,
                ));
            }

            if let Some((translation, phonetics)) =
                call_youdao_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    translation,
                    "youdao",
                    vec![],
                    phonetics,
                ));
            }

            collect_and_return(fallback_translate(&normalized_text))
        }
        TranslationProvider::Ai => {
            if let Some(ai_text) = call_ai_translation(
                &state,
                &normalized_text,
                None,
                normalized_context.as_deref(),
                false,
            )
            .await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    ai_text,
                    "ai",
                    vec![],
                    phonetics,
                ));
            }

            if let Some(google_text) =
                call_google_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    google_text,
                    "google",
                    vec![],
                    phonetics,
                ));
            }

            if let Some(public_text) =
                call_public_translate(&state, &normalized_text, &direction).await?
            {
                let phonetics = lookup_youdao_phonetics(&state, &normalized_text, &direction).await;
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    public_text,
                    "mymemory",
                    vec![],
                    phonetics,
                ));
            }

            if let Some((translation, phonetics)) =
                call_youdao_translate(&state, &normalized_text, &direction).await?
            {
                return collect_and_return(build_translation_result(
                    &direction,
                    &normalized_text,
                    translation,
                    "youdao",
                    vec![],
                    phonetics,
                ));
            }

            collect_and_return(fallback_translate(&normalized_text))
        }
    }
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

    let result = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
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
pub async fn generate_memory_for_word(
    id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let conn = connect_db(&state.db_path)?;
    let (term, translation): (String, String) = conn
        .query_row(
            "SELECT term, translation FROM vocabulary WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let generated = build_memory_content(&state, &term, &translation, now_seed()).await?;

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
    let conn = connect_db(&state.db_path)?;
    let (term, translation): (String, String) = conn
        .query_row(
            "SELECT term, translation FROM vocabulary WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let generated =
        build_memory_content(&state, &term, &translation, now_seed().wrapping_add(97)).await?;

    conn.execute(
        "UPDATE vocabulary SET ai_memory = ?, updated_at=CURRENT_TIMESTAMP WHERE id = ?",
        params![generated, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(generated)
}

#[tauri::command]
pub fn read_clipboard_text() -> Result<Option<String>, String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    match clipboard.get_text() {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_translation_input_merges_wrapped_lines() {
        let raw = "This is a trans-\nlation demo.\r\nAnd line two.";
        let normalized = normalize_translation_input(raw);
        assert_eq!(normalized, "This is a translation demo. And line two.");
    }

    #[test]
    fn normalize_translation_input_trims_and_compacts_spaces() {
        let raw = "   hello   \n\n   world   ";
        let normalized = normalize_translation_input(raw);
        assert_eq!(normalized, "hello world");
    }

    #[test]
    fn split_glossary_terms_removes_punct_and_limits_count() {
        let terms = split_glossary_terms("Hello, robust-world! test-case latency.");
        assert_eq!(terms, vec!["Hello", "robust", "world"]);
    }

    #[test]
    fn resolve_chat_url_appends_path_when_missing() {
        let url = resolve_chat_completions_url("https://open.bigmodel.cn/api/paas/v4/");
        assert_eq!(url, "https://open.bigmodel.cn/api/paas/v4/chat/completions");
    }

    #[test]
    fn resolve_chat_url_keeps_existing_endpoint() {
        let url = resolve_chat_completions_url("https://openrouter.ai/api/v1/chat/completions");
        assert_eq!(url, "https://openrouter.ai/api/v1/chat/completions");
    }

    #[test]
    fn resolve_chat_url_handles_empty() {
        let url = resolve_chat_completions_url("   ");
        assert_eq!(url, "");
    }

    #[test]
    fn local_api_base_url_detects_local_hosts_and_ollama() {
        assert!(is_local_api_base_url("http://127.0.0.1:11434/v1/chat/completions"));
        assert!(is_local_api_base_url("http://localhost:11434/v1/chat/completions"));
        assert!(is_local_api_base_url("http://0.0.0.0:8080/v1"));
        assert!(is_local_api_base_url("http://[::1]:11434/v1"));
        assert!(is_local_api_base_url("https://ollama.local/v1/chat/completions"));
    }

    #[test]
    fn local_api_base_url_keeps_cloud_endpoints_available() {
        assert!(!is_local_api_base_url("https://openrouter.ai/api/v1/chat/completions"));
        assert!(!is_local_api_base_url("https://open.bigmodel.cn/api/paas/v4/"));
    }
}
