use rusqlite::{params, Connection};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use futures_util::stream::StreamExt;
use directories::ProjectDirs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadProgress {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub version: String,
    pub status: String,
    pub progress: f64,
    pub speed: f64,             // Bytes/sec
    pub remaining_time: u64,    // Seconds
    pub downloaded_size: u64,
    pub file_size: u64,
    pub error_message: Option<String>,
}

pub struct ActiveDownload {
    pub cancel_tx: mpsc::Sender<()>,
    pub app_id: String,
    pub url: String,
    pub save_path: PathBuf,
}

pub struct DownloadManager {
    pub active_downloads: Arc<Mutex<HashMap<String, ActiveDownload>>>,
    pub db_path: PathBuf,
}

impl DownloadManager {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            db_path,
        }
    }
}

// Global state wrapped in Tauri state
pub struct DownloadManagerState(pub DownloadManager);

// Custom helper to generate random string IDs (like UUIDs)
fn generate_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", now)
}

pub async fn start_download_impl(
    app: AppHandle,
    manager: &DownloadManager,
    app_id: String,
    app_name: String,
    version: String,
    download_url: String,
    custom_save_path: Option<String>,
) -> Result<String, String> {
    let download_id = generate_id();
    
    // Resolve save path
    let save_dir = if let Some(path) = custom_save_path {
        if path.is_empty() {
            get_default_download_dir()
        } else {
            PathBuf::from(path)
        }
    } else {
        get_default_download_dir()
    };
    
    fs::create_dir_all(&save_dir).map_err(|e| format!("Failed to create download directory: {}", e))?;
    
    // Extract filename from URL or default
    let filename = download_url
        .split('/')
        .last()
        .unwrap_or("installer.bin")
        .split('?')
        .next()
        .unwrap_or("installer.bin");
        
    let save_path = save_dir.join(format!("{}_{}", download_id, filename));
    
    // Insert pending entry into SQLite downloads database
    let conn = Connection::open(&manager.db_path)
        .map_err(|e| format!("Database connection error: {}", e))?;
        
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
        
    conn.execute(
        "INSERT INTO downloads (id, app_id, app_name, version, status, progress, speed, remaining_time, download_url, save_path, file_size, downloaded_size, timestamp)
         VALUES (?1, ?2, ?3, ?4, 'Pending', 0.0, 0.0, 0, ?5, ?6, 0, 0, ?7)",
        params![download_id, app_id, app_name, version, download_url, save_path.to_string_lossy(), timestamp],
    ).map_err(|e| format!("Failed to save download to database: {}", e))?;

    // Spawn the background downloader task
    let (cancel_tx, cancel_rx) = mpsc::channel(1);
    
    let active_download = ActiveDownload {
        cancel_tx,
        app_id: app_id.clone(),
        url: download_url.clone(),
        save_path: save_path.clone(),
    };
    
    {
        let mut active = manager.active_downloads.lock().await;
        active.insert(download_id.clone(), active_download);
    }
    
    let db_path = manager.db_path.clone();
    let download_id_clone = download_id.clone();
    let active_downloads = manager.active_downloads.clone();
    
    tokio::spawn(async move {
        let res = run_download_loop(
            app.clone(),
            db_path,
            download_id_clone.clone(),
            app_id,
            app_name,
            version,
            download_url,
            save_path,
            cancel_rx,
        ).await;
        
        // Remove from active downloads
        let mut active = active_downloads.lock().await;
        active.remove(&download_id_clone);
        
        if let Err(err_msg) = res {
            eprintln!("Download task {} failed: {}", download_id_clone, err_msg);
        }
    });

    Ok(download_id)
}

