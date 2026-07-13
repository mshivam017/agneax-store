use rusqlite::{params, Connection, Result};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub id: Option<i64>,
    pub app_id: String,
    pub step: String,      // Downloading, Extracting, Installing, Creating Shortcut, Verification, Finished, Failed
    pub status: String,    // Info, Success, Error
    pub message: String,
    pub timestamp: i64,
}

pub fn log_step(conn: &Connection, app_id: &str, step: &str, status: &str, message: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    
    let _ = conn.execute(
        "INSERT INTO installation_logs (app_id, step, status, message, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![app_id, step, status, message, now]
    );
    
    // Output log to stdout for real-time development inspection
    println!("[INSTALL-LOG] [{}][{}][{}] {}", app_id, step, status, message);
}

pub fn get_logs(conn: &Connection, app_id: &str) -> Vec<LogEntry> {
    let mut stmt = match conn.prepare(
        "SELECT id, app_id, step, status, message, timestamp FROM installation_logs WHERE app_id = ?1 ORDER BY id ASC"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    
    let log_iter = stmt.query_map(params![app_id], |row| {
        Ok(LogEntry {
            id: Some(row.get(0)?),
            app_id: row.get(1)?,
            step: row.get(2)?,
            status: row.get(3)?,
            message: row.get(4)?,
            timestamp: row.get(5)?,
        })
    });
    
    match log_iter {
        Ok(iter) => iter.filter_map(Result::ok).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn export_to_file(conn: &Connection, app_id: &str, target_path: PathBuf) -> std::io::Result<()> {
    let logs = get_logs(conn, app_id);
    let mut log_text = format!("Agneax Store Installation Logs for App ID: {}\n", app_id);
    log_text.push_str("===================================================\n\n");
    
    for entry in logs {
        let datetime = format_timestamp(entry.timestamp);
        log_text.push_str(&format!(
            "[{}] [{:<17}] [{:<7}] {}\n",
            datetime, entry.step, entry.status, entry.message
        ));
    }
    
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target_path, log_text)?;
    Ok(())
}

fn format_timestamp(secs: i64) -> String {
    // Custom readable timestamp format without extra crates
    let days_since_epoch = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;
    format!("Epoch Day {} - {:02}:{:02}:{:02} UTC", days_since_epoch, hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_system() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        
        log_step(&conn, "test-app", "Download", "Success", "Downloaded mock installer successfully");
        log_step(&conn, "test-app", "Install", "Error", "Failed due to missing dependency");
        
        let logs = get_logs(&conn, "test-app");
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].step, "Download");
        assert_eq!(logs[0].status, "Success");
        assert_eq!(logs[1].step, "Install");
        assert_eq!(logs[1].status, "Error");
    }
}
