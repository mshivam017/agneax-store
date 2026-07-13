mod db;
mod logger;
mod downloader;
mod github;
mod installer;
mod launcher;
mod shortcut;

use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use rusqlite::params;

#[tauri::command]
fn get_settings() -> Result<serde_json::Value, String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(|e| e.to_string())?;
    let settings_iter = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    
    let mut map = serde_json::Map::new();
    for entry in settings_iter.filter_map(Result::ok) {
        map.insert(entry.0, serde_json::Value::String(entry.1));
    }
    Ok(serde_json::Value::Object(map))
}

#[tauri::command]
fn save_setting(key: String, value: String) -> Result<(), String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_app_logs(app_id: String) -> Result<Vec<logger::LogEntry>, String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    Ok(logger::get_logs(&conn, &app_id))
}

#[tauri::command]
fn pick_download_directory() -> Result<Option<String>, String> {
    let folder = rfd::FileDialog::new()
        .set_title("Select Save Directory")
        .pick_folder();
    Ok(folder.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
fn export_app_logs_interactive(app_id: String) -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .set_title("Save Installation Logs")
        .set_file_name(&format!("{}_install_log.txt", app_id))
        .add_filter("Text Files", &["txt"])
        .save_file();
        
    if let Some(path) = file {
        let db_path = db::get_db_path();
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
        logger::export_to_file(&conn, &app_id, path.clone()).map_err(|e| e.to_string())?;
        Ok(Some(path.to_string_lossy().into_owned()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn toggle_favorite(app_id: String) -> Result<bool, String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM favorites WHERE app_id = ?1)",
        params![app_id],
        |row| row.get(0)
    ).unwrap_or(false);
    
    if exists {
        conn.execute("DELETE FROM favorites WHERE app_id = ?1", params![app_id]).map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        conn.execute("INSERT INTO favorites (app_id, added_date) VALUES (?1, ?2)", params![app_id, now]).map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
fn is_app_favorite(app_id: String) -> Result<bool, String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM favorites WHERE app_id = ?1)",
        params![app_id],
        |row| row.get(0)
    ).unwrap_or(false);
    Ok(exists)
}

#[tauri::command]
fn get_favorites() -> Result<Vec<String>, String> {
    let db_path = db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT app_id FROM favorites").map_err(|e| e.to_string())?;
    let fav_iter = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    let list: Vec<String> = fav_iter.filter_map(Result::ok).collect();
    Ok(list)
}

// Window control commands for our custom frameless titlebar
#[tauri::command]
fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_maximize_window(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. Initialize SQLite Database path and tables
    let db_path = db::get_db_path();
    let _conn = db::init_db();
    
    // 2. Initialize the download manager
    let download_manager = downloader::DownloadManager::new(db_path.clone());
    
    let sync_db_path = db_path.clone();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(downloader::DownloadManagerState(download_manager))
        .setup(move |_app| {
            github::start_background_sync(sync_db_path);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings Commands
            get_settings,
            save_setting,
            // Favorites Commands
            toggle_favorite,
            is_app_favorite,
            get_favorites,
            // Installation Logs Commands
            get_app_logs,
            export_app_logs_interactive,
            pick_download_directory,
            // Downloader Commands
            downloader::start_download,
            downloader::pause_download,
            downloader::resume_download,
            downloader::cancel_download,
            downloader::get_downloads_history,
            // GitHub Sync Commands
            github::sync_catalog,
            github::get_cached_apps,
            github::get_app_details,
            // Installer / Uninstaller Commands
            installer::install_app,
            installer::uninstall_app,
            installer::get_installed_apps,
            // App Launcher Commands
            launcher::launch_app,
            // Window controls
            minimize_window,
            toggle_maximize_window,
            close_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
