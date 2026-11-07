use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use rusqlite::Connection;

use crate::error::AppError;

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/0001_init.sql"))];
const READERS: usize = 4;

/// One serialized writer plus a small pool of read connections.
pub struct Db {
    writer: Mutex<Connection>,
    readers: Vec<Mutex<Connection>>,
    next_reader: AtomicUsize,
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Arc<Self>> {
        let mut writer = connect(path)?;
        migrate(&mut writer)?;
        let readers = (0..READERS)
            .map(|_| connect(path).map(Mutex::new))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Arc::new(Self {
            writer: Mutex::new(writer),
            readers,
            next_reader: AtomicUsize::new(0),
        }))
    }

    /// Runs `f` on the writer connection. All writes and DDL go through here.
    pub async fn write<T, F>(self: &Arc<Self>, f: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T, AppError> + Send + 'static,
    {
        let db = Arc::clone(self);
        run_blocking(move || {
            let mut conn = db.writer.lock().map_err(|_| poisoned())?;
            f(&mut conn)
        })
        .await
    }

    /// Runs `f` on a pooled read connection.
    pub async fn read<T, F>(self: &Arc<Self>, f: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, AppError> + Send + 'static,
    {
        let db = Arc::clone(self);
        run_blocking(move || {
            let i = db.next_reader.fetch_add(1, Ordering::Relaxed) % db.readers.len();
            let conn = db.readers[i].lock().map_err(|_| poisoned())?;
            f(&conn)
        })
        .await
    }
}

async fn run_blocking<T, F>(f: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

fn poisoned() -> AppError {
    AppError::Internal("database lock poisoned".into())
}

fn connect(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(conn)
}

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        )",
    )?;
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute("INSERT INTO schema_migrations (version) VALUES (?1)", [version])?;
        tx.commit()?;
        tracing::info!(version, "applied migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrates_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let db = Db::open(&path).unwrap();
        drop(db);
        let db = Db::open(&path).unwrap();

        let (tables, version): (i64, i64) = db
            .read(|c| {
                let tables = c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name IN ('users','tables','fields','views')",
                    [],
                    |r| r.get(0),
                )?;
                let version = c.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0))?;
                Ok((tables, version))
            })
            .await
            .unwrap();
        assert_eq!(tables, 4);
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn uses_wal_and_enforces_foreign_keys() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("test.db")).unwrap();

        let mode: String = db
            .read(|c| Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(mode, "wal");

        let err = db
            .write(|c| {
                c.execute(
                    "INSERT INTO sessions (id, user_id, expires_at, created_at) VALUES ('s', 'missing', 'x', 'x')",
                    [],
                )?;
                Ok(())
            })
            .await;
        assert!(err.is_err());
    }
}
