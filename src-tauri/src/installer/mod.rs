use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection};
use serde_json::Value;
use sha2::{Sha256, Digest};
use futures_util::stream::StreamExt;
use crate::logger::log_step;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[tauri::command]
pub async fn install_app(app_id: String) -> Result<(), String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // 1. Get application metadata from local DB cache
    let (name, version, platform_data_str): (String, String, String) = conn.query_row(
        "SELECT name, version, platform_data FROM cached_apps WHERE id = ?1",
        params![app_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| format!("App {} metadata not found in database cache. Sync first.", app_id))?;
    
    let platform_data: Value = serde_json::from_str(&platform_data_str)
        .map_err(|e| format!("Failed to parse platform data: {}", e))?;
        
    log_step(&conn, &app_id, "Installing", "Info", &format!("Starting installation process for {}...", name));
    
    // 2. Dispatch installation depending on compile target
    #[cfg(target_os = "windows")]
    {
        let win_data = &platform_data["windows"];
        if win_data.is_null() {
            let err = "No Windows installation instructions found in application metadata".to_string();
            log_step(&conn, &app_id, "Failed", "Error", &err);
            return Err(err);
        }
        
        let default_manager = win_data["default_manager"].as_str().unwrap_or("direct");
        let mut installed_ok = false;
        let mut install_method = default_manager.to_string();
        let mut exec_path = String::new();
        
        // Strategy A: Try preferred package managers
        if default_manager == "winget" && windows::is_winget_available() {
            if let Some(pkg_id) = win_data["winget"]["id"].as_str() {
                if windows::run_winget_install(pkg_id, &app_id, &db_path).is_ok() {
                    installed_ok = true;
                }
            }
        } else if default_manager == "chocolatey" && windows::is_choco_available() {
            if let Some(pkg_id) = win_data["chocolatey"]["id"].as_str() {
                if windows::run_choco_install(pkg_id, &app_id, &db_path).is_ok() {
                    installed_ok = true;
                }
            }
        } else if default_manager == "scoop" && windows::is_scoop_available() {
            if let Some(pkg_id) = win_data["scoop"]["id"].as_str() {
                if windows::run_scoop_install(pkg_id, &app_id, &db_path).is_ok() {
                    installed_ok = true;
                }
            }
        }
        
        // Strategy B: Fallback to direct installer download (EXE/MSI/Portable ZIP)
        if !installed_ok && !win_data["direct"].is_null() {
            let direct_data = &win_data["direct"];
            let dl_url = direct_data["url"].as_str().ok_or_else(|| "Missing download URL".to_string())?;
            let file_type = direct_data["file_type"].as_str().unwrap_or("exe");
            let silent_args = direct_data["silent_args"].as_str().unwrap_or("");
            let expected_checksum = direct_data["checksum"].as_str().unwrap_or("");
            
            install_method = if file_type == "zip" { "portable".to_string() } else { "direct".to_string() };
            
            // Download installer file to cache dir
            let proj_dirs = directories::ProjectDirs::from("com", "agneax", "store").unwrap();
            let cache_dir = proj_dirs.cache_dir();
            let _ = fs::create_dir_all(&cache_dir);
            let temp_filename = format!("install_{}_{}.{}", app_id, version, file_type);
            let download_path = cache_dir.join(&temp_filename);
            
            log_step(&conn, &app_id, "Downloading", "Info", &format!("Downloading installer from: {}", dl_url));
            download_installer_file(dl_url, &download_path).await?;
            log_step(&conn, &app_id, "Downloading", "Success", "Installer downloaded successfully.");
            
            // Verify hash if present
            if !expected_checksum.is_empty() {
                log_step(&conn, &app_id, "Installing", "Info", "Verifying SHA-256 installer checksum...");
                let checksum_ok = verify_sha256(&download_path, expected_checksum)?;
                if !checksum_ok {
                    let err = "SHA-256 checksum verification failed. The installer file may be corrupted or modified.".to_string();
                    log_step(&conn, &app_id, "Failed", "Error", &err);
                    let _ = fs::remove_file(&download_path);
                    return Err(err);
                }
                log_step(&conn, &app_id, "Installing", "Success", "SHA-256 checksum verified successfully.");
            }
            
            // Run installer
            if file_type == "zip" {
                exec_path = windows::run_portable_zip_install(&download_path, &app_id, &name, &db_path)?;
                installed_ok = true;
            } else {
                windows::run_direct_msi_exe_install(&download_path, file_type, silent_args, &app_id, &name, &db_path)?;
                installed_ok = true;
            }
            
            // Cleanup temp file
            let _ = fs::remove_file(&download_path);
        }
        
        if !installed_ok {
            let err = "Failed to install application. No compatible package managers found and direct installer fallback was not configured.".to_string();
            log_step(&conn, &app_id, "Failed", "Error", &err);
            return Err(err);
        }
        
        // Register in DB
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        conn.execute(
            "INSERT OR REPLACE INTO installed_apps (app_id, name, version, install_date, install_method, executable_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![app_id, name, version, now, install_method, exec_path],
        ).map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    {
        let lin_data = &platform_data["linux"];
        if lin_data.is_null() {
            let err = "No Linux installation instructions found in application metadata".to_string();
            log_step(&conn, &app_id, "Failed", "Error", &err);
            return Err(err);
        }
        
        let default_manager = lin_data["default_manager"].as_str().unwrap_or("flatpak");
        let mut installed_ok = false;
        let mut install_method = default_manager.to_string();
        let mut exec_path = String::new();
        
        // Strategy A: Try Flatpak (user-space, preferred for security and non-root access)
        if default_manager == "flatpak" && linux::is_command_available("flatpak") {
            if let Some(pkg_id) = lin_data["flatpak"]["id"].as_str() {
                if linux::run_flatpak_install(pkg_id, &app_id, &db_path).is_ok() {
                    installed_ok = true;
                }
            }
        }
        
        // Strategy B: Try native package manager depending on available host commands
        if !installed_ok {
            if linux::is_command_available("apt-get") && !lin_data["apt"].is_null() {
                if let Some(pkg_id) = lin_data["apt"]["id"].as_str() {
                    install_method = "apt".to_string();
                    if linux::run_apt_install(pkg_id, &app_id, &db_path).is_ok() {
                        installed_ok = true;
                    }
                }
            } else if linux::is_command_available("dnf") && !lin_data["dnf"].is_null() {
                if let Some(pkg_id) = lin_data["dnf"]["id"].as_str() {
                    install_method = "dnf".to_string();
                    if linux::run_dnf_install(pkg_id, &app_id, &db_path).is_ok() {
                        installed_ok = true;
                    }
                }
            } else if linux::is_command_available("pacman") && !lin_data["pacman"].is_null() {
                if let Some(pkg_id) = lin_data["pacman"]["id"].as_str() {
                    install_method = "pacman".to_string();
                    if linux::run_pacman_install(pkg_id, &app_id, &db_path).is_ok() {
                        installed_ok = true;
                    }
                }
            }
        }
        
        // Strategy C: Fallback to direct binaries (AppImage, tar.gz)
        if !installed_ok && !lin_data["direct"].is_null() {
            let direct_data = &lin_data["direct"];
            let dl_url = direct_data["url"].as_str().ok_or_else(|| "Missing download URL".to_string())?;
            let file_type = direct_data["file_type"].as_str().unwrap_or("appimage");
            let expected_checksum = direct_data["checksum"].as_str().unwrap_or("");
            
            install_method = file_type.to_string();
            
            // Download installer file to cache dir
            let proj_dirs = directories::ProjectDirs::from("com", "agneax", "store").unwrap();
            let cache_dir = proj_dirs.cache_dir();
            let _ = fs::create_dir_all(&cache_dir);
            let temp_filename = format!("install_{}_{}.{}", app_id, version, file_type);
            let download_path = cache_dir.join(&temp_filename);
            
            log_step(&conn, &app_id, "Downloading", "Info", &format!("Downloading source file: {}", dl_url));
            download_installer_file(dl_url, &download_path).await?;
            log_step(&conn, &app_id, "Downloading", "Success", "Source download complete.");
            
            // Verify hash if present
            if !expected_checksum.is_empty() {
                log_step(&conn, &app_id, "Installing", "Info", "Verifying SHA-256 download checksum...");
                let checksum_ok = verify_sha256(&download_path, expected_checksum)?;
                if !checksum_ok {
                    let err = "SHA-256 checksum verification failed.".to_string();
                    log_step(&conn, &app_id, "Failed", "Error", &err);
                    let _ = fs::remove_file(&download_path);
                    return Err(err);
                }
                log_step(&conn, &app_id, "Installing", "Success", "SHA-256 checksum verified.");
            }
            
            // Install appimage or tarball
            if file_type == "appimage" {
                exec_path = linux::run_appimage_install(&download_path, &app_id, &name, &db_path)?;
                installed_ok = true;
            } else if file_type == "targz" || file_type == "tar.gz" {
                exec_path = linux::run_targz_install(&download_path, &app_id, &name, &db_path)?;
                installed_ok = true;
            }
            
            // Cleanup temp file
            let _ = fs::remove_file(&download_path);
        }
        
        if !installed_ok {
            let err = "Linux installation failed. Compatible package managers (Flatpak, Snap, APT) not found and direct binary installation failed.".to_string();
            log_step(&conn, &app_id, "Failed", "Error", &err);
            return Err(err);
        }
        
        // Register in DB
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        conn.execute(
            "INSERT OR REPLACE INTO installed_apps (app_id, name, version, install_date, install_method, executable_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![app_id, name, version, now, install_method, exec_path],
        ).map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Unsupported operating system target".to_string())
    }
}

#[tauri::command]
pub async fn uninstall_app(app_id: String) -> Result<(), String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // 1. Query registry in database to find install method and metadata mapping
    let (install_method, name): (String, String) = conn.query_row(
        "SELECT install_method, name FROM installed_apps WHERE app_id = ?1",
        params![app_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| format!("App {} is not registered as installed in local database.", app_id))?;
    
    // 2. Query platform data to find package ID mapping
    let platform_data_str: String = conn.query_row(
        "SELECT platform_data FROM cached_apps WHERE id = ?1",
        params![app_id],
        |row| row.get(0),
    ).unwrap_or_else(|_| "{}".to_string());
    
    let platform_data: Value = serde_json::from_str(&platform_data_str).unwrap_or(Value::Null);
    
    #[cfg(target_os = "windows")]
    {
        let win_data = &platform_data["windows"];
        let package_id = match install_method.as_str() {
            "winget" => win_data["winget"]["id"].as_str().unwrap_or(&app_id),
            "chocolatey" => win_data["chocolatey"]["id"].as_str().unwrap_or(&app_id),
            "scoop" => win_data["scoop"]["id"].as_str().unwrap_or(&app_id),
            _ => &app_id
        };
        windows::uninstall_package_windows(&app_id, &install_method, package_id, &db_path)?;
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    {
        let lin_data = &platform_data["linux"];
        let package_id = match install_method.as_str() {
            "flatpak" => lin_data["flatpak"]["id"].as_str().unwrap_or(&app_id),
            "snap" => lin_data["snap"]["id"].as_str().unwrap_or(&app_id),
            "apt" => lin_data["apt"]["id"].as_str().unwrap_or(&app_id),
            _ => &app_id
        };
        linux::uninstall_package_linux(&app_id, &install_method, package_id, &db_path)?;
        Ok(())
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Unsupported operating system target".to_string())
    }
}

#[tauri::command]
pub fn get_installed_apps() -> Result<serde_json::Value, String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(
        "SELECT app_id, name, version, install_date, install_method, executable_path FROM installed_apps"
    ).map_err(|e| e.to_string())?;
    
    let apps_iter = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "app_id": row.get::<_, String>(0)?,
            "name": row.get::<_, String>(1)?,
            "version": row.get::<_, String>(2)?,
            "install_date": row.get::<_, i64>(3)?,
            "install_method": row.get::<_, String>(4)?,
            "executable_path": row.get::<_, Option<String>>(5)?,
        }))
    }).map_err(|e| e.to_string())?;
    
    let list: Vec<serde_json::Value> = apps_iter.filter_map(Result::ok).collect();
    Ok(serde_json::json!(list))
}

// Download Helper
async fn download_installer_file(url: &str, save_path: &Path) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client.get(url).send().await
        .map_err(|e| format!("Failed to initiate download: {}", e))?;
        
    if !res.status().is_success() {
        return Err(format!("Download request failed with status: {}", res.status()));
    }
    
    let mut file = fs::File::create(save_path)
        .map_err(|e| format!("Failed to create local destination file: {}", e))?;
        
    let mut stream = res.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Error receiving payload chunk: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk to local storage: {}", e))?;
    }
    
    Ok(())
}

// Checksum Helper
fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<bool, String> {
    let mut file = fs::File::open(file_path).map_err(|e| format!("Failed to open file for checksum: {}", e))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| format!("Error copying bytes to hash worker: {}", e))?;
    let hash = hasher.finalize();
    let computed_hex = format!("{:x}", hash);
    Ok(computed_hex.eq_ignore_ascii_case(expected_hex))
}
