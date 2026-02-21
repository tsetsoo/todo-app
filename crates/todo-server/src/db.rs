use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool(db_path: &str) -> DbPool {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("Failed to create database directory");
        }
    }

    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder()
        .max_size(8)
        .build(manager)
        .expect("Failed to create connection pool");

    // Enable WAL mode and create schema
    let conn = pool.get().expect("Failed to get connection from pool");

    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .expect("Failed to enable WAL mode");

    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .expect("Failed to enable foreign keys");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS todos (
            id         TEXT PRIMARY KEY,
            section    TEXT NOT NULL CHECK(section IN ('Sp','I','Si','P')),
            title      TEXT NOT NULL,
            completed  INTEGER NOT NULL DEFAULT 0,
            importance TEXT NOT NULL DEFAULT 'medium' CHECK(importance IN ('low','medium','high','critical')),
            due_date   TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        params![],
    )
    .expect("Failed to create todos table");

    // Migrate: add columns if they don't exist (for existing databases)
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(todos)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if !cols.iter().any(|c| c == "importance") {
        conn.execute("ALTER TABLE todos ADD COLUMN importance TEXT NOT NULL DEFAULT 'medium'", [])
            .expect("Failed to add importance column");
    }
    if !cols.iter().any(|c| c == "due_date") {
        conn.execute("ALTER TABLE todos ADD COLUMN due_date TEXT", [])
            .expect("Failed to add due_date column");
    }

    pool
}
