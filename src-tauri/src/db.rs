use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

pub fn get_db_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "agneax", "store")
        .expect("Failed to get project directories");
    let data_dir = proj_dirs.data_dir();
    fs::create_dir_all(data_dir).expect("Failed to create app data directory");
    data_dir.join("agneax_store.db")
}

pub fn init_db() -> Connection {
    let db_path = get_db_path();
    let conn = Connection::open(db_path).expect("Failed to open SQLite database");
    
    // Enable WAL mode for better concurrency in SQLite
    let _ = conn.execute("PRAGMA journal_mode=WAL;", []);
    
    // Run migrations to create tables
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS cached_apps (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            version TEXT NOT NULL,
            developer TEXT NOT NULL,
            website TEXT NOT NULL,
            github TEXT NOT NULL,
            license TEXT NOT NULL,
            category TEXT NOT NULL,
            icon_url TEXT NOT NULL,
            screenshots TEXT NOT NULL,       -- JSON array (string representation)
            download_size TEXT NOT NULL,
            installed_size TEXT NOT NULL,
            dependencies TEXT NOT NULL,       -- JSON array (string representation)
            min_os TEXT NOT NULL,             -- JSON object (string representation)
            supported_architectures TEXT,    -- JSON array (string representation)
            change_logs TEXT,
            platform_data TEXT NOT NULL,      -- JSON object (string representation)
            last_synced INTEGER NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS installed_apps (
            app_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            install_date INTEGER NOT NULL,
            install_method TEXT NOT NULL,     -- winget, flatpak, choco, direct, appimage, etc.
            executable_path TEXT,
            shortcut_path TEXT
        );
        
        CREATE TABLE IF NOT EXISTS downloads (
            id TEXT PRIMARY KEY,              -- UUID / Random String ID
            app_id TEXT NOT NULL,
            app_name TEXT NOT NULL,
            version TEXT NOT NULL,
            status TEXT NOT NULL,             -- Pending, Downloading, Paused, Extracting, Installing, Completed, Failed, Cancelled
            progress REAL DEFAULT 0.0,
            speed REAL DEFAULT 0.0,
            remaining_time INTEGER DEFAULT 0,
            download_url TEXT NOT NULL,
            save_path TEXT NOT NULL,
            file_size INTEGER DEFAULT 0,
            downloaded_size INTEGER DEFAULT 0,
            error_message TEXT,
            timestamp INTEGER NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS favorites (
            app_id TEXT PRIMARY KEY,
            added_date INTEGER NOT NULL
        );
        
        CREATE TABLE IF NOT EXISTS installation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            app_id TEXT NOT NULL,
            step TEXT NOT NULL,               -- Downloading, Extracting, Installing, Creating Shortcut, Verification, Finished, Failed
            status TEXT NOT NULL,             -- Info, Success, Error
            message TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        );
    ").expect("Failed to initialize SQLite database schema");
    
    // Check and populate default configuration settings if empty
    let settings_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM settings",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    if settings_count == 0 {
        let default_settings = [
            ("theme", "dark"),
            ("accent_color", "#FF6A00"),
            ("language", "en"),
            ("github_repo", "agneax/store-repo"),
            ("download_location", ""), // Empty string defaults to system Downloads or App Cache
            ("concurrent_downloads", "3"),
            ("auto_update", "true"),
            ("auto_launch_after_install", "false"),
            ("desktop_shortcut", "true"),
            ("notifications", "true")
        ];
        for (key, val) in default_settings.iter() {
            let _ = conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                params![key, val]
            );
        }
    }
    
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_initialization() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        
        let tables = vec!["settings", "cached_apps", "installed_apps", "downloads", "favorites", "installation_logs"];
        for table in tables {
            let count: i64 = conn.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            ).unwrap();
            assert_eq!(count, 1, "Table {} should exist after initialization", table);
        }
        
        let theme: String = conn.query_row(
            "SELECT value FROM settings WHERE key='theme'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(theme, "dark");
    }
}
