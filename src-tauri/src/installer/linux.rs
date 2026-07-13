#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use rusqlite::{params, Connection};
use crate::shortcut::create_linux_shortcut;
use crate::logger::log_step;
use flate2::read::GzDecoder;
use tar::Archive;

pub fn is_command_available(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn run_flatpak_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running flatpak command: flatpak install --user -y flathub {}", package_id));
    
    let output = Command::new("flatpak")
        .args(&["install", "--user", "-y", "flathub", package_id])
        .output()
        .map_err(|e| format!("Failed to spawn flatpak process: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Flatpak installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Flatpak installation failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_snap_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running snap command with pkexec elevation: snap install {}", package_id));
    
    // Snaps usually require root, so we elevate via pkexec
    let output = Command::new("pkexec")
        .args(&["snap", "install", package_id, "--classic"])
        .output()
        .map_err(|e| format!("Failed to spawn pkexec snap: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Snap installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Snap installation failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_apt_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running APT install with pkexec elevation: apt-get install -y {}", package_id));
    
    // We execute apt-get update and apt-get install wrapped in pkexec
    let output = Command::new("pkexec")
        .args(&["sh", "-c", &format!("apt-get update && apt-get install -y {}", package_id)])
        .output()
        .map_err(|e| format!("Failed to spawn pkexec apt: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "APT installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("APT installation failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_dnf_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running DNF install with pkexec elevation: dnf install -y {}", package_id));
    
    let output = Command::new("pkexec")
        .args(&["dnf", "install", "-y", package_id])
        .output()
        .map_err(|e| format!("Failed to spawn pkexec dnf: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "DNF installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("DNF installation failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_pacman_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running Pacman install with pkexec elevation: pacman -S --noconfirm {}", package_id));
    
    let output = Command::new("pkexec")
        .args(&["pacman", "-S", "--noconfirm", package_id])
        .output()
        .map_err(|e| format!("Failed to spawn pkexec pacman: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Pacman installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Pacman installation failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_appimage_install(
    file_path: &Path,
    app_id: &str,
    app_name: &str,
    db_path: &Path,
) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", "Configuring AppImage file...");
    
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let bin_dir = Path::new(&home).join(".local").join("bin");
    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create folder ~/.local/bin: {}", e))?;
        
    let filename = file_path.file_name().ok_or_else(|| "Invalid file path".to_string())?;
    let dest_path = bin_dir.join(filename);
    
    fs::copy(file_path, &dest_path)
        .map_err(|e| format!("Failed to move AppImage to ~/.local/bin: {}", e))?;
        
    // Mark AppImage as executable (chmod +x)
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(&dest_path) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        fs::set_permissions(&dest_path, perms)
            .map_err(|e| format!("Failed to make AppImage executable: {}", e))?;
    }
    
    log_step(&conn, app_id, "Creating Shortcut", "Info", "Generating .desktop shortcut integration...");
    
    // Check if we have an icon cache or logo path. If not, default to standard generic icon
    let icon_dir = Path::new(&home).join(".local").join("share").join("icons");
    let _ = fs::create_dir_all(&icon_dir);
    let icon_dest = icon_dir.join(format!("{}.png", app_id));
    
    // If our app had a downloaded icon, we would specify it. Otherwise we fallback to generic
    let icon_str = if icon_dest.exists() {
        icon_dest.to_string_lossy().into_owned()
    } else {
        "system-run".to_string() // Generic fallback desktop icon
    };
    
    let exec_path_str = dest_path.to_string_lossy().into_owned();
    
    match create_linux_shortcut(app_id, app_name, &exec_path_str, &icon_str, "Utility") {
        Ok(shortcut_path) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut registered at: {}", shortcut_path));
            Ok(exec_path_str)
        }
        Err(e) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut creation failed: {}", e));
            Ok(exec_path_str)
        }
    }
}

