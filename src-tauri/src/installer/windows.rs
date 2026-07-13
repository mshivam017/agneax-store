#![cfg(target_os = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use rusqlite::{params, Connection};
use crate::shortcut::create_windows_shortcut;
use crate::logger::log_step;

pub fn is_winget_available() -> bool {
    Command::new("where.exe").arg("winget").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn is_choco_available() -> bool {
    Command::new("where.exe").arg("choco").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn is_scoop_available() -> bool {
    Command::new("where.exe").arg("scoop").output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn run_winget_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running winget command: winget install {} --silent", package_id));
    
    let output = Command::new("winget")
        .args(&["install", package_id, "--silent", "--accept-source-agreements", "--accept-package-agreements"])
        .output()
        .map_err(|e| format!("Failed to spawn winget process: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Winget installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let error_msg = format!("Winget failed. Stdout: {}. Stderr: {}", out, err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_choco_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running choco command: choco install {} -y", package_id));
    
    // Chocolatey usually requires administrator privileges, so we can try to run it.
    // If it fails, the error logs will guide the user.
    let output = Command::new("choco")
        .args(&["install", package_id, "-y"])
        .output()
        .map_err(|e| format!("Failed to spawn choco process: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Chocolatey installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Chocolatey failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_scoop_install(package_id: &str, app_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", &format!("Running scoop command: scoop install {}", package_id));
    
    let output = Command::new("scoop")
        .args(&["install", package_id])
        .output()
        .map_err(|e| format!("Failed to spawn scoop process: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Scoop installation completed successfully.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let error_msg = format!("Scoop failed: {}", err);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}

pub fn run_direct_msi_exe_install(
    file_path: &Path,
    file_type: &str,
    silent_args: &str,
    app_id: &str,
    app_name: &str,
    db_path: &Path,
) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Installing", "Info", "Running installer executable silently...");
    
    let installer_str = file_path.to_string_lossy();
    
    let (program, args) = if file_type.eq_ignore_ascii_case("msi") {
        ("msiexec.exe".to_string(), vec!["/i".to_string(), format!("\"{}\"", installer_str), "/qn".to_string(), "/norestart".to_string()])
    } else {
        // Splitting arguments from settings
        let mut parts = vec![installer_str.into_owned()];
        for arg in silent_args.split_whitespace() {
            parts.push(arg.to_string());
        }
        let prog = parts[0].clone();
        let r_args = parts[1..].to_vec();
        (prog, r_args)
    };
    
    // We launch the process and wait for it to complete.
    // If it requires UAC admin prompt, Windows will trigger UAC automatically for installers.
    // However, executing in standard Command might fail if the UAC cannot prompt.
    // In that case, we spawn it using powershell Start-Process -Verb RunAs which forces UAC prompt:
    log_step(&conn, app_id, "Installing", "Info", &format!("Executing command: {} {}", program, args.join(" ")));
    
    let mut cmd = Command::new(&program);
    for arg in &args {
        cmd.arg(arg);
    }
    
    let output = cmd.output()
        .map_err(|e| format!("Failed to execute installer process: {}", e))?;
        
    if output.status.success() {
        log_step(&conn, app_id, "Finished", "Success", "Direct installer executed successfully.");
        // Registry scanning is done in launcher.rs to find executable paths
        Ok("".to_string())
    } else {
        // If standard run fails due to privilege error, retry using PowerShell RunAs (Admin prompt)
        log_step(&conn, app_id, "Installing", "Info", "Standard execution returned non-zero. Attempting elevated installation via UAC prompt...");
        
        let args_str = args.iter().map(|a| format!("'{}'", a)).collect::<Vec<String>>().join(",");
        let ps_cmd = format!("Start-Process -FilePath '{}' -ArgumentList @({}) -Verb RunAs -Wait -PassThru", program, args_str);
        
        let elevate_out = Command::new("powershell")
            .args(&["-NoProfile", "-Command", &ps_cmd])
            .output()
            .map_err(|e| format!("Failed to run elevation script: {}", e))?;
            
        if elevate_out.status.success() {
            log_step(&conn, app_id, "Finished", "Success", "Elevated installation completed successfully.");
            Ok("".to_string())
        } else {
            let err = String::from_utf8_lossy(&elevate_out.stderr);
            let error_msg = format!("Elevated installation failed or was rejected by user. Error: {}", err);
            log_step(&conn, app_id, "Failed", "Error", &error_msg);
            Err(error_msg)
        }
    }
}

pub fn run_portable_zip_install(
    zip_path: &Path,
    app_id: &str,
    app_name: &str,
    db_path: &Path,
) -> Result<String, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Extracting", "Info", "Extracting portable ZIP archive...");
    
    let proj_dirs = directories::ProjectDirs::from("com", "agneax", "store")
        .ok_or_else(|| "Failed to get project directories".to_string())?;
    
    let install_dir = proj_dirs.data_dir().join("installed").join(app_id);
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("Failed to create folder: {}", e))?;
        
    let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Zip index error: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => install_dir.join(path),
            None => continue,
        };
        
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    
    log_step(&conn, app_id, "Extracting", "Success", "ZIP extraction completed successfully.");
    
    // Auto-detect primary executable
    let mut exec_path = PathBuf::new();
    let mut found = false;
    
    if let Ok(entries) = fs::read_dir(&install_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "exe") {
                // Ignore updater files if possible
                let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                if !filename.contains("update") && !filename.contains("uninstall") {
                    exec_path = path;
                    found = true;
                    break;
                }
            }
        }
    }
    
    // If not found in root, check subdirectories
    if !found {
        if let Ok(entries) = fs::read_dir(&install_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(subentries) = fs::read_dir(&path) {
                        for subentry in subentries.filter_map(Result::ok) {
                            let subpath = subentry.path();
                            if subpath.is_file() && subpath.extension().map_or(false, |ext| ext == "exe") {
                                exec_path = subpath;
                                found = true;
                                break;
                            }
                        }
                    }
                }
                if found { break; }
            }
        }
    }
    
    if !found {
        let error_msg = "Failed to auto-detect any executable (.exe) inside the extracted ZIP folder".to_string();
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        return Err(error_msg);
    }
    
    let exec_path_str = exec_path.to_string_lossy().into_owned();
    log_step(&conn, app_id, "Creating Shortcut", "Info", "Creating Start Menu shortcut for portable app...");
    
    match create_windows_shortcut(app_name, &exec_path_str, &install_dir.to_string_lossy()) {
        Ok(shortcut_path) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut created at: {}", shortcut_path));
            Ok(exec_path_str)
        }
        Err(e) => {
            log_step(&conn, app_id, "Finished", "Success", &format!("Installation completed. Shortcut creation failed: {}", e));
            Ok(exec_path_str)
        }
    }
}

