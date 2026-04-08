use rusqlite::Connection;

use aerotrans_lib::db::migrations::run_migrations;

#[test]
fn migrates_from_empty_to_latest() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();

    assert_eq!(version, 2);

    let exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('vocabulary') WHERE name = 'ai_memory'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(exists, 1);
}
