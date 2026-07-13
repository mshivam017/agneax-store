use std::path::{Path, PathBuf};
use std::process::Command;
use rusqlite::params;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub fn launch_app(app_id: String) -> Result<(), String> {
    let db_path = crate::db::get_db_path();
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // 1. Look up installed app record in database
    let (exec_path_db, name): (Option<String>, String) = conn.query_row(
        "SELECT executable_path, name FROM installed_apps WHERE app_id = ?1",
        params![app_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| format!("App {} is not registered as installed in database.", app_id))?;
    
    // 2. Resolve executable command
    let resolved_command = if let Some(path) = exec_path_db {
        if !path.is_empty() && Path::new(&path).exists() {
            path
        } else {
            auto_detect_exec(&app_id, &name)?
        }
    } else {
        auto_detect_exec(&app_id, &name)?
    };
    
    // 3. Launch process in detached mode depending on host OS
    launch_detached(&resolved_command)?;
    
    Ok(())
}

fn launch_detached(cmd_str: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW = 0x08000000, DETACHED_PROCESS = 0x00000008
        // Launching via 'cmd /c start' is extremely robust on Windows for launching GUI apps detached
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg("start").arg("").arg(cmd_str);
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS flag
        cmd.spawn().map_err(|e| format!("Failed to spawn Windows process: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Spawning via sh -c 'exec ... &' lets the shell spawn and fork the process into the background
        Command::new("sh")
            .arg("-c")
            .arg(format!("nohup {} >/dev/null 2>&1 &", cmd_str))
            .spawn()
            .map_err(|e| format!("Failed to spawn Linux process: {}", e))?;
    }
    
    Ok(())
}

fn auto_detect_exec(app_id: &str, app_name: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // 1. Check if the app ID matches common executable command names in the PATH
        let common_cmds = match app_id {
            "vscode" => vec!["code.cmd", "code.exe"],
            "brave" => vec!["brave.exe"],
            "gimp" => vec!["gimp-2.10.exe", "gimp.exe"],
            "obs" => vec!["obs64.exe", "obs.exe"],
            "blender" => vec!["blender.exe"],
            "discord" => vec!["Update.exe", "Discord.exe"],
            "vlc" => vec!["vlc.exe"],
            "spotify" => vec!["Spotify.exe"],
            "python" => vec!["python.exe"],
            _ => vec![],
        };
        
        for cmd in common_cmds {
            if is_command_in_path(cmd) {
                return Ok(cmd.to_string());
            }
        }
        
        // 2. Query registry paths or standard directories
        let standard_paths = [
            format!(r"C:\Program Files\{}\{}\{}.exe", app_name, app_name, app_id),
            format!(r"C:\Program Files (x86)\{}\{}.exe", app_name, app_id),
            format!(r"C:\Program Files\Microsoft VS Code\Code.exe"),
            format!(r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"),
            format!(r"C:\Program Files\GIMP 2\bin\gimp-2.10.exe"),
            format!(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
            format!(r"C:\Program Files\Blender Foundation\Blender 4.1\blender.exe"),
            format!(r"C:\Program Files\VideoLAN\VLC\vlc.exe"),
            format!(r"C:\Users\Shivam\AppData\Local\Microsoft\WindowsApps\code.exe"),
            format!(r"C:\Users\Shivam\AppData\Local\Discord\Update.exe"),
        ];
        
        for path in &standard_paths {
            if Path::new(path).exists() {
                // Return quoted path to handle spaces safely
                return Ok(format!("\"{}\"", path));
            }
        }
        
        // Check local profile directories for windows user-level installations
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let discord_path = Path::new(&local_appdata).join("Discord").join("Update.exe");
            if discord_path.exists() {
                return Ok(format!("\"{}\" --processStart Discord.exe", discord_path.to_string_lossy()));
            }
            let vscode_path = Path::new(&local_appdata).join("Programs").join("Microsoft VS Code").join("Code.exe");
            if vscode_path.exists() {
                return Ok(format!("\"{}\"", vscode_path.to_string_lossy()));
            }
        }
        
        // Final fallback: assume it is in PATH under the standard ID name
        Ok(app_id.to_string())
    }
    
    #[cfg(target_os = "linux")]
    {
        // 1. Check if command is in PATH
        let common_cmds = match app_id {
            "vscode" => vec!["code"],
            "brave" => vec!["brave-browser", "brave"],
            "gimp" => vec!["gimp"],
            "obs" => vec!["obs"],
            "blender" => vec!["blender"],
            "discord" => vec!["discord"],
            "vlc" => vec!["vlc"],
            "spotify" => vec!["spotify"],
            _ => vec![app_id],
        };
        
        for cmd in common_cmds {
            if is_command_in_path(cmd) {
                return Ok(cmd.to_string());
            }
        }
        
        // 2. Check flatpak runners
        let flatpak_ids = match app_id {
            "vscode" => vec!["com.visualstudio.code"],
            "brave" => vec!["com.brave.Browser"],
            "gimp" => vec!["org.gimp.GIMP"],
            "obs" => vec!["com.obsproject.Studio"],
            "blender" => vec!["org.blender.Blender"],
            "discord" => vec!["com.discordapp.Discord"],
            "vlc" => vec!["org.videolan.VLC"],
            "spotify" => vec!["com.spotify.Client"],
            _ => vec![],
        };
        
        for fid in flatpak_ids {
            if is_flatpak_installed(fid) {
                return Ok(format!("flatpak run {}", fid));
            }
        }
        
        // 3. Check snap runner
        let snap_cmds = match app_id {
            "vscode" => vec!["snap run code"],
            "discord" => vec!["snap run discord"],
            _ => vec![],
        };
        for scmd in snap_cmds {
            let snap_bin = scmd.split_whitespace().last().unwrap_or("");
            if Path::new(&format!("/snap/bin/{}", snap_bin)).exists() {
                return Ok(scmd.to_string());
            }
        }
        
        // Final fallback: try executing app_id directly
        Ok(app_id.to_string())
    }
}

fn is_command_in_path(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Run where.exe to check path
        let out = Command::new("where.exe").arg(cmd).output();
        if let Ok(o) = out {
            return o.status.success();
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Run which to check path
        let out = Command::new("which").arg(cmd).output();
        if let Ok(o) = out {
            return o.status.success();
        }
    }
    
    false
}

#[cfg(target_os = "linux")]
fn is_flatpak_installed(flatpak_id: &str) -> bool {
    let out = Command::new("flatpak")
        .arg("info")
        .arg(flatpak_id)
        .output();
        
    if let Ok(o) = out {
        return o.status.success();
    }
    false
}
