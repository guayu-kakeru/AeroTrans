pub mod commands;
pub mod db;
pub mod desktop_registration;
pub mod models;
pub mod security;
pub mod state;

use std::{
    fs,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use reqwest::Client;
use rusqlite::Connection;
use security::credential_store::KeyringStore;
use state::AppState;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::db::migrations::run_migrations;

pub fn version() -> &'static str {
    "0.1.0"
}

fn bootstrap_database(db_path: &str) -> anyhow::Result<(String, String)> {
    let conn = Connection::open(db_path)?;
    run_migrations(&conn)?;

    let shortcuts = conn.query_row(
        "SELECT spotlight_shortcut, selection_shortcut FROM app_settings WHERE id = 1",
        [],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?;

    Ok(shortcuts)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    let Some(state) = app.try_state::<AppState>() else {
                        return;
                    };

                    let spotlight_id = state.spotlight_hotkey_id.lock().ok().and_then(|id| *id);
                    let selection_id = state.selection_hotkey_id.lock().ok().and_then(|id| *id);

                    if Some(shortcut.id) == spotlight_id {
                        let _ = commands::toggle_spotlight_window_inner(app);
                    } else if Some(shortcut.id) == selection_id {
                        let _ = commands::handle_selection_hotkey_inner(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("aerotrans.sqlite");

            if let Err(err) = desktop_registration::ensure_current_user_registration() {
                eprintln!("[AeroTrans] Failed to register app in Windows launcher locations: {err}");
            }

            let (spotlight_shortcut, selection_shortcut) =
                bootstrap_database(db_path.to_string_lossy().as_ref())
                    .map_err(|e| anyhow!("Database init failed: {e}"))?;

            let state = AppState {
                db_path: db_path.to_string_lossy().to_string(),
                secrets: Arc::new(KeyringStore::new("AeroTrans")),
                http_client: Client::builder()
                    .timeout(Duration::from_secs(12))
                    .build()
                    .map_err(|e| anyhow!("HTTP client init failed: {e}"))?,
                spotlight_hotkey_id: Mutex::new(None),
                selection_hotkey_id: Mutex::new(None),
                spotlight_hotkey_str: Mutex::new(None),
                selection_hotkey_str: Mutex::new(None),
            };

            let app_handle = app.handle().clone();

            if let Err(err) = commands::register_spotlight_shortcut_internal(
                &app_handle,
                &state,
                &spotlight_shortcut,
            ) {
                eprintln!("[AeroTrans] Failed to register Spotlight shortcut: {err}");
                let fallback_spotlight = "Alt+Shift+F";
                if spotlight_shortcut != fallback_spotlight {
                    match commands::register_spotlight_shortcut_internal(
                        &app_handle,
                        &state,
                        fallback_spotlight,
                    ) {
                        Ok(_) => {
                            if let Ok(conn) = Connection::open(&state.db_path) {
                                let _ = conn.execute(
                                    "UPDATE app_settings SET spotlight_shortcut=?, updated_at=CURRENT_TIMESTAMP WHERE id=1",
                                    [fallback_spotlight],
                                );
                            }
                            eprintln!(
                                "[AeroTrans] Spotlight shortcut fallback applied: {}",
                                fallback_spotlight
                            );
                        }
                        Err(fallback_err) => {
                            eprintln!(
                                "[AeroTrans] Failed to register fallback Spotlight shortcut: {}",
                                fallback_err
                            );
                        }
                    }
                }
            }
            if let Err(err) = commands::register_selection_shortcut_internal(
                &app_handle,
                &state,
                &selection_shortcut,
            ) {
                eprintln!("[AeroTrans] Failed to register selection shortcut: {err}");
            }

            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = commands::quit_app(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::health_check,
            commands::load_settings,
            commands::save_settings,
            commands::sync_spotlight_shortcut,
            commands::sync_selection_shortcut,
            commands::toggle_spotlight_window,
            commands::show_spotlight_window,
            commands::hide_spotlight_window,
            commands::quit_app,
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
