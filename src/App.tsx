import React, { useState, useEffect } from "react";
import { AppProvider, useApp, AppMetadata } from "./context/AppContext";
import { Icons } from "./components/Icons";

// --- CUSTOM HEADER TITLEBAR ---
const Titlebar: React.FC<{ activePage: string; onBack: () => void; canGoBack: boolean }> = ({
  activePage,
  onBack,
  canGoBack,
}) => {
  const handleMinimize = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("minimize_window");
    } catch (e) {
      console.log("Minimize clicked");
    }
  };

  const handleMaximize = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("toggle_maximize_window");
    } catch (e) {
      console.log("Maximize clicked");
    }
  };

  const handleClose = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("close_window");
    } catch (e) {
      console.log("Close clicked");
    }
  };

  return (
    <header className="custom-titlebar">
      <div className="titlebar-drag-region" data-tauri-drag-region>
        {canGoBack && (
          <button className="titlebar-button" onClick={onBack} title="Back">
            <Icons.ChevronLeft size={16} />
          </button>
        )}
        <div className="titlebar-logo">
          <Icons.Downloads size={18} style={{ color: "var(--primary)" }} />
          <span>Agneax Store {activePage ? `| ${activePage.toUpperCase()}` : ""}</span>
        </div>
      </div>
      <div className="titlebar-controls">
        <button className="titlebar-button" onClick={handleMinimize} title="Minimize">
          &minus;
        </button>
        <button className="titlebar-button" onClick={handleMaximize} title="Maximize">
          &#9634;
        </button>
        <button className="titlebar-button close" onClick={handleClose} title="Close">
          &times;
        </button>
      </div>
    </header>
  );
};

// --- APP CARD COMPONENT ---
const AppCard: React.FC<{ app: AppMetadata; onClick: () => void }> = ({ app, onClick }) => {
  const { installedApps, installApp, launchApp, downloads } = useApp();
  
  const activeDl = Object.values(downloads).find(
    (dl) => dl.app_id === app.id && (dl.status === "Downloading" || dl.status === "Pending" || dl.status === "Paused")
  );
  const isInstalled = installedApps.some((ia) => ia.app_id === app.id);

  const handleActionClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isInstalled) {
      launchApp(app.id);
    } else {
      installApp(app.id);
    }
  };

  return (
    <div className="app-card" onClick={onClick}>
      <div className="app-card-header">
        <div className="app-card-icon">
          <img src={app.icon_url} alt={app.name} onError={(e) => {
            (e.target as HTMLImageElement).src = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect width='18' height='18' x='3' y='3' rx='2'/><path d='M21 9H3M21 15H3M12 3v18'/></svg>";
          }} />
        </div>
        <div className="app-card-info">
          <h4 className="app-card-name" title={app.name}>{app.name}</h4>
          <span className="app-card-dev">{app.developer}</span>
        </div>
      </div>
      <p className="app-card-desc">{app.description}</p>
      <div className="app-card-footer">
        <div className="app-card-meta">
          <span className="app-card-category">{app.category}</span>
          <span className="app-card-price">{app.license}</span>
        </div>
        <button className="btn-card-action" onClick={handleActionClick} disabled={!!activeDl} style={activeDl ? { opacity: 0.8, cursor: "not-allowed" } : {}}>
          {isInstalled ? "Launch" : activeDl ? `${activeDl.progress.toFixed(0)}%` : "Install"}
        </button>
      </div>
    </div>
  );
};

