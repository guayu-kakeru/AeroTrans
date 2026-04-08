use rusqlite::{Connection, Result};

pub struct Migration {
    pub version: i32,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../../migrations/0002_vocab_ai_memory.sql"),
    },
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch("BEGIN;")?;
            if let Err(err) = conn.execute_batch(migration.sql) {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(err);
            }
            conn.execute_batch(&format!("PRAGMA user_version = {};", migration.version))?;
            conn.execute_batch("COMMIT;")?;
        }
    }

    Ok(())
}
