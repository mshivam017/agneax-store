use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn create_windows_shortcut(name: &str, exec_path: &str, work_dir: &str) -> Result<String, String> {
    let app_data = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    let shortcut_dir = Path::new(&app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Agneax Store");
    
    std::fs::create_dir_all(&shortcut_dir)
        .map_err(|e| format!("Failed to create shortcut directory: {}", e))?;
        
    let shortcut_path = shortcut_dir.join(format!("{}.lnk", name));
    
    // Inline PowerShell script using COM object to write the LNK file
    let ps_script = format!(
        "$WshShell = New-Object -ComObject WScript.Shell; \
         $Shortcut = $WshShell.CreateShortcut('{}'); \
         $Shortcut.TargetPath = '{}'; \
         $Shortcut.WorkingDirectory = '{}'; \
         $Shortcut.Save()",
        shortcut_path.to_string_lossy().replace("'", "''"),
        exec_path.replace("'", "''"),
        work_dir.replace("'", "''")
    );
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell script: {}", e))?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("PowerShell execution failed: {}", err));
    }
    
    Ok(shortcut_path.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "windows"))]
pub fn create_windows_shortcut(_name: &str, _exec_path: &str, _work_dir: &str) -> Result<String, String> {
    Err("Windows shortcut creation is only supported on Windows hosts".to_string())
}

#[cfg(target_os = "linux")]
pub fn create_linux_shortcut(id: &str, name: &str, exec_path: &str, icon_path: &str, categories: &str) -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let desktop_dir = Path::new(&home).join(".local").join("share").join("applications");
    
    std::fs::create_dir_all(&desktop_dir)
        .map_err(|e| format!("Failed to create Linux applications directory: {}", e))?;
        
    let desktop_path = desktop_dir.join(format!("{}.desktop", id));
    
    let desktop_content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={}\n\
         Exec={}\n\
         Icon={}\n\
         Categories={};\n\
         Terminal=false\n\
         StartupNotify=true\n",
        name, exec_path, icon_path, categories
    );
    
    std::fs::write(&desktop_path, desktop_content)
        .map_err(|e| format!("Failed to write .desktop file: {}", e))?;
        
    // Set executable permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&desktop_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755); // rwxr-xr-x
            let _ = std::fs::set_permissions(&desktop_path, perms);
        }
    }
        
    // Update local desktop application database cache
    let _ = Command::new("update-desktop-database")
        .arg(&desktop_dir)
        .output();
        
    Ok(desktop_path.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "linux"))]
pub fn create_linux_shortcut(_id: &str, _name: &str, _exec_path: &str, _icon_path: &str, _categories: &str) -> Result<String, String> {
    Err("Linux shortcut creation is only supported on Linux hosts".to_string())
}

#[cfg(target_os = "windows")]
pub fn remove_windows_shortcut(name: &str) -> Result<(), String> {
    let app_data = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    let shortcut_path = Path::new(&app_data)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Agneax Store")
        .join(format!("{}.lnk", name));
        
    if shortcut_path.exists() {
        std::fs::remove_file(shortcut_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_windows_shortcut(_name: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn remove_linux_shortcut(id: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let desktop_path = Path::new(&home)
        .join(".local")
        .join("share")
        .join("applications")
        .join(format!("{}.desktop", id));
        
    if desktop_path.exists() {
        std::fs::remove_file(desktop_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn remove_linux_shortcut(_id: &str) -> Result<(), String> {
    Ok(())
}