async fn run_download_loop(
    app: AppHandle,
    db_path: PathBuf,
    id: String,
    app_id: String,
    app_name: String,
    version: String,
    url: String,
    save_path: PathBuf,
    mut cancel_rx: mpsc::Receiver<()>,
) -> Result<(), String> {
    // 1. Update status to Downloading
    update_db_status(&db_path, &id, "Downloading", None).await?;
    
    // 2. Determine file offset if resuming
    let mut file_offset = 0;
    if save_path.exists() {
        if let Ok(metadata) = fs::metadata(&save_path) {
            file_offset = metadata.len();
        }
    }
    
    // 3. Initiate HTTP request
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    
    if file_offset > 0 {
        req = req.header("Range", format!("bytes={}-", file_offset));
    }
    
    let res = req.send().await.map_err(|e| {
        let msg = format!("Failed to send download request: {}", e);
        let _ = update_db_status(&db_path, &id, "Failed", Some(&msg));
        msg
    })?;
    
    let status_code = res.status();
    
    // Check if range request is supported and server returns partial content (206) or full content (200)
    let is_resume = status_code == reqwest::StatusCode::PARTIAL_CONTENT;
    let actual_offset = if is_resume { file_offset } else { 0 };
    
    // Total size of the file
    let content_len = res.content_length().unwrap_or(0);
    let total_size = if is_resume {
        content_len + file_offset
    } else {
        content_len
    };
    
    // 4. Open file in correct mode
    let mut file = if actual_offset > 0 {
        OpenOptions::new()
            .write(true)
            .open(&save_path)
            .map_err(|e| {
                let msg = format!("Failed to open existing file for resume: {}", e);
                let _ = update_db_status(&db_path, &id, "Failed", Some(&msg));
                msg
            })?
    } else {
        fs::create_dir_all(save_path.parent().unwrap()).unwrap_or(());
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&save_path)
            .map_err(|e| {
                let msg = format!("Failed to create download file: {}", e);
                let _ = update_db_status(&db_path, &id, "Failed", Some(&msg));
                msg
            })?
    };
    
    if actual_offset > 0 {
        file.seek(SeekFrom::Start(actual_offset)).map_err(|e| format!("File seek failed: {}", e))?;
    }
    
    // Initialize stream download metrics
    let mut downloaded = actual_offset;
    let mut stream = res.bytes_stream();
    let start_time = Instant::now();
    let mut last_emit = Instant::now();
    let mut session_downloaded = 0;
    
    while let Some(chunk_result) = stream.next().await {
        // Check for cancellation or pause
        if cancel_rx.try_recv().is_ok() {
            // Check in database if it was a pause or a cancel
            let status = get_db_status(&db_path, &id).await.unwrap_or_else(|_| "Paused".to_string());
            let final_status = if status == "Cancelled" { "Cancelled" } else { "Paused" };
            update_db_status(&db_path, &id, final_status, None).await?;
            
            // Emit progress event indicating termination
            let _ = app.emit("download-progress", DownloadProgress {
                id: id.clone(),
                app_id: app_id.clone(),
                app_name: app_name.clone(),
                version: version.clone(),
                status: final_status.to_string(),
                progress: (downloaded as f64 / total_size as f64) * 100.0,
                speed: 0.0,
                remaining_time: 0,
                downloaded_size: downloaded,
                file_size: total_size,
                error_message: None,
            });
            
            if final_status == "Cancelled" {
                let _ = fs::remove_file(&save_path); // Cleanup cancelled downloads
            }
            
            return Ok(());
        }
        
        let chunk = chunk_result.map_err(|e| {
            let msg = format!("Error downloading chunk: {}", e);
            let _ = update_db_status(&db_path, &id, "Failed", Some(&msg));
            msg
        })?;
        
        file.write_all(&chunk).map_err(|e| {
            let msg = format!("Failed to write chunk to file: {}", e);
            let _ = update_db_status(&db_path, &id, "Failed", Some(&msg));
            msg
        })?;
        
        downloaded += chunk.len() as u64;
        session_downloaded += chunk.len() as u64;
        
        // Throttle event emissions to avoid spamming the main thread (~every 200ms)
        if last_emit.elapsed().as_millis() >= 200 || downloaded == total_size {
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let speed = if elapsed_sec > 0.0 { session_downloaded as f64 / elapsed_sec } else { 0.0 };
            
            let remaining_time = if speed > 0.0 && total_size > downloaded {
                ((total_size - downloaded) as f64 / speed) as u64
            } else {
                0
            };
            
            let progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64) * 100.0
            } else {
                0.0
            };
            
            // Update SQLite table with current progress metrics
            update_db_progress(&db_path, &id, progress, speed, remaining_time as i64, downloaded, total_size).await?;
            
            // Emit progress event to React
            let _ = app.emit("download-progress", DownloadProgress {
                id: id.clone(),
                app_id: app_id.clone(),
                app_name: app_name.clone(),
                version: version.clone(),
                status: "Downloading".to_string(),
                progress,
                speed,
                remaining_time,
                downloaded_size: downloaded,
                file_size: total_size,
                error_message: None,
            });
            
            last_emit = Instant::now();
        }
    }
    
    // Complete download
    update_db_status(&db_path, &id, "Completed", None).await?;
    let _ = app.emit("download-progress", DownloadProgress {
        id: id.clone(),
        app_id: app_id.clone(),
        app_name: app_name.clone(),
        version: version.clone(),
        status: "Completed".to_string(),
        progress: 100.0,
        speed: 0.0,
        remaining_time: 0,
        downloaded_size: downloaded,
        file_size: total_size,
        error_message: None,
    });
    
    Ok(())
}

