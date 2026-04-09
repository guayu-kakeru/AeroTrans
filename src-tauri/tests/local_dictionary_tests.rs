use rusqlite::Connection;

use aerotrans_lib::db::migrations::run_migrations;

#[test]
fn seeded_local_dictionary_contains_core_entries() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let translation: String = conn
        .query_row(
            "SELECT translation FROM local_dictionary WHERE direction = 'en_to_zh' AND lower(term) = 'hello'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(!translation.trim().is_empty());
}

#[test]
fn local_dictionary_patch_contains_apple() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let translation: String = conn
        .query_row(
            "SELECT translation FROM local_dictionary WHERE direction = 'en_to_zh' AND lower(term) = 'apple'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(!translation.trim().is_empty());
}
