use rusqlite::{params, Connection};
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub developer: String,
    pub website: String,
    pub github: String,
    pub license: String,
    pub category: String,
    pub icon_url: String,
    pub screenshots: Vec<String>,
    pub download_size: String,
    pub installed_size: String,
    pub dependencies: Vec<String>,
    pub min_os: serde_json::Value,
    pub supported_architectures: Option<Vec<String>>,
    pub change_logs: Option<String>,
    pub platform_data: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct GithubContentItem {
    name: String,
    download_url: Option<String>,
    #[serde(rename = "type")]
    item_type: String,
}

// Write the mock data as a fallback to ensure 100% functionality out of the box
fn get_bundled_apps() -> Vec<AppMetadata> {
    let vscode = AppMetadata {
        id: "vscode".to_string(),
        name: "Visual Studio Code".to_string(),
        description: "Visual Studio Code is a lightweight but powerful source code editor which runs on your desktop and is available for Windows, macOS and Linux.".to_string(),
        version: "1.91.0".to_string(),
        developer: "Microsoft".to_string(),
        website: "https://code.visualstudio.com".to_string(),
        github: "https://github.com/microsoft/vscode".to_string(),
        license: "Proprietary".to_string(),
        category: "Developer Tools".to_string(),
        icon_url: "https://raw.githubusercontent.com/microsoft/vscode-icons/main/icons/stable/code.svg".to_string(),
        screenshots: vec![
            "https://code.visualstudio.com/assets/home/home-screenshot-win-lg.png".to_string(),
            "https://code.visualstudio.com/assets/home/home-screenshot-mac-lg.png".to_string()
        ],
        download_size: "95 MB".to_string(),
        installed_size: "350 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string(), "arm64".to_string()]),
        change_logs: Some("Security fixes and editor improvements.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Microsoft.VisualStudioCode",
                    "install_command": "winget install Microsoft.VisualStudioCode --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Microsoft.VisualStudioCode --silent"
                },
                "chocolatey": {
                    "id": "vscode",
                    "install_command": "choco install vscode -y"
                },
                "direct": {
                    "url": "https://update.code.visualstudio.com/latest/win32-x64-user/stable",
                    "file_type": "exe",
                    "silent_args": "/verysilent /mergetasks=!runcode",
                    "checksum": ""
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "com.visualstudio.code",
                    "install_command": "flatpak install --user -y flathub com.visualstudio.code",
                    "uninstall_command": "flatpak uninstall --user -y com.visualstudio.code"
                },
                "snap": {
                    "id": "code",
                    "install_command": "snap install code --classic"
                },
                "apt": {
                    "id": "code",
                    "install_command": "pkexec apt-get update && pkexec apt-get install -y code"
                }
            }
        })
    };

    let brave = AppMetadata {
        id: "brave".to_string(),
        name: "Brave Browser".to_string(),
        description: "The Brave browser is a fast, private and secure web browser for PC, Mac and mobile. It blocks ads and trackers by default.".to_string(),
        version: "1.67.123".to_string(),
        developer: "Brave Software".to_string(),
        website: "https://brave.com".to_string(),
        github: "https://github.com/brave/brave-browser".to_string(),
        license: "Open Source (MPL 2.0)".to_string(),
        category: "Utilities".to_string(),
        icon_url: "https://brave.com/static-assets/images/brave-logo-sans-text.svg".to_string(),
        screenshots: vec![
            "https://brave.com/static-assets/images/brave-screenshot-1.png".to_string()
        ],
        download_size: "110 MB".to_string(),
        installed_size: "280 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Security updates and browser shield fixes.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Brave.Brave",
                    "install_command": "winget install Brave.Brave --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Brave.Brave --silent"
                },
                "direct": {
                    "url": "https://laptop-updates.brave.com/latest/winx64",
                    "file_type": "exe",
                    "silent_args": "/silent /install",
                    "checksum": ""
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "com.brave.Browser",
                    "install_command": "flatpak install --user -y flathub com.brave.Browser",
                    "uninstall_command": "flatpak uninstall --user -y com.brave.Browser"
                }
            }
        })
    };

    let gimp = AppMetadata {
        id: "gimp".to_string(),
        name: "GIMP Image Editor".to_string(),
        description: "GIMP is a cross-platform image editor available for GNU/Linux, macOS, Windows and more operating systems. It is free software, you can change its source code and distribute your changes.".to_string(),
        version: "2.10.38".to_string(),
        developer: "The GIMP Team".to_string(),
        website: "https://www.gimp.org".to_string(),
        github: "https://gitlab.gnome.org/GNOME/gimp".to_string(),
        license: "GPLv3".to_string(),
        category: "Multimedia".to_string(),
        icon_url: "https://www.gimp.org/images/gimp-logo.png".to_string(),
        screenshots: vec![
            "https://www.gimp.org/screenshots/gimp-2.10-screenshot.jpg".to_string()
        ],
        download_size: "310 MB".to_string(),
        installed_size: "800 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Bug fixes, color management updates.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "GIMP.GIMP",
                    "install_command": "winget install GIMP.GIMP --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall GIMP.GIMP --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "org.gimp.GIMP",
                    "install_command": "flatpak install --user -y flathub org.gimp.GIMP",
                    "uninstall_command": "flatpak uninstall --user -y org.gimp.GIMP"
                }
            }
        })
    };

    let obs = AppMetadata {
        id: "obs".to_string(),
        name: "OBS Studio".to_string(),
        description: "Free and open source software for video recording and live streaming. Download and start streaming quickly and easily on Windows, Mac or Linux.".to_string(),
        version: "30.1.2".to_string(),
        developer: "OBS Project".to_string(),
        website: "https://obsproject.com".to_string(),
        github: "https://github.com/obsproject/obs-studio".to_string(),
        license: "GPLv2".to_string(),
        category: "Multimedia".to_string(),
        icon_url: "https://obsproject.com/assets/images/logo.png".to_string(),
        screenshots: vec![
            "https://obsproject.com/assets/images/screenshot_main.png".to_string()
        ],
        download_size: "128 MB".to_string(),
        installed_size: "400 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 22.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Fixed NVENC encoder issues and audio sync fixes.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "OBSProject.OBSStudio",
                    "install_command": "winget install OBSProject.OBSStudio --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall OBSProject.OBSStudio --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "com.obsproject.Studio",
                    "install_command": "flatpak install --user -y flathub com.obsproject.Studio",
                    "uninstall_command": "flatpak uninstall --user -y com.obsproject.Studio"
                }
            }
        })
    };

    let blender = AppMetadata {
        id: "blender".to_string(),
        name: "Blender 3D".to_string(),
        description: "Blender is the free and open source 3D creation suite. It supports the entirety of the 3D pipeline—modeling, rigging, animation, simulation, rendering, compositing and motion tracking, video editing and 2D animation pipeline.".to_string(),
        version: "4.1.1".to_string(),
        developer: "Blender Foundation".to_string(),
        website: "https://www.blender.org".to_string(),
        github: "https://github.com/blender/blender".to_string(),
        license: "GPLv3".to_string(),
        category: "Multimedia".to_string(),
        icon_url: "https://www.blender.org/wp-content/uploads/2020/07/blender_logo_stacked.png".to_string(),
        screenshots: vec![
            "https://www.blender.org/wp-content/uploads/2024/03/blender_4_1_splash.jpg".to_string()
        ],
        download_size: "315 MB".to_string(),
        installed_size: "950 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Cycles GPU rendering enhancements and geometry nodes improvements.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "BlenderFoundation.Blender",
                    "install_command": "winget install BlenderFoundation.Blender --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall BlenderFoundation.Blender --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "org.blender.Blender",
                    "install_command": "flatpak install --user -y flathub org.blender.Blender",
                    "uninstall_command": "flatpak uninstall --user -y org.blender.Blender"
                }
            }
        })
    };

    let discord = AppMetadata {
        id: "discord".to_string(),
        name: "Discord".to_string(),
        description: "Discord is the easiest way to talk over voice, video, and text. Talk, chat, hang out, and stay close with your friends and communities.".to_string(),
        version: "1.0.9002".to_string(),
        developer: "Discord Inc.".to_string(),
        website: "https://discord.com".to_string(),
        github: "https://github.com/discord".to_string(),
        license: "Proprietary".to_string(),
        category: "Productivity".to_string(),
        icon_url: "https://assets-global.website-files.com/6257adef93867e50d84d30e2/636e0a6a49cf127bf92de1e2_icon_clyde_blurple_RGB.svg".to_string(),
        screenshots: vec![
            "https://discord.com/assets/8a9d187ccafaf4e930f72023d5ec6866.svg".to_string()
        ],
        download_size: "85 MB".to_string(),
        installed_size: "220 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Performance optimizations, overlay improvements.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Discord.Discord",
                    "install_command": "winget install Discord.Discord --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Discord.Discord --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "com.discordapp.Discord",
                    "install_command": "flatpak install --user -y flathub com.discordapp.Discord",
                    "uninstall_command": "flatpak uninstall --user -y com.discordapp.Discord"
                }
            }
        })
    };

    let vlc = AppMetadata {
        id: "vlc".to_string(),
        name: "VLC Media Player".to_string(),
        description: "VLC is a free and open source cross-platform multimedia player and framework that plays most multimedia files as well as DVDs, Audio CDs, VCDs, and various streaming protocols.".to_string(),
        version: "3.0.20".to_string(),
        developer: "VideoLAN".to_string(),
        website: "https://www.videolan.org".to_string(),
        github: "https://github.com/videolan/vlc".to_string(),
        license: "GPLv2".to_string(),
        category: "Multimedia".to_string(),
        icon_url: "https://images.videolan.org/vlc/images/vlc-cone.svg".to_string(),
        screenshots: vec![
            "https://images.videolan.org/vlc/screenshots/vlc-3.0.jpg".to_string()
        ],
        download_size: "40 MB".to_string(),
        installed_size: "150 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 18.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Fixed playback hardware acceleration bug and audio filters.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "VideoLAN.VLC",
                    "install_command": "winget install VideoLAN.VLC --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall VideoLAN.VLC --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "org.videolan.VLC",
                    "install_command": "flatpak install --user -y flathub org.videolan.VLC",
                    "uninstall_command": "flatpak uninstall --user -y org.videolan.VLC"
                }
            }
        })
    };

    let spotify = AppMetadata {
        id: "spotify".to_string(),
        name: "Spotify".to_string(),
        description: "Spotify is a digital music, podcast, and video service that gives you access to millions of songs and other content from creators all over the world.".to_string(),
        version: "1.2.40.599".to_string(),
        developer: "Spotify AB".to_string(),
        website: "https://spotify.com".to_string(),
        github: "https://github.com/spotify".to_string(),
        license: "Proprietary".to_string(),
        category: "Multimedia".to_string(),
        icon_url: "https://www.scdn.co/co/artist/spotify-logo.png".to_string(),
        screenshots: vec![
            "https://www.spotify.com/us/og-image.png".to_string()
        ],
        download_size: "80 MB".to_string(),
        installed_size: "190 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("UI updates and minor bug fixes.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Spotify.Spotify",
                    "install_command": "winget install Spotify.Spotify --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Spotify.Spotify --silent"
                }
            },
            "linux": {
                "default_manager": "flatpak",
                "flatpak": {
                    "id": "com.spotify.Client",
                    "install_command": "flatpak install --user -y flathub com.spotify.Client",
                    "uninstall_command": "flatpak uninstall --user -y com.spotify.Client"
                }
            }
        })
    };

    let docker = AppMetadata {
        id: "docker".to_string(),
        name: "Docker Desktop".to_string(),
        description: "Docker Desktop is an easy-to-install application for your Mac, Windows or Linux environment that enables you to build and share containerized applications and microservices.".to_string(),
        version: "4.31.0".to_string(),
        developer: "Docker Inc.".to_string(),
        website: "https://www.docker.com".to_string(),
        github: "https://github.com/docker/docker-desktop".to_string(),
        license: "Proprietary (Free for Personal Use)".to_string(),
        category: "Developer Tools".to_string(),
        icon_url: "https://www.docker.com/wp-content/uploads/2023/05/symbol-blue-docker-logo.png".to_string(),
        screenshots: vec![
            "https://www.docker.com/wp-content/uploads/2021/11/Docker-Desktop-Dashboard.png".to_string()
        ],
        download_size: "550 MB".to_string(),
        installed_size: "1.2 GB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "11",
            "linux": "Ubuntu 22.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string()]),
        change_logs: Some("Added support for Docker Scout and WSL2 backend optimization.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Docker.DockerDesktop",
                    "install_command": "winget install Docker.DockerDesktop --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Docker.DockerDesktop --silent"
                }
            },
            "linux": {
                "default_manager": "apt",
                "apt": {
                    "id": "docker-desktop",
                    "install_command": "pkexec apt-get update && pkexec apt-get install -y docker-desktop"
                }
            }
        })
    };

    let python = AppMetadata {
        id: "python".to_string(),
        name: "Python Development Environment".to_string(),
        description: "Python is a programming language that lets you work quickly and integrate systems more effectively. This package configures standard Python, pip, and core packages.".to_string(),
        version: "3.12.3".to_string(),
        developer: "Python Software Foundation".to_string(),
        website: "https://www.python.org".to_string(),
        github: "https://github.com/python/cpython".to_string(),
        license: "PSF License".to_string(),
        category: "Developer Tools".to_string(),
        icon_url: "https://www.python.org/static/community_logos/python-logo-only.png".to_string(),
        screenshots: vec![
            "https://www.python.org/static/img/python-logo.png".to_string()
        ],
        download_size: "25 MB".to_string(),
        installed_size: "90 MB".to_string(),
        dependencies: vec![],
        min_os: serde_json::json!({
            "windows": "10",
            "linux": "Ubuntu 20.04"
        }),
        supported_architectures: Some(vec!["x86_64".to_string(), "arm64".to_string()]),
        change_logs: Some("Standard security updates and performance improvements.".to_string()),
        platform_data: serde_json::json!({
            "windows": {
                "default_manager": "winget",
                "winget": {
                    "id": "Python.Python.3.12",
                    "install_command": "winget install Python.Python.3.12 --silent --accept-source-agreements --accept-package-agreements",
                    "uninstall_command": "winget uninstall Python.Python.3.12 --silent"
                }
            },
            "linux": {
                "default_manager": "apt",
                "apt": {
                    "id": "python3",
                    "install_command": "pkexec apt-get update && pkexec apt-get install -y python3 python3-pip"
                }
            }
        })
    };

    vec![vscode, brave, gimp, obs, blender, discord, vlc, spotify, docker, python]
}