pub fn run_targz_install(
    file_path: &Path,
    app_id: &str,
    app_name: &str,
    db_path: &Path,
) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Extracting", "Info", "Extracting tar.gz archive...");
    
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let install_dir = Path::new(&home).join(".local").join("share").join("agneax-store").join("installed").join(app_id);
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create folder: {}", e))?;
        
    let tar_gz = fs::File::open(file_path).map_err(|e| format!("Failed to open tar.gz: {}", e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    
    archive.unpack(&install_dir)
        .map_err(|e| format!("Extraction failed: {}", e))?;
        
    log_step(&conn, app_id, "Extracting", "Success", "Tarball unpacked successfully.");
    
    // Detect binary in extracted directory (look for executable files)
    let mut exec_path = PathBuf::new();
    let mut found = false;
    
    // Recursively check files inside unpack directory
    fn find_executable(dir: &Path) -> Option<PathBuf> {
        use std::os::unix::fs::MetadataExt;
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = fs::metadata(&path) {
                        let mode = meta.mode();
                        // Check if file is executable by user (0o100)
                        if (mode & 0o111) != 0 {
                            let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                            if !filename.contains("update") && !filename.contains("uninstall") && !filename.ends_with(".sh") {
                                return Some(path);
                            }
                        }
                    }
                } else if path.is_dir() {
                    if let Some(p) = find_executable(&path) {
                        return Some(p);
                    }
                }
            }
        }
        None
    }
    
    if let Some(p) = find_executable(&install_dir) {
        exec_path = p;
        found = true;
    }
    
    if !found {
        let error_msg = "Failed to detect executable binary inside the extracted tarball".to_string();
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        return Err(error_msg);
    }
    
    let exec_path_str = exec_path.to_string_lossy().into_owned();
    log_step(&conn, app_id, "Creating Shortcut", "Info", "Creating launcher shortcut...");
    
    match create_linux_shortcut(app_id, app_name, &exec_path_str, "system-run", "Utility") {
        Ok(shortcut_path) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut registered at: {}", shortcut_path));
            Ok(exec_path_str)
        }
        Err(e) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut creation failed: {}", e));
            Ok(exec_path_str)
        }
    }
}

pub fn uninstall_package_linux(app_id: &str, method: &str, package_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Uninstalling", "Info", &format!("Running uninstaller using method: {}", method));
    
    let success = match method {
        "flatpak" => {
            let output = Command::new("flatpak")
                .args(&["uninstall", "--user", "-y", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "snap" => {
            let output = Command::new("pkexec")
                .args(&["snap", "remove", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "apt" => {
            let output = Command::new("pkexec")
                .args(&["apt-get", "remove", "-y", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "dnf" => {
            let output = Command::new("pkexec")
                .args(&["dnf", "remove", "-y", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "pacman" => {
            let output = Command::new("pkexec")
                .args(&["pacman", "-R", "--noconfirm", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "appimage" => {
            // Delete ~/.local/bin/ AppImage files matching app_id
            let home = std::env::var("HOME").unwrap_or_default();
            let bin_dir = Path::new(&home).join(".local").join("bin");
            if let Ok(entries) = fs::read_dir(bin_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    if filename.contains(app_id) && filename.contains("appimage") {
                        let _ = fs::remove_file(path);
                    }
                }
            }
            let _ = crate::shortcut::remove_linux_shortcut(app_id);
            true
        }
        "targz" => {
            // Delete ~/.local/share/agneax-store/installed/<app_id> folder
            let home = std::env::var("HOME").unwrap_or_default();
            let install_dir = Path::new(&home).join(".local").join("share").join("agneax-store").join("installed").join(app_id);
            if install_dir.exists() {
                let _ = fs::remove_dir_all(install_dir);
            }
            let _ = crate::shortcut::remove_linux_shortcut(app_id);
            true
        }
        _ => false
    };
    
    if success {
        log_step(&conn, app_id, "Finished", "Success", "Application uninstalled successfully.");
        let _ = conn.execute("DELETE FROM installed_apps WHERE app_id = ?1", params![app_id]);
        Ok(())
    } else {
        let error_msg = format!("Linux uninstallation failed for method: {}", method);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}