// --- APP DETAILS MODAL DIALOG ---
const AppDetailModal: React.FC<{ app: AppMetadata; onClose: () => void }> = ({ app, onClose }) => {
  const { installedApps, favorites, toggleFavorite, installApp, uninstallApp, launchApp, showToast, downloads } = useApp();
  
  const activeDl = Object.values(downloads).find(
    (dl) => dl.app_id === app.id && (dl.status === "Downloading" || dl.status === "Pending" || dl.status === "Paused")
  );
  const [logs, setLogs] = useState<any[]>([]);
  const isInstalled = installedApps.some((ia) => ia.app_id === app.id);
  const installedVersion = installedApps.find((ia) => ia.app_id === app.id)?.version;
  const isFav = favorites.includes(app.id);

  // Poll database logs for real-time installer feedback
  useEffect(() => {
    const fetchLogs = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const logData = await invoke<any[]>("get_app_logs", { appId: app.id });
        setLogs(logData);
      } catch (e) {
        console.log("Error querying logs");
      }
    };
    
    fetchLogs();
    const interval = setInterval(fetchLogs, 2000);
    return () => clearInterval(interval);
  }, [app.id]);

  const handleExportLogs = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const path = await invoke<string | null>("export_app_logs_interactive", { appId: app.id });
      if (path) {
        showToast(`Logs exported to ${path}`, "success");
      }
    } catch (e) {
      showToast(`Logs export failed: ${e}`, "error");
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close-btn" onClick={onClose}>&times;</button>
        
        <div className="modal-body">
          <div className="app-detail-header">
            <div className="app-detail-icon">
              <img src={app.icon_url} alt={app.name} onError={(e) => {
                (e.target as HTMLImageElement).src = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect width='18' height='18' x='3' y='3' rx='2'/><path d='M21 9H3M21 15H3M12 3v18'/></svg>";
              }} />
            </div>
            <div className="app-detail-info">
              <h2 className="app-detail-name">{app.name}</h2>
              <span className="app-detail-dev">{app.developer}</span>
              
              <div className="app-detail-actions">
                {isInstalled ? (
                  <>
                    <button className="btn-primary" onClick={() => launchApp(app.id)}>Launch</button>
                    <button className="btn-secondary" onClick={() => uninstallApp(app.id)}>Uninstall</button>
                  </>
                ) : activeDl ? (
                  <button className="btn-primary" disabled style={{ opacity: 0.8, cursor: "not-allowed", display: "flex", alignItems: "center", gap: "8px" }}>
                    {activeDl.status === "Paused" ? `Paused (${activeDl.progress.toFixed(0)}%)` : `Downloading (${activeDl.progress.toFixed(0)}%)`}
                    <div className="spinner"></div>
                  </button>
                ) : (
                  <button className="btn-primary" onClick={() => installApp(app.id)}>Install</button>
                )}
                
                <button className={`btn-favorite ${isFav ? "active" : ""}`} onClick={() => toggleFavorite(app.id)}>
                  {isFav ? <Icons.HeartFilled size={20} /> : <Icons.Heart size={20} />}
                </button>
                
                <a href={app.website} target="_blank" rel="noreferrer" className="btn-secondary" style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  Website <Icons.ExternalLink size={14} />
                </a>
                
                <a href={app.github} target="_blank" rel="noreferrer" className="btn-secondary" style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  GitHub <Icons.Github size={14} />
                </a>
              </div>
            </div>
          </div>
          
          {app.screenshots && app.screenshots.length > 0 && (
            <div className="screenshots-carousel">
              {app.screenshots.map((s, idx) => (
                <div className="screenshot-item" key={idx}>
                  <img src={s} alt={`${app.name} screenshot ${idx + 1}`} onError={(e) => {
                    (e.target as HTMLImageElement).src = "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=600&auto=format&fit=crop";
                  }} />
                </div>
              ))}
            </div>
          )}
          
          <div className="app-detail-split">
            <div className="app-detail-left">
              <div>
                <h3 className="detail-section-title">Description</h3>
                <p className="detail-desc">{app.description}</p>
              </div>
              
              {app.change_logs && (
                <div>
                  <h3 className="detail-section-title">Release Notes</h3>
                  <p className="detail-desc">{app.change_logs}</p>
                </div>
              )}
              
              {logs.length > 0 && (
                <div>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
                    <h3 className="detail-section-title" style={{ margin: 0 }}>Installation Logs</h3>
                    <button className="btn-card-action" onClick={handleExportLogs}>Export Logs</button>
                  </div>
                  <div style={{ background: "rgba(0,0,0,0.3)", borderRadius: "8px", padding: "16px", maxHeight: "150px", overflowY: "auto", fontFamily: "monospace", fontSize: "0.8rem", border: "1px solid var(--border)" }}>
                    {logs.map((log) => (
                      <div key={log.id} style={{ color: log.status === "Error" ? "#E81123" : log.status === "Success" ? "#00CC66" : "var(--text-secondary)", marginBottom: "4px" }}>
                        [{log.step}] {log.message}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
            
            <div className="app-detail-right">
              <h3 className="detail-section-title" style={{ fontSize: "1rem" }}>Specifications</h3>
              <div className="spec-list">
                <div className="spec-item">
                  <span className="spec-label">Version</span>
                  <span className="spec-val">{app.version}</span>
                </div>
                {isInstalled && (
                  <div className="spec-item">
                    <span className="spec-label">Installed Version</span>
                    <span className="spec-val" style={{ color: "var(--primary)", fontWeight: "bold" }}>{installedVersion}</span>
                  </div>
                )}
                <div className="spec-item">
                  <span className="spec-label">Category</span>
                  <span className="spec-val">{app.category}</span>
                </div>
                <div className="spec-item">
                  <span className="spec-label">License</span>
                  <span className="spec-val">{app.license}</span>
                </div>
                <div className="spec-item">
                  <span className="spec-label">Download Size</span>
                  <span className="spec-val">{app.download_size}</span>
                </div>
                <div className="spec-item">
                  <span className="spec-label">Installed Size</span>
                  <span className="spec-val">{app.installed_size}</span>
                </div>
                <div className="spec-item">
                  <span className="spec-label">Min OS requirement</span>
                  <span className="spec-val">Windows {app.min_os?.windows || "10"} / Linux</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// --- PAGES RENDERING ---

// 1. Home Page
const HomePage: React.FC<{ onSelectApp: (app: AppMetadata) => void }> = ({ onSelectApp }) => {
  const { apps, syncCatalog, syncing } = useApp();
  const [activeCategory, setActiveCategory] = useState<string>("All");
  
  const categories = ["All", "Gaming", "Developer Tools", "AI Tools", "Utilities", "Productivity", "Multimedia"];
  
  const filteredApps = activeCategory === "All"
    ? apps
    : apps.filter((app) => app.category.toLowerCase() === activeCategory.toLowerCase());

  const featuredApp = apps.find(a => a.id === "vscode") || apps[0];

  return (
    <div className="page-container">
      {featuredApp && (
        <div className="hero-banner" onClick={() => onSelectApp(featuredApp)} style={{ cursor: "pointer" }}>
          <span className="hero-tag">Featured App</span>
          <h2 className="hero-title">{featuredApp.name}</h2>
          <p className="hero-desc">{featuredApp.description}</p>
        </div>
      )}

      <div className="section-container">
        <div className="section-header">
          <h3 className="section-title">Categories</h3>
          <button className={`sync-badge ${syncing ? "syncing" : ""}`} onClick={syncCatalog} disabled={syncing}>
            {syncing ? <Icons.Spinner size={14} /> : <Icons.Refresh size={14} />}
            {syncing ? "Syncing..." : "Sync Catalog"}
          </button>
        </div>
        <div className="categories-row">
          {categories.map((c) => (
            <button key={c} className={`category-pill ${activeCategory === c ? "active" : ""}`} onClick={() => setActiveCategory(c)}>
              {c}
            </button>
          ))}
        </div>
      </div>

      <div className="section-container">
        <h3 className="section-title">Explore Applications</h3>
        {filteredApps.length === 0 ? (
          <p style={{ color: "var(--text-secondary)", fontStyle: "italic", padding: "16px 0" }}>No apps found in this category.</p>
        ) : (
          <div className="apps-grid">
            {filteredApps.map((app) => (
              <AppCard key={app.id} app={app} onClick={() => onSelectApp(app)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

// 2. Categories Page
const CategoriesPage: React.FC<{ onSelectApp: (app: AppMetadata) => void }> = ({ onSelectApp }) => {
  const { apps } = useApp();
  const categoryGroups = ["Developer Tools", "Utilities", "Productivity", "Multimedia", "Gaming", "AI Tools"];

  return (
    <div className="page-container">
      <h2 style={{ fontSize: "1.5rem", fontWeight: 700 }}>Software Categories</h2>
      
      {categoryGroups.map((cat) => {
        const catApps = apps.filter((app) => app.category.toLowerCase() === cat.toLowerCase());
        if (catApps.length === 0) return null;
        return (
          <div className="section-container" key={cat} style={{ marginBottom: "20px" }}>
            <h3 className="section-title">{cat}</h3>
            <div className="apps-grid">
              {catApps.slice(0, 4).map((app) => (
                <AppCard key={app.id} app={app} onClick={() => onSelectApp(app)} />
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
};

// 3. Search Page
const SearchPage: React.FC<{ onSelectApp: (app: AppMetadata) => void }> = ({ onSelectApp }) => {
  const { apps } = useApp();
  const [query, setQuery] = useState("");
  const [filterCategory, setFilterCategory] = useState("All");
  const [filterLicense, setFilterLicense] = useState("All");

  const categories = ["All", "Developer Tools", "Utilities", "Productivity", "Multimedia", "Gaming", "AI Tools"];
  const licenses = ["All", "Open Source", "Proprietary"];

  const filteredApps = apps.filter((app) => {
    const matchesQuery = app.name.toLowerCase().includes(query.toLowerCase()) || 
                         app.developer.toLowerCase().includes(query.toLowerCase()) ||
                         app.description.toLowerCase().includes(query.toLowerCase());
    const matchesCategory = filterCategory === "All" || app.category.toLowerCase() === filterCategory.toLowerCase();
    const matchesLicense = filterLicense === "All" || 
      (filterLicense === "Open Source" && app.license.toLowerCase().includes("open source")) ||
      (filterLicense === "Proprietary" && app.license.toLowerCase().includes("proprietary"));
      
    return matchesQuery && matchesCategory && matchesLicense;
  });

  return (
    <div className="page-container">
      <div className="search-container">
        <div className="search-input-wrapper">
          <Icons.Search size={18} className="search-icon-svg" />
          <input className="search-input" type="text" placeholder="Search applications, developers, keywords..." value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
      </div>

      <div className="filter-panel">
        <div className="filter-group">
          <label>Category</label>
          <select className="filter-select" value={filterCategory} onChange={(e) => setFilterCategory(e.target.value)}>
            {categories.map((c) => <option key={c} value={c}>{c}</option>)}
          </select>
        </div>

        <div className="filter-group">
          <label>License</label>
          <select className="filter-select" value={filterLicense} onChange={(e) => setFilterLicense(e.target.value)}>
            {licenses.map((l) => <option key={l} value={l}>{l}</option>)}
          </select>
        </div>
      </div>

      <div className="section-container">
        <h3 className="section-title">Search Results ({filteredApps.length})</h3>
        {filteredApps.length === 0 ? (
          <p style={{ color: "var(--text-secondary)", fontStyle: "italic", padding: "16px 0" }}>No apps match the search filters.</p>
        ) : (
          <div className="apps-grid">
            {filteredApps.map((app) => (
              <AppCard key={app.id} app={app} onClick={() => onSelectApp(app)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

// 4. Installed Page
const InstalledPage: React.FC<{ onSelectApp: (app: AppMetadata) => void }> = ({ onSelectApp }) => {
  const { installedApps, apps, launchApp, uninstallApp } = useApp();

  return (
    <div className="page-container">
      <h2 style={{ fontSize: "1.5rem", fontWeight: 700 }}>Installed Applications</h2>
      
      {installedApps.length === 0 ? (
        <div style={{ textAlign: "center", padding: "64px 0", color: "var(--text-secondary)" }}>
          <Icons.Installed size={48} style={{ color: "var(--text-muted)", marginBottom: "16px" }} />
          <p>No applications installed on this machine via Agneax Store yet.</p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          {installedApps.map((installed) => {
            const appMeta = apps.find((a) => a.id === installed.app_id);
            const dateStr = new Date(installed.install_date * 1000).toLocaleDateString();
            
            return (
              <div key={installed.app_id} className="settings-card" onClick={() => appMeta && onSelectApp(appMeta)} style={{ cursor: "pointer", transition: "var(--transition)" }}>
                <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
                  <div className="app-card-icon" style={{ width: "44px", height: "44px" }}>
                    <img src={appMeta?.icon_url || ""} alt={installed.name} onError={(e) => {
                      (e.target as HTMLImageElement).src = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect width='18' height='18' x='3' y='3' rx='2'/><path d='M21 9H3M21 15H3M12 3v18'/></svg>";
                    }} />
                  </div>
                  <div className="settings-info">
                    <div className="settings-title">{installed.name}</div>
                    <div className="settings-desc">Version: {installed.version} • Installed via {installed.install_method} on {dateStr}</div>
                  </div>
                </div>
                <div className="settings-control" onClick={(e) => e.stopPropagation()} style={{ display: "flex", gap: "12px" }}>
                  <button className="btn-primary" onClick={() => launchApp(installed.app_id)}>Launch</button>
                  <button className="btn-secondary" onClick={() => uninstallApp(installed.app_id)}>Uninstall</button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

// 5. Updates Page
const UpdatesPage: React.FC<{ onSelectApp: (app: AppMetadata) => void }> = ({ onSelectApp }) => {
  const { installedApps, apps, installApp } = useApp();
  
  // Find apps where the installed version is different from the catalog cache version
  const updateAvailableApps = installedApps.filter((ia) => {
    const catalogApp = apps.find((a) => a.id === ia.app_id);
    return catalogApp && catalogApp.version !== ia.version;
  });

  const handleUpdateAll = async () => {
    alert("Starting update for all outdated packages in the background...");
    for (const app of updateAvailableApps) {
      await installApp(app.app_id);
    }
  };

  return (
    <div className="page-container">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h2 style={{ fontSize: "1.5rem", fontWeight: 700 }}>Updates Center</h2>
        {updateAvailableApps.length > 0 && (
          <button className="btn-primary" onClick={handleUpdateAll}>Update All</button>
        )}
      </div>

      {updateAvailableApps.length === 0 ? (
        <div style={{ textAlign: "center", padding: "64px 0", color: "var(--text-secondary)" }}>
          <Icons.Check size={48} style={{ color: "#00CC66", marginBottom: "16px" }} />
          <p style={{ fontWeight: 600, color: "var(--text)" }}>All systems up to date</p>
          <p style={{ fontSize: "0.85rem", marginTop: "4px" }}>Every package on your machine matches the latest GitHub release registry.</p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          {updateAvailableApps.map((installed) => {
            const catalogApp = apps.find((a) => a.id === installed.app_id);
            if (!catalogApp) return null;
            return (
              <div key={installed.app_id} className="settings-card" onClick={() => onSelectApp(catalogApp)} style={{ cursor: "pointer" }}>
                <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
                  <div className="app-card-icon" style={{ width: "44px", height: "44px" }}>
                    <img src={catalogApp.icon_url} alt={installed.name} onError={(e) => {
                      (e.target as HTMLImageElement).src = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect width='18' height='18' x='3' y='3' rx='2'/><path d='M21 9H3M21 15H3M12 3v18'/></svg>";
                    }} />
                  </div>
                  <div className="settings-info">
                    <div className="settings-title">{installed.name}</div>
                    <div className="settings-desc">Installed: {installed.version} • Latest Available: <span style={{ color: "var(--primary)", fontWeight: "bold" }}>{catalogApp.version}</span></div>
                  </div>
                </div>
                <div className="settings-control" onClick={(e) => e.stopPropagation()}>
                  <button className="btn-primary" onClick={() => installApp(installed.app_id)}>Update</button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

// 6. Downloads Page
const DownloadsPage: React.FC = () => {
  const { downloads, pauseDownload, resumeDownload, cancelDownload } = useApp();
  const [history, setHistory] = useState<any[]>([]);

  const activeDownloadsList = Object.values(downloads).filter(
    (dl) => dl.status === "Downloading" || dl.status === "Pending" || dl.status === "Paused"
  );

  useEffect(() => {
    const fetchHistory = async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const data = await invoke<any[]>("get_downloads_history");
        setHistory(data);
      } catch (e) {
        console.log("Error loading download history");
      }
    };
    fetchHistory();
    const interval = setInterval(fetchHistory, 3000);
    return () => clearInterval(interval);
  }, [downloads]);

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return "0 B/s";
    const k = 1024;
    const sizes = ["B/s", "KB/s", "MB/s", "GB/s"];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  return (
    <div className="page-container">
      <h2 style={{ fontSize: "1.5rem", fontWeight: 700 }}>Download Center</h2>
      
      <div className="section-container">
        <h3 className="section-title">Active Queue</h3>
        {activeDownloadsList.length === 0 ? (
          <p style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>No active downloads in queue.</p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
            {activeDownloadsList.map((dl) => (
              <div key={dl.id} className="settings-card" style={{ padding: "20px" }}>
                <div style={{ flexGrow: 1, display: "flex", flexDirection: "column", gap: "8px", marginRight: "24px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontWeight: 600 }}>
                    <span>{dl.app_name} <span style={{ color: "var(--text-secondary)", fontSize: "0.8rem" }}>v{dl.version}</span></span>
                    <span style={{ color: "var(--primary)" }}>{dl.progress.toFixed(1)}%</span>
                  </div>
                  <div className="download-progress-bar-bg">
                    <div className="download-progress-bar-fill" style={{ width: `${dl.progress}%` }}></div>
                  </div>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem", color: "var(--text-secondary)" }}>
                    <span>{formatSize(dl.downloaded_size)} / {formatSize(dl.file_size)}</span>
                    <span>Speed: {formatSpeed(dl.speed)} • ETA: {dl.remaining_time}s</span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "8px" }}>
                  {dl.status === "Paused" ? (
                    <button className="btn-primary" onClick={() => resumeDownload(dl.id)} style={{ padding: "6px 12px", fontSize: "0.8rem" }}>Resume</button>
                  ) : (
                    <button className="btn-secondary" onClick={() => pauseDownload(dl.id)} style={{ padding: "6px 12px", fontSize: "0.8rem" }}>Pause</button>
                  )}
                  <button className="btn-secondary" onClick={() => cancelDownload(dl.id)} style={{ padding: "6px 12px", fontSize: "0.8rem", color: "#E81123" }}>Cancel</button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="section-container">
        <h3 className="section-title">Download History</h3>
        {history.length === 0 ? (
          <p style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>No download history recorded.</p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "10px", maxHeight: "300px", overflowY: "auto" }}>
            {history.map((h) => (
              <div key={h.id} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "12px 16px", background: "rgba(255,255,255,0.01)", border: "1px solid var(--border)", borderRadius: "8px", fontSize: "0.85rem" }}>
                <div>
                  <span style={{ fontWeight: 600 }}>{h.app_name}</span>
                  <span style={{ color: "var(--text-secondary)", marginLeft: "8px" }}>v{h.version}</span>
                </div>
                <div style={{ display: "flex", gap: "20px", alignItems: "center" }}>
                  <span style={{ color: h.status === "Completed" ? "#00CC66" : h.status === "Cancelled" ? "var(--text-secondary)" : "#E81123", fontWeight: 500 }}>
                    {h.status}
                  </span>
                  <span style={{ color: "var(--text-secondary)", fontSize: "0.8rem" }}>{formatSize(h.file_size)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

// 7. Settings Page
const SettingsPage: React.FC = () => {
  const { settings, saveSetting } = useApp();

  const handleSelectDir = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const selected = await invoke<string | null>("pick_download_directory");
      if (selected) {
        await saveSetting("download_location", selected);
      }
    } catch (e) {
      alert("Folder selection failed: " + e);
    }
  };

  return (
    <div className="page-container">
      <h2 style={{ fontSize: "1.5rem", fontWeight: 700 }}>Settings Configuration</h2>
      
      <div className="settings-list">
        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Application Theme Mode</span>
            <span className="settings-desc">Switch between obsidian dark theme and premium slate light theme layouts.</span>
          </div>
          <div className="settings-control">
            <select value={settings.theme} onChange={(e) => saveSetting("theme", e.target.value)}>
              <option value="dark">Dark Theme</option>
              <option value="light">Light Theme</option>
            </select>
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">GitHub Repository Source</span>
            <span className="settings-desc">Specify the target repository owner/repo format for catalog sync data.</span>
          </div>
          <div className="settings-control">
            <input type="text" value={settings.github_repo} onChange={(e) => saveSetting("github_repo", e.target.value)} />
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">App Accent Color Theme</span>
            <span className="settings-desc">Choose a custom primary color theme to customize the store layout.</span>
          </div>
          <div className="settings-control" style={{ display: "flex", gap: "10px", alignItems: "center" }}>
            {[
              { name: "Cyan", hex: "#00E5FF" },
              { name: "Blue", hex: "#2979FF" },
              { name: "Pink", hex: "#FF1744" },
              { name: "Purple", hex: "#D500F9" },
              { name: "Green", hex: "#00E676" },
              { name: "Orange", hex: "#FF9100" }
            ].map((color) => (
              <button
                key={color.hex}
                title={color.name}
                onClick={() => saveSetting("accent_color", color.hex)}
                style={{
                  width: "24px",
                  height: "24px",
                  borderRadius: "50%",
                  border: settings.accent_color === color.hex ? "2px solid #FFFFFF" : "1px solid rgba(255,255,255,0.2)",
                  background: color.hex,
                  cursor: "pointer",
                  boxShadow: settings.accent_color === color.hex ? `0 0 10px ${color.hex}` : "none",
                  transition: "all 0.2s"
                }}
              />
            ))}
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Download Save Location</span>
            <span className="settings-desc">The folder where installer packages are downloaded (blank defaults to system Downloads).</span>
          </div>
          <div className="settings-control" style={{ display: "flex", gap: "8px" }}>
            <input type="text" value={settings.download_location} readOnly placeholder="Default Downloads Folder" />
            <button className="btn-secondary" onClick={handleSelectDir}>Browse</button>
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Auto Launch App</span>
            <span className="settings-desc">Automatically open the desktop application immediately after installation completes.</span>
          </div>
          <div className="settings-control">
            <label className="switch">
              <input type="checkbox" checked={settings.auto_launch_after_install === "true"} onChange={(e) => saveSetting("auto_launch_after_install", e.target.checked ? "true" : "false")} />
              <span className="slider"></span>
            </label>
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Create Desktop Shortcuts</span>
            <span className="settings-desc">Automatically create Start Menu (Windows) or .desktop launcher (Linux) entries.</span>
          </div>
          <div className="settings-control">
            <label className="switch">
              <input type="checkbox" checked={settings.desktop_shortcut === "true"} onChange={(e) => saveSetting("desktop_shortcut", e.target.checked ? "true" : "false")} />
              <span className="slider"></span>
            </label>
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Application Notifications</span>
            <span className="settings-desc">Show interactive notification bubbles for installations, completions, or errors.</span>
          </div>
          <div className="settings-control">
            <label className="switch">
              <input type="checkbox" checked={settings.notifications === "true"} onChange={(e) => saveSetting("notifications", e.target.checked ? "true" : "false")} />
              <span className="slider"></span>
            </label>
          </div>
        </div>

        <div className="settings-card">
          <div className="settings-info">
            <span className="settings-title">Concurrent Limit</span>
            <span className="settings-desc">Maximum number of downloads in progress running at the same time.</span>
          </div>
          <div className="settings-control">
            <select style={{ minWidth: "100px" }} value={settings.concurrent_downloads} onChange={(e) => saveSetting("concurrent_downloads", e.target.value)}>
              <option value="1">1</option>
              <option value="3">3</option>
              <option value="5">5</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
};

// 8. About Page
const AboutPage: React.FC = () => {
  return (
    <div className="page-container">
      <div className="about-header">
        <div className="about-logo">
          <Icons.Downloads size={40} style={{ color: "var(--primary)" }} />
        </div>
        <h2 style={{ fontSize: "1.6rem", fontWeight: 800 }}>Agneax Store</h2>
        <span className="about-version">v1.0.0 Stable</span>
        <p style={{ color: "var(--text-secondary)", maxWidth: "450px", fontSize: "0.85rem", lineHeight: 1.5 }}>
          The official application package store for AgneaxOS, supporting robust packaging, verification, and silent installer configurations.
        </p>
      </div>

      <div className="about-grid">
        <div className="about-card">
          <span className="about-card-title">Cross-Platform Engine</span>
          <p className="about-card-text">
            Builds on Tauri v2 and Rust, integrating native package managers like Winget, Chocolatey, Scoop on Windows, and Flatpak, Snap, APT, DNF, Pacman on Linux systems.
          </p>
        </div>
        <div className="about-card">
          <span className="about-card-title">Security & Sandboxing</span>
          <p className="about-card-text">
            All direct downloads are scanned, verified using SHA256 checksums, and launched with localized permissions constraints to ensure maximum desktop security.
          </p>
        </div>
      </div>

      <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: "0.75rem", marginTop: "24px" }}>
        © 2026 AgneaxOS Team. All rights reserved.
      </div>
    </div>
  );
};

// --- CORE LAYOUT COMPONENT ---
const AppLayout: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("home");
  const [historyStack, setHistoryStack] = useState<string[]>(["home"]);
  const [selectedApp, setSelectedApp] = useState<AppMetadata | null>(null);
  
  const { downloads, toasts, dismissToast, settings } = useApp();

  useEffect(() => {
    if (settings.accent_color) {
      document.documentElement.style.setProperty("--primary", settings.accent_color);
      if (settings.accent_color.startsWith("#")) {
        const hex = settings.accent_color.replace("#", "");
        const r = parseInt(hex.substring(0, 2), 16);
        const g = parseInt(hex.substring(2, 4), 16);
        const b = parseInt(hex.substring(4, 6), 16);
        document.documentElement.style.setProperty("--primary-glow", `rgba(${r}, ${g}, ${b}, 0.15)`);
        
        const hr = Math.min(255, r + 20);
        const hg = Math.min(255, g + 20);
        const hb = Math.min(255, b + 20);
        document.documentElement.style.setProperty("--primary-hover", `rgb(${hr}, ${hg}, ${hb})`);
      }
    }
  }, [settings.accent_color]);
  
  const navigateTo = (tab: string) => {
    setSelectedApp(null);
    setActiveTab(tab);
    setHistoryStack((prev) => [...prev, tab]);
  };

  const handleBack = () => {
    if (selectedApp) {
      setSelectedApp(null);
      return;
    }
    if (historyStack.length > 1) {
      const newStack = [...historyStack];
      newStack.pop(); // Remove current
      const prevTab = newStack[newStack.length - 1];
      setHistoryStack(newStack);
      setActiveTab(prevTab);
    }
  };

  const handleAppSelect = (app: AppMetadata) => {
    setSelectedApp(app);
  };

  // Determine active download speed for bottom monitor
  const activeDownloads = Object.values(downloads).filter(
    (dl) => dl.status === "Downloading" || dl.status === "Pending"
  );
  const totalSpeed = activeDownloads.reduce((acc, dl) => acc + dl.speed, 0);

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return "0 KB/s";
    const k = 1024;
    const sizes = ["B/s", "KB/s", "MB/s"];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(k));
    return parseFloat((bytesPerSec / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  return (
    <div className={`app-container ${settings.theme === "light" ? "light-theme" : "dark-theme"}`}>
      {/* Titlebar */}
      <Titlebar
        activePage={activeTab}
        onBack={handleBack}
        canGoBack={historyStack.length > 1 || selectedApp !== null}
      />
      
      {/* Sidebar Navigation */}
      <aside className="sidebar">
        <div className="sidebar-menu">
          <button className={`sidebar-item ${activeTab === "home" ? "active" : ""}`} onClick={() => navigateTo("home")}>
            <Icons.Home size={18} /> Home
          </button>
          <button className={`sidebar-item ${activeTab === "categories" ? "active" : ""}`} onClick={() => navigateTo("categories")}>
            <Icons.Categories size={18} /> Categories
          </button>
          <button className={`sidebar-item ${activeTab === "search" ? "active" : ""}`} onClick={() => navigateTo("search")}>
            <Icons.Search size={18} /> Search
          </button>
          <button className={`sidebar-item ${activeTab === "installed" ? "active" : ""}`} onClick={() => navigateTo("installed")}>
            <Icons.Installed size={18} /> Installed
          </button>
          <button className={`sidebar-item ${activeTab === "updates" ? "active" : ""}`} onClick={() => navigateTo("updates")}>
            <Icons.Updates size={18} /> Updates
          </button>
          <button className={`sidebar-item ${activeTab === "downloads" ? "active" : ""}`} onClick={() => navigateTo("downloads")}>
            <Icons.Downloads size={18} /> Downloads
          </button>
          <button className={`sidebar-item ${activeTab === "settings" ? "active" : ""}`} onClick={() => navigateTo("settings")}>
            <Icons.Settings size={18} /> Settings
          </button>
        </div>
        <div className="sidebar-footer" onClick={() => navigateTo("about")} style={{ cursor: "pointer" }}>
          <Icons.About size={18} style={{ color: "var(--text-secondary)" }} />
          <span style={{ fontSize: "0.85rem", color: "var(--text-secondary)", fontWeight: 500 }}>About Agneax</span>
        </div>
      </aside>
      
      {/* Main Pages content wrapper */}
      <main className="main-content">
        {activeTab === "home" && <HomePage onSelectApp={handleAppSelect} />}
        {activeTab === "categories" && <CategoriesPage onSelectApp={handleAppSelect} />}
        {activeTab === "search" && <SearchPage onSelectApp={handleAppSelect} />}
        {activeTab === "installed" && <InstalledPage onSelectApp={handleAppSelect} />}
        {activeTab === "updates" && <UpdatesPage onSelectApp={handleAppSelect} />}
        {activeTab === "downloads" && <DownloadsPage />}
        {activeTab === "settings" && <SettingsPage />}
        {activeTab === "about" && <AboutPage />}
        
        {/* Floating Download monitor overlay */}
        {activeDownloads.length > 0 && activeTab !== "downloads" && (
          <div className="floating-download-queue" onClick={() => navigateTo("downloads")} style={{ cursor: "pointer" }}>
            <div className="download-queue-header">
              <span>Active Downloads ({activeDownloads.length})</span>
              <span>{formatSpeed(totalSpeed)}</span>
            </div>
            {activeDownloads.slice(0, 2).map((dl) => (
              <div key={dl.id} className="download-queue-item">
                <div className="download-queue-item-name">
                  <span style={{ textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap", maxWidth: "200px" }}>{dl.app_name}</span>
                  <span>{dl.progress.toFixed(0)}%</span>
                </div>
                <div className="download-progress-bar-bg">
                  <div className="download-progress-bar-fill" style={{ width: `${dl.progress}%` }}></div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
      
      {/* App Details Modal */}
      {selectedApp && (
        <AppDetailModal app={selectedApp} onClose={() => setSelectedApp(null)} />
      )}
      
      {/* Toast Alert System overlay */}
      <div className="toast-container">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.type}`} onClick={() => dismissToast(toast.id)} style={{ cursor: "pointer" }}>
            {toast.type === "error" ? <Icons.AlertTriangle size={16} style={{ color: "#E81123" }} /> : <Icons.Check size={16} style={{ color: "#00CC66" }} />}
            <span>{toast.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

// --- APP ENTRY ROOT ---
function App() {
  return (
    <AppProvider>
      <AppLayout />
    </AppProvider>
  );
}

export default App;
