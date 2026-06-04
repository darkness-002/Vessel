use std::fs;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};
use tauri::Manager;
use crate::state::VesselNotification;

pub fn notification_db_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("notifications.sqlite")
}

pub fn sessions_root_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("sessions")
}

fn get_or_create_encryption_key() -> Result<String, String> {
    use keyring::Entry;
    use rand::RngCore;

    let entry = Entry::new("com.vessel.app", "master_key")
        .map_err(|e| format!("failed to initialize keyring entry: {}", e))?;

    match entry.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => {
            let mut key_bytes = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key_bytes);
            
            use std::fmt::Write;
            let mut hex_key = String::with_capacity(64);
            for &b in &key_bytes {
                write!(&mut hex_key, "{:02x}", b).unwrap();
            }

            entry.set_password(&hex_key)
                .map_err(|e| format!("failed to save key to keyring: {}", e))?;
            Ok(hex_key)
        }
        Err(e) => Err(format!("failed to retrieve key from keyring: {}", e)),
    }
}

pub fn open_db(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("failed to open notification db: {e}"))?;
    let key = get_or_create_encryption_key()?;
    conn.pragma_update(None, "key", &key)
        .map_err(|e| format!("failed to encrypt database: {e}"))?;

    if let Err(e) = conn.query_row("SELECT 1", [], |_| Ok(())) {
        return Err(format!("file is not a database: {e}"));
    }

    Ok(conn)
}

pub fn ensure_db(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create db directory: {e}"))?;
    }

    let conn = match open_db(path) {
        Ok(c) => c,
        Err(e) if e.to_lowercase().contains("file is not a database") => {
            eprintln!("Notification database invalid or key mismatch. Recreating...");
            let _ = fs::remove_file(path);
            open_db(path).map_err(|e| format!("failed to open database after recreation: {e}"))?
        }
        Err(e) => return Err(e),
    };

    if let Err(e) = conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            app_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            time TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_notifications_created_at
        ON notifications(created_at DESC);
        ",
    ) {
        let err_msg = e.to_string();
        if err_msg.to_lowercase().contains("file is not a database") {
            eprintln!("Notification database invalid during schema init. Recreating...");
            let _ = fs::remove_file(path);
            let conn = open_db(path).map_err(|e| format!("failed to open database after second recreation: {e}"))?;
            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS notifications (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    app_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    body TEXT NOT NULL,
                    time TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_notifications_created_at
                ON notifications(created_at DESC);
                ",
            ).map_err(|e| format!("failed to initialize notification db schema after recreation: {e}"))?;
        } else {
            return Err(format!("failed to initialize notification db schema: {e}"));
        }
    }

    Ok(())
}

pub fn insert_notification(path: &Path, note: &VesselNotification) -> Result<(), String> {
    let conn = open_db(path)?;
    conn.execute(
        "
        INSERT INTO notifications (app_id, title, body, time, created_at)
        VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))
        ",
        params![note.app_id, note.title, note.body, note.time],
    )
    .map_err(|e| format!("failed to insert notification: {e}"))?;

    conn.execute(
        "
        DELETE FROM notifications
        WHERE id NOT IN (
            SELECT id FROM notifications
            ORDER BY created_at DESC, id DESC
            LIMIT 500
        )
        ",
        [],
    )
    .map_err(|e| format!("failed to trim notification history: {e}"))?;

    Ok(())
}

pub fn list_notifications(path: &Path, limit: u32) -> Result<Vec<VesselNotification>, String> {
    let conn = open_db(path)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT app_id, title, body, time
            FROM notifications
            ORDER BY created_at DESC, id DESC
            LIMIT ?1
            ",
        )
        .map_err(|e| format!("failed to query notifications: {e}"))?;

    let mut rows = stmt
        .query(params![limit])
        .map_err(|e| format!("failed to read notifications: {e}"))?;

    let mut result = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("failed to iterate notifications: {e}"))?
    {
        result.push(VesselNotification {
            app_id: row.get::<_, String>(0).unwrap_or_default(),
            title: row.get::<_, String>(1).unwrap_or_default(),
            body: row.get::<_, String>(2).unwrap_or_default(),
            time: row.get::<_, String>(3).unwrap_or_default(),
        });
    }

    Ok(result)
}

pub fn clear_notifications_db(path: &Path) -> Result<(), String> {
    let conn = open_db(path)?;
    conn.execute("DELETE FROM notifications", [])
        .map_err(|e| format!("failed to clear notifications: {e}"))?;
    Ok(())
}

pub fn get_encryption_key() -> Result<String, String> {
    get_or_create_encryption_key()
}