pub async fn sync_catalog_impl(db_path: PathBuf, github_repo: String) -> Result<usize, String> {
    // 1. Set up client with User-Agent header (required by GitHub API)
    let client = reqwest::Client::builder()
        .user_agent("agneax-store-desktop-client/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
        
    // 2. Open connection to cache database
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Database connection error: {}", e))?;
        
    // 3. Try to download index file (apps.json) first for efficiency
    let index_url = format!("https://raw.githubusercontent.com/{}/main/apps.json", github_repo);
    let mut fetched_apps: Vec<AppMetadata> = Vec::new();
    let mut success = false;
    
    if let Ok(res) = client.get(&index_url).send().await {
        if res.status().is_success() {
            if let Ok(apps) = res.json::<Vec<AppMetadata>>().await {
                fetched_apps = apps;
                success = true;
                println!("[SYNC] Successfully synced using consolidated apps.json");
            }
        }
    }
    
    // 4. Fall back to crawling the apps/ directory using GitHub Contents API
    if !success {
        let contents_url = format!("https://api.github.com/repos/{}/contents/apps", github_repo);
        if let Ok(res) = client.get(&contents_url).send().await {
            if res.status().is_success() {
                if let Ok(items) = res.json::<Vec<GithubContentItem>>().await {
                    println!("[SYNC] Found {} files in /apps. Syncing individually...", items.len());
                    for item in items {
                        if item.item_type == "file" && item.name.ends_with(".json") {
                            if let Some(dl_url) = item.download_url {
                                if let Ok(res_item) = client.get(&dl_url).send().await {
                                    if let Ok(app_meta) = res_item.json::<AppMetadata>().await {
                                        fetched_apps.push(app_meta);
                                    }
                                }
                            }
                        }
                    }
                    if !fetched_apps.is_empty() {
                        success = true;
                    }
                }
            }
        }
    }
    
    // 5. If both failed, use local bundled fallback apps catalog
    if !success || fetched_apps.is_empty() {
        println!("[SYNC] Sync failed. Falling back to pre-bundled local app catalog.");
        fetched_apps = get_bundled_apps();
    }
    
    // 6. Write the fetched metadata cache to SQLite database
    for app in fetched_apps.iter() {
        let screenshots_json = serde_json::to_string(&app.screenshots).unwrap_or_else(|_| "[]".to_string());
        let dependencies_json = serde_json::to_string(&app.dependencies).unwrap_or_else(|_| "[]".to_string());
        let min_os_json = serde_json::to_string(&app.min_os).unwrap_or_else(|_| "{}".to_string());
        let architectures_json = serde_json::to_string(&app.supported_architectures).unwrap_or_else(|_| "[]".to_string());
        let platform_data_json = serde_json::to_string(&app.platform_data).unwrap_or_else(|_| "{}".to_string());
        
        let _ = conn.execute(
            "INSERT OR REPLACE INTO cached_apps (id, name, description, version, developer, website, github, license, category, icon_url, screenshots, download_size, installed_size, dependencies, min_os, supported_architectures, change_logs, platform_data, last_synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                app.id,
                app.name,
                app.description,
                app.version,
                app.developer,
                app.website,
                app.github,
                app.license,
                app.category,
                app.icon_url,
                screenshots_json,
                app.download_size,
                app.installed_size,
                dependencies_json,
                min_os_json,
                architectures_json,
                app.change_logs,
                platform_data_json,
                now
            ],
        );
    }
    
    Ok(fetched_apps.len())
}// Background Sync Worker Loop
pub fn start_background_sync(db_path: PathBuf) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[SYNC-WORKER] Failed to create background tokio runtime: {}", e);
                return;
            }
        };
        
        rt.block_on(async move {
            loop {
                // Retrieve synchronization repository from settings
                let repo = {
                    if let Ok(conn) = Connection::open(&db_path) {
                        conn.query_row::<String, _, _>(
                            "SELECT value FROM settings WHERE key = 'github_repo'",
                            [],
                            |row| row.get(0),
                        ).unwrap_or_else(|_| "agneax/store-repo".to_string())
                    } else {
                        "agneax/store-repo".to_string()
                    }
                };
                
                println!("[SYNC-WORKER] Launching background catalog sync for repo: {}", repo);
                let _ = sync_catalog_impl(db_path.clone(), repo).await;
                
                // Sleep for 1 hour before next sync
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    });
}

