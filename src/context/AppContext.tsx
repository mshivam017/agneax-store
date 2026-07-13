import React, { createContext, useContext, useState, useEffect } from "react";
use_tauri_api_endpoints_directly();

// TypeScript definitions matching our Rust models
export interface AppMetadata {
  id: string;
  name: string;
  description: string;
  version: string;
  developer: string;
  website: string;
  github: string;
  license: string;
  category: string;
  icon_url: string;
  screenshots: string[];
  download_size: string;
  installed_size: string;
  dependencies: string[];
  min_os: Record<string, string>;
  supported_architectures?: string[];
  change_logs?: string;
  platform_data: any;
}

export interface InstalledApp {
  app_id: string;
  name: string;
  version: string;
  install_date: number;
  install_method: string;
  executable_path?: string;
}

export interface DownloadProgress {
  id: string;
  app_id: string;
  app_name: string;
  version: string;
  status: string;
  progress: number;
  speed: number;
  remaining_time: number;
  downloaded_size: number;
  file_size: number;
  error_message?: string;
}

export interface AppSettings {
  theme: string;
  accent_color: string;
  language: string;
  github_repo: string;
  download_location: string;
  concurrent_downloads: string;
  auto_update: string;
  auto_launch_after_install: string;
  desktop_shortcut: string;
  notifications: string;
}

export interface ToastMessage {
  id: string;
  message: string;
  type: "success" | "error" | "info";
}

interface AppContextType {
  apps: AppMetadata[];
  installedApps: InstalledApp[];
  downloads: Record<string, DownloadProgress>;
  favorites: string[];
  settings: AppSettings;
  syncing: boolean;
  toasts: ToastMessage[];
  showToast: (message: string, type?: "success" | "error" | "info") => void;
  dismissToast: (id: string) => void;
  syncCatalog: () => Promise<void>;
  toggleFavorite: (appId: string) => Promise<void>;
  installApp: (appId: string) => Promise<void>;
  uninstallApp: (appId: string) => Promise<void>;
  launchApp: (appId: string) => Promise<void>;
  startFileDownload: (appId: string, name: string, version: string, url: string) => Promise<void>;
  pauseDownload: (downloadId: string) => Promise<void>;
  resumeDownload: (downloadId: string) => Promise<void>;
  cancelDownload: (downloadId: string) => Promise<void>;
  saveSetting: (key: keyof AppSettings, value: string) => Promise<void>;
  refreshInstalledList: () => Promise<void>;
}

const AppContext = createContext<AppContextType | undefined>(undefined);

