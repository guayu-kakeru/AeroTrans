use rusqlite::Connection;

use aerotrans_lib::db::migrations::run_migrations;

#[test]
fn migrates_from_empty_to_latest() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();

    assert_eq!(version, 11);

    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('vocabulary') WHERE name = 'ai_memory'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(exists, 1);

    let settings_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_settings'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(settings_exists, 1);

    let provider_column: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('app_settings') WHERE name = 'translation_provider'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(provider_column, 1);

    let local_dictionary_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='local_dictionary'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(local_dictionary_exists, 1);

    let seeded_rows: i32 = conn
        .query_row("SELECT COUNT(*) FROM local_dictionary", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(seeded_rows >= 10);

    let apple_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM local_dictionary WHERE direction='en_to_zh' AND lower(term)='apple'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(apple_exists, 1);

    let provider: String = conn
        .query_row(
            "SELECT translation_provider FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(provider, "youdao");

    let api_base_url: String = conn
        .query_row(
            "SELECT api_base_url FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        api_base_url,
        "https://openrouter.ai/api/v1/chat/completions"
    );

    let memory_prompt_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('app_settings') WHERE name = 'memory_prompt'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(memory_prompt_exists, 1);

    let selection_shortcut: String = conn
        .query_row(
            "SELECT selection_shortcut FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(selection_shortcut, "Alt+Shift+L");

    let memory_prompt: String = conn
        .query_row(
            "SELECT memory_prompt FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(memory_prompt.len() > 20);
}

#[test]
fn migration_11_repairs_old_selection_shortcut_and_placeholder_prompt() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "UPDATE app_settings SET selection_shortcut = 'Ctrl+Shift+D', memory_prompt = '????broken prompt????' WHERE id = 1",
        [],
    )
    .unwrap();
    conn.execute_batch("PRAGMA user_version = 10;").unwrap();

    run_migrations(&conn).unwrap();

    let repaired_shortcut: String = conn
        .query_row(
            "SELECT selection_shortcut FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(repaired_shortcut, "Alt+Shift+L");

    let repaired_prompt: String = conn
        .query_row(
            "SELECT memory_prompt FROM app_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!repaired_prompt.contains("????"));
}