// Tauri commands
#[tauri::command]
pub async fn sync_catalog(app: AppHandle) -> Result<usize, String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let repo: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'github_repo'",
        [],
        |row| row.get(0),
    ).unwrap_or_else(|_| "agneax/store-repo".to_string());
    
    let count = sync_catalog_impl(db_path, repo).await?;
    let _ = app.emit("catalog-synced", count);
    Ok(count)
}

#[tauri::command]
pub fn get_cached_apps() -> Result<Vec<AppMetadata>, String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(
        "SELECT id, name, description, version, developer, website, github, license, category, icon_url, screenshots, download_size, installed_size, dependencies, min_os, supported_architectures, change_logs, platform_data FROM cached_apps"
    ).map_err(|e| e.to_string())?;
    
    let apps_iter = stmt.query_map([], |row| {
        let screenshots_str: String = row.get(10)?;
        let dependencies_str: String = row.get(13)?;
        let min_os_str: String = row.get(14)?;
        let archs_str: Option<String> = row.get::<_, Option<String>>(15).unwrap_or(None);
        let platform_data_str: String = row.get(17)?;
        
        let screenshots: Vec<String> = serde_json::from_str(&screenshots_str).unwrap_or_default();
        let dependencies: Vec<String> = serde_json::from_str(&dependencies_str).unwrap_or_default();
        let min_os: serde_json::Value = serde_json::from_str(&min_os_str).unwrap_or(serde_json::json!({}));
        let supported_architectures: Option<Vec<String>> = archs_str.and_then(|s| serde_json::from_str(&s).ok());
        let platform_data: serde_json::Value = serde_json::from_str(&platform_data_str).unwrap_or(serde_json::json!({}));
        
        Ok(AppMetadata {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            version: row.get(3)?,
            developer: row.get(4)?,
            website: row.get(5)?,
            github: row.get(6)?,
            license: row.get(7)?,
            category: row.get(8)?,
            icon_url: row.get(9)?,
            screenshots,
            download_size: row.get(11)?,
            installed_size: row.get(12)?,
            dependencies,
            min_os,
            supported_architectures,
            change_logs: row.get(16)?,
            platform_data,
        })
    }).map_err(|e| e.to_string())?;
    
    let apps: Vec<AppMetadata> = apps_iter.filter_map(Result::ok).collect();
    Ok(apps)
}