// Database Helpers
async fn update_db_status(db_path: &Path, id: &str, status: &str, err_msg: Option<&str>) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE downloads SET status = ?1, error_message = ?2 WHERE id = ?3",
        params![status, err_msg, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

async fn get_db_status(db_path: &Path, id: &str) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let status: String = conn.query_row(
        "SELECT status FROM downloads WHERE id = ?1",
        params![id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    Ok(status)
}

async fn update_db_progress(
    db_path: &Path,
    id: &str,
    progress: f64,
    speed: f64,
    remaining_time: i64,
    downloaded_size: u64,
    file_size: u64,
) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE downloads SET progress = ?1, speed = ?2, remaining_time = ?3, downloaded_size = ?4, file_size = ?5
         WHERE id = ?6",
        params![progress, speed, remaining_time, downloaded_size as i64, file_size as i64, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn get_default_download_dir() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| {
            let proj_dirs = ProjectDirs::from("com", "agneax", "store").unwrap();
            proj_dirs.cache_dir().join("downloads")
        })
}

// Tauri commands exposed to the frontend
#[tauri::command]
pub async fn start_download(
    state: State<'_, DownloadManagerState>,
    app: AppHandle,
    app_id: String,
    app_name: String,
    version: String,
    url: String,
    save_path: Option<String>,
) -> Result<String, String> {
    start_download_impl(app, &state.0, app_id, app_name, version, url, save_path).await
}

#[tauri::command]
pub async fn pause_download(state: State<'_, DownloadManagerState>, id: String) -> Result<(), String> {
    let active = state.0.active_downloads.lock().await;
    if let Some(dl) = active.get(&id) {
        // Mark as paused in db first so the loop knows it was a pause, not cancel
        update_db_status(&state.0.db_path, &id, "Paused", None).await?;
        let _ = dl.cancel_tx.send(()).await;
        Ok(())
    } else {
        Err("Download not active or already completed/paused".to_string())
    }
}

#[tauri::command]
pub async fn resume_download(
    state: State<'_, DownloadManagerState>,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    // Check download parameters in the database
    let conn = Connection::open(&state.0.db_path).map_err(|e| e.to_string())?;
    let (app_id, app_name, version, url, save_path_str): (String, String, String, String, String) = conn.query_row(
        "SELECT app_id, app_name, version, download_url, save_path FROM downloads WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).map_err(|e| format!("Download record not found: {}", e))?;
    
    // Check if download is already active
    {
        let active = state.0.active_downloads.lock().await;
        if active.contains_key(&id) {
            return Err("Download is already running".to_string());
        }
    }
    
    let save_path = PathBuf::from(save_path_str);
    let (cancel_tx, cancel_rx) = mpsc::channel(1);
    
    let active_download = ActiveDownload {
        cancel_tx,
        app_id: app_id.clone(),
        url: url.clone(),
        save_path: save_path.clone(),
    };
    
    {
        let mut active = state.0.active_downloads.lock().await;
        active.insert(id.clone(), active_download);
    }
    
    let db_path = state.0.db_path.clone();
    let id_clone = id.clone();
    let active_downloads = state.0.active_downloads.clone();
    
    tokio::spawn(async move {
        let res = run_download_loop(
            app.clone(),
            db_path,
            id_clone.clone(),
            app_id,
            app_name,
            version,
            url,
            save_path,
            cancel_rx,
        ).await;
        
        let mut active = active_downloads.lock().await;
        active.remove(&id_clone);
        
        if let Err(err) = res {
            eprintln!("Resumed download failed: {}", err);
        }
    });
    
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, DownloadManagerState>, id: String) -> Result<(), String> {
    let mut active = state.0.active_downloads.lock().await;
    if let Some(dl) = active.get(&id) {
        update_db_status(&state.0.db_path, &id, "Cancelled", None).await?;
        let _ = dl.cancel_tx.send(()).await;
        active.remove(&id);
        Ok(())
    } else {
        // If not active, just update its status in db to Cancelled
        update_db_status(&state.0.db_path, &id, "Cancelled", None).await?;
        
        // Attempt cleanup of the file if it exists
        let conn = Connection::open(&state.0.db_path).map_err(|e| e.to_string())?;
        if let Ok(save_path_str) = conn.query_row::<String, _, _>(
            "SELECT save_path FROM downloads WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ) {
            let path = PathBuf::from(save_path_str);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

#[tauri::command]
pub fn get_downloads_history(state: State<'_, DownloadManagerState>) -> Result<Vec<DownloadProgress>, String> {
    let conn = Connection::open(&state.0.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, app_id, app_name, version, status, progress, speed, remaining_time, downloaded_size, file_size
         FROM downloads ORDER BY timestamp DESC"
    ).map_err(|e| e.to_string())?;
    
    let history_iter = stmt.query_map([], |row| {
        Ok(DownloadProgress {
            id: row.get(0)?,
            app_id: row.get(1)?,
            app_name: row.get(2)?,
            version: row.get(3)?,
            status: row.get(4)?,
            progress: row.get(5)?,
            speed: row.get(6)?,
            remaining_time: row.get::<_, i64>(7)? as u64,
            downloaded_size: row.get::<_, i64>(8)? as u64,
            file_size: row.get::<_, i64>(9)? as u64,
            error_message: None,
        })
    }).map_err(|e| e.to_string())?;
    
    let list: Vec<DownloadProgress> = history_iter.filter_map(Result::ok).collect();
    Ok(list)
}