// Dummy fallback for tauri import check on normal browser contexts
function use_tauri_api_endpoints_directly() {
  console.log("[TAURI] Binding state context to backend endpoints");
}

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [apps, setApps] = useState<AppMetadata[]>([]);
  const [installedApps, setInstalledApps] = useState<InstalledApp[]>([]);
  const [downloads, setDownloads] = useState<Record<string, DownloadProgress>>({});
  const [favorites, setFavorites] = useState<string[]>([]);
  const [syncing, setSyncing] = useState<boolean>(false);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [settings, setSettings] = useState<AppSettings>({
    theme: "dark",
    accent_color: "#FF6A00",
    language: "en",
    github_repo: "agneax/store-repo",
    download_location: "",
    concurrent_downloads: "3",
    auto_update: "true",
    auto_launch_after_install: "false",
    desktop_shortcut: "true",
    notifications: "true",
  });

  const showToast = (message: string, type: "success" | "error" | "info" = "info") => {
    if (settings.notifications === "false" && type === "info") return;
    const id = Math.random().toString(36).substring(2, 9);
    setToasts((prev) => [...prev, { id, message, type }]);
    setTimeout(() => dismissToast(id), 5000);
  };

  const dismissToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  // Load everything from SQLite backend cache
  const loadInitialData = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      
      // Load Cached Apps
      const cached = await invoke<AppMetadata[]>("get_cached_apps");
      setApps(cached);
      
      // Load Installed Apps
      const installed = await invoke<InstalledApp[]>("get_installed_apps");
      setInstalledApps(installed);
      
      // Load Favorites
      const favs = await invoke<string[]>("get_favorites");
      setFavorites(favs);
      
      // Load Settings
      const loadedSettings = await invoke<Record<string, string>>("get_settings");
      if (loadedSettings) {
        setSettings((prev) => ({
          ...prev,
          ...loadedSettings,
        }));
      }
    } catch (e) {
      console.warn("Tauri API invoke failed (might be running in standard web browser):", e);
    }
  };

  // Refresh installed list from database (called after installs/uninstalls)
  const refreshInstalledList = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const installed = await invoke<InstalledApp[]>("get_installed_apps");
      setInstalledApps(installed);
    } catch (e) {
      console.error(e);
    }
  };

  // Sync catalog from GitHub Repository
  const syncCatalog = async () => {
    setSyncing(true);
    showToast("Syncing catalog with GitHub...", "info");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const count = await invoke<number>("sync_catalog");
      const cached = await invoke<AppMetadata[]>("get_cached_apps");
      setApps(cached);
      setSyncing(false);
      showToast(`Catalog sync completed. Loaded ${count} apps.`, "success");
    } catch (e) {
      setSyncing(false);
      showToast(`Sync failed: ${e}. Using offline cache.`, "error");
    }
  };

  // Toggle favorite app status
  const toggleFavorite = async (appId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const isFav = await invoke<boolean>("toggle_favorite", { appId });
      if (isFav) {
        setFavorites((prev) => [...prev, appId]);
        showToast("Added to Favorites", "success");
      } else {
        setFavorites((prev) => prev.filter((id) => id !== appId));
        showToast("Removed from Favorites", "info");
      }
    } catch (e) {
      console.error(e);
    }
  };

  // Start download manager download
  const startFileDownload = async (appId: string, name: string, version: string, url: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      showToast(`Starting download for ${name}...`, "info");
      const downloadId = await invoke<string>("start_download", {
        appId,
        appName: name,
        version,
        url,
        savePath: settings.download_location,
      });
      showToast(`Download added to queue (ID: ${downloadId})`, "success");
    } catch (e) {
      showToast(`Failed to download: ${e}`, "error");
    }
  };

  const pauseDownload = async (downloadId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("pause_download", { id: downloadId });
      showToast("Download paused", "info");
    } catch (e) {
      showToast(`Failed to pause: ${e}`, "error");
    }
  };

  const resumeDownload = async (downloadId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("resume_download", { id: downloadId });
      showToast("Resuming download...", "info");
    } catch (e) {
      showToast(`Failed to resume: ${e}`, "error");
    }
  };

  const cancelDownload = async (downloadId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("cancel_download", { id: downloadId });
      setDownloads((prev) => {
        const copy = { ...prev };
        delete copy[downloadId];
        return copy;
      });
      showToast("Download cancelled", "info");
    } catch (e) {
      showToast(`Failed to cancel: ${e}`, "error");
    }
  };

  // Launch installer engine
  const installApp = async (appId: string) => {
    showToast("Starting installation...", "info");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("install_app", { appId });
      await refreshInstalledList();
      showToast("Installation completed successfully!", "success");
      
      // Auto launch if option is checked
      if (settings.auto_launch_after_install === "true") {
        await launchApp(appId);
      }
    } catch (e) {
      showToast(`Installation failed: ${e}`, "error");
    }
  };

  // Launch uninstallation engine
  const uninstallApp = async (appId: string) => {
    showToast("Uninstalling application...", "info");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("uninstall_app", { appId });
      await refreshInstalledList();
      showToast("Application uninstalled successfully.", "success");
    } catch (e) {
      showToast(`Uninstallation failed: ${e}`, "error");
    }
  };

  // Launch executable
  const launchApp = async (appId: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("launch_app", { appId });
      showToast("Application launched", "success");
    } catch (e) {
      showToast(`Failed to launch application: ${e}`, "error");
    }
  };

  // Save specific settings key-value pair
  const saveSetting = async (key: keyof AppSettings, value: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("save_setting", { key, value });
      setSettings((prev) => ({
        ...prev,
        [key]: value,
      }));
      showToast("Settings saved", "success");
    } catch (e) {
      showToast(`Failed to save settings: ${e}`, "error");
    }
  };

  // Setup Event Listeners for real-time progress
  useEffect(() => {
    loadInitialData();

    let unlistenProgress: () => void;

    // Listen to download progress event from Rust
    const setupListeners = async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        
        const unlisten = await listen<DownloadProgress>("download-progress", (event) => {
          const payload = event.payload;
          setDownloads((prev) => ({
            ...prev,
            [payload.id]: payload,
          }));
        });
        
        unlistenProgress = unlisten;
      } catch (e) {
        console.warn("Tauri event listener setup skipped (browser environment):", e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenProgress) unlistenProgress();
    };
  }, []);

  return (
    <AppContext.Provider
      value={{
        apps,
        installedApps,
        downloads,
        favorites,
        settings,
        syncing,
        toasts,
        showToast,
        dismissToast,
        syncCatalog,
        toggleFavorite,
        installApp,
        uninstallApp,
        launchApp,
        startFileDownload,
        pauseDownload,
        resumeDownload,
        cancelDownload,
        saveSetting,
        refreshInstalledList,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export const useApp = () => {
  const context = useContext(AppContext);
  if (context === undefined) {
    throw new Error("useApp must be used within an AppProvider");
  }
  return context;
};