#[tauri::command]
pub fn get_app_details(id: String) -> Result<AppMetadata, String> {
    let db_path = crate::db::get_db_path();
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let app = conn.query_row(
        "SELECT id, name, description, version, developer, website, github, license, category, icon_url, screenshots, download_size, installed_size, dependencies, min_os, supported_architectures, change_logs, platform_data 
         FROM cached_apps WHERE id = ?1",
        params![id],
        |row| {
            let screenshots_str: String = row.get(10)?;
            let dependencies_str: String = row.get(13)?;
            let min_os_str: String = row.get(14)?;
            let archs_str: Option<String> = row.get::<_, Option<String>>(15).unwrap_or(None);
            let platform_data_str: String = row.get(17)?;
            
            let screenshots: Vec<String> = serde_json::from_str(&screenshots_str).unwrap_or_default();
            let dependencies: Vec<String> = serde_json::from_str(&dependencies_str).unwrap_or_default();
            let min_os: serde_json::Value = serde_json::from_str(&min_os_str).unwrap_or(serde_json::json!({}));
            let supported_architectures: Option<Vec<String>> = archs_str.and_then(|s| serde_json::from_str(&s).ok());
            let platform_data: serde_json::Value = serde_json::from_str(&platform_data_str).unwrap_or(serde_json::json!({}));
            
            Ok(AppMetadata {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                developer: row.get(4)?,
                website: row.get(5)?,
                github: row.get(6)?,
                license: row.get(7)?,
                category: row.get(8)?,
                icon_url: row.get(9)?,
                screenshots,
                download_size: row.get(11)?,
                installed_size: row.get(12)?,
                dependencies,
                min_os,
                supported_architectures,
                change_logs: row.get(16)?,
                platform_data,
            })
        }
    ).map_err(|e| format!("Application not found in database cache: {}", e))?;
    
    Ok(app)
}