pub fn uninstall_package_windows(app_id: &str, method: &str, package_id: &str, db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    log_step(&conn, app_id, "Uninstalling", "Info", &format!("Running uninstaller using method: {}", method));
    
    let success = match method {
        "winget" => {
            let output = Command::new("winget")
                .args(&["uninstall", package_id, "--silent"])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "chocolatey" => {
            let output = Command::new("choco")
                .args(&["uninstall", package_id, "-y"])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "scoop" => {
            let output = Command::new("scoop")
                .args(&["uninstall", package_id])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
        "direct" | "portable" => {
            // Delete install folder for portable
            let proj_dirs = directories::ProjectDirs::from("com", "agneax", "store");
            if let Some(dirs) = proj_dirs {
                let install_dir = dirs.data_dir().join("installed").join(app_id);
                if install_dir.exists() {
                    let _ = fs::remove_dir_all(install_dir);
                }
            }
            // Remove shortcut
            let _ = crate::shortcut::remove_windows_shortcut(app_id);
            true
        }
        _ => false
    };
    
    if success {
        log_step(&conn, app_id, "Finished", "Success", "Application uninstalled successfully.");
        // Delete record from DB
        let _ = conn.execute("DELETE FROM installed_apps WHERE app_id = ?1", params![app_id]);
        Ok(())
    } else {
        let error_msg = format!("Uninstallation failed for method: {}", method);
        log_step(&conn, app_id, "Failed", "Error", &error_msg);
        Err(error_msg)
    }
}
