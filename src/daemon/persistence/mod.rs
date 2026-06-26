use anyhow::Context;
use rusqlite::Connection;
#[cfg(test)]
use std::path::Path;

/// Current database schema version. Increment when adding/altering tables.
/// Migrations are applied sequentially from the stored version up to this number.
const CURRENT_SCHEMA_VERSION: i64 = 1;

pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite database at ~/.local/share/deskbrid/deskbrid.db.
    /// Enables WAL mode, runs schema initialization, and applies any pending migrations.
    pub fn open() -> anyhow::Result<Self> {
        let data_dir = dirs::data_dir()
            .context("could not determine XDG data directory")?
            .join("deskbrid");
        std::fs::create_dir_all(&data_dir).context("failed to create deskbrid data directory")?;
        let db_path = data_dir.join("deskbrid.db");

        let conn = Connection::open(&db_path).context("failed to open SQLite database")?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("failed to set WAL journal mode")?;

        let db = Self { conn };
        db.init_db()?;
        db.run_migrations()?;

        Ok(db)
    }

    #[cfg(test)]
    pub(crate) fn open_path(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create test database directory")?;
        }

        let conn = Connection::open(path).context("failed to open test SQLite database")?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("failed to set WAL journal mode")?;

        let db = Self { conn };
        db.init_db()?;
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database (fallback when the on-disk DB is unavailable).
    pub fn memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory database")?;
        let db = Self { conn };
        db.init_db()?;
        // In-memory DBs always start fresh — set version directly.
        db.conn
            .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
        Ok(db)
    }

    fn init_db(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                source TEXT,
                copied_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY,
                seq INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                action TEXT NOT NULL,
                params TEXT,
                status TEXT NOT NULL,
                duration_ms INTEGER,
                timestamp INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY,
                app_name TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                urgency TEXT DEFAULT 'normal',
                actions TEXT,
                timestamp INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS macros (
                name TEXT PRIMARY KEY,
                actions_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS blackboard (
                key TEXT NOT NULL,
                namespace TEXT NOT NULL DEFAULT 'default',
                value_json TEXT NOT NULL,
                ttl INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (key, namespace)
            );
            CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                data_json TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                last_active INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                trigger_json TEXT NOT NULL,
                condition_json TEXT,
                action_type TEXT NOT NULL,
                action_params TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                max_fires INTEGER,
                cooldown_ms INTEGER
            );",
        )?;
        Ok(())
    }

    /// Run any pending schema migrations from the stored version up to
    /// CURRENT_SCHEMA_VERSION. Each step is a match arm keyed by the
    /// version being migrated *from*.
    fn run_migrations(&self) -> anyhow::Result<()> {
        let stored: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .context("failed to read schema version")?;

        if stored > CURRENT_SCHEMA_VERSION {
            anyhow::bail!(
                "database schema version {stored} is newer than this binary (max {CURRENT_SCHEMA_VERSION}). \
                 Downgrade is not supported.",
            );
        }

        // Wrap all migrations in a transaction so a crash mid-migration
        // rolls back the entire batch rather than leaving the schema
        // half-migrated with user_version already incremented.
        self.conn
            .execute_batch("BEGIN EXCLUSIVE")
            .context("failed to begin migration transaction")?;

        for v in stored..CURRENT_SCHEMA_VERSION {
            match v {
                // v0 → v1: initial schema (CREATE TABLE IF NOT EXISTS handled by init_db)
                0 => { /* no DDL needed; tables created by init_db above */ }
                // Future migrations go here:
                // 1 => {
                //     self.conn.execute_batch("ALTER TABLE ... ADD COLUMN ...")?;
                // }
                other => anyhow::bail!("unknown schema version {other}"),
            }
            self.conn
                .pragma_update(None, "user_version", v + 1)
                .context(format!("failed to update schema version to {}", v + 1))?;
            tracing::info!("Migrated database schema v{v} → v{}", v + 1);
        }

        self.conn
            .execute_batch("COMMIT")
            .context("failed to commit migration transaction")?;
        Ok(())
    }
}

/// Current Unix timestamp in seconds.
pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub mod audit;
pub mod blackboard;
pub mod clipboard;
pub mod macros;
pub mod notifications;
pub mod rules;
pub mod sessions;

#[cfg(test)]
mod tests;
