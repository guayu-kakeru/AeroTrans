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
    Migration {
        version: 3,
        sql: include_str!("../../migrations/0003_settings.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("../../migrations/0004_translation_provider.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("../../migrations/0005_local_dictionary.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("../../migrations/0006_local_dictionary_patch.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("../../migrations/0007_provider_default_google.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("../../migrations/0008_hotkey_provider_defaults.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("../../migrations/0009_default_ai_provider.sql"),
    },
    Migration {
        version: 10,
        sql: include_str!("../../migrations/0010_memory_prompt.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("../../migrations/0011_selection_shortcut_and_prompt_fix.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("../../migrations/0012_pollinations_free_default.sql"),
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
