pub mod commands;
pub mod db;
pub mod models;
pub mod security;
pub mod state;

use std::{fs, sync::Arc};

use reqwest::Client;
use security::credential_store::KeyringStore;
use state::AppState;
use tauri::Manager;

pub fn version() -> &'static str {
    "0.1.0"
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("aerotrans.sqlite");

            let state = AppState {
                db_path: db_path.to_string_lossy().to_string(),
                secrets: Arc::new(KeyringStore::new("AeroTrans")),
                http_client: Client::new(),
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::load_settings,
            commands::save_settings,
            commands::load_api_config,
            commands::save_api_config,
            commands::save_api_key,
            commands::has_api_key,
            commands::check_shortcut_conflict,
            commands::translate_text,
            commands::add_vocabulary,
            commands::list_vocabulary,
            commands::delete_vocabulary,
            commands::generate_memory_for_word,
            commands::regenerate_memory_for_word,
            commands::read_clipboard_text,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AeroTrans");
}
