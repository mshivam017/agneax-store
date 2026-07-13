# 🛡️ Agneax Store

Agneax Store is the official package management and application store client designed for **AgneaxOS**, with standalone support for standard **Windows 10/11** and **Linux distributions** (Ubuntu, Debian, Linux Mint, Fedora, Arch Linux). 

Built using **Tauri v2**, **Rust**, **React**, and **TypeScript**, the store offers a modern frosted glassmorphic UI, high-performance background download orchestration, silent software installers, and automatic system shortcuts generation.

---

## ✨ Features

- **Multi-Source Package Registry:** Syncs catalogs directly from GitHub repository releases (`apps.json`) with an offline pre-bundled fallback of 10 popular applications.
- **Robust Downloader Engine:** Multi-threaded downloader with range-based pause/resume support, real-time speed tracking, size measurement, and ETA estimation.
- **Multi-OS Installer Dispatcher:**
  - **Windows:** Supports native package managers (`winget`, `choco`, `scoop`) and direct silent installer execution (for `MSI` and `EXE` formats with UAC override).
  - **Linux:** Coordinates packages through `flatpak`, `snap`, `apt`, `dnf`, `pacman`, and standalone `AppImage` files.
- **Detached Application Launcher:** Locates installations through registries and environment paths, spawning apps as completely detached child processes (the store can be closed without shutting down launched apps).
- **Shortcut Generator:** Writes Start Menu `.lnk` files on Windows (via PowerShell COM shell scripting) and standard `.desktop` entries on Linux.
- **Local Settings & Logging:** Utilizes an optimized SQLite DB running in WAL mode to track download history, installed packages, user preferences, and real-time step-by-step installation log reports.

---

## 🛠️ Tech Stack

- **Frontend:** React, TypeScript, Vite, Vanilla CSS (Glassmorphism design tokens)
- **Backend:** Rust, Tauri v2
- **Database:** SQLite (`rusqlite` with bundled driver support)
- **Networking:** `reqwest` (stream-based range requests)
- **Compilers:** MSVC / GNU MinGW GCC (`WinLibs` bundle)

---

## 📁 Repository Structure

```text
├── .github/workflows/       # GitHub Actions automated build scripts for Windows & Linux
├── src/                     # React Frontend Source Code
│   ├── assets/              # Logos and media
│   ├── components/          # Icons.tsx (custom SVG vector assets)
│   ├── context/             # AppContext.tsx (Tauri state event listeners & sqlite hooks)
│   ├── App.tsx              # Main Page Router, Titlebar, & Modal implementations
│   ├── App.css              # Frosted glass stylesheet and animations config
│   └── main.tsx             # React DOM injection entry
├── src-tauri/               # Tauri Backend Source Code
│   ├── capabilities/        # Tauri permissions and security configuration profiles
│   ├── src/                 # Rust core modules
│   │   ├── installer/       # Windows & Linux package execution sub-engines
│   │   ├── db.rs            # SQLite initialization, table migrations, and seeding
│   │   ├── logger.rs        # Step-by-step logging and text log export controls
│   │   ├── downloader.rs    # HTTP download loops, pause/resume maps, and task queue
│   │   ├── github.rs        # Catalog sync worker threads
│   │   ├── launcher.rs      # Detached process launcher
│   │   ├── shortcut.rs      # OS shortcut generation modules
│   │   ├── lib.rs           # Tauri command registries, setup hooks, and windows commands
│   │   └── main.rs          # Executable entry point
│   ├── Cargo.toml           # Rust dependencies declaration (rusqlite, rfd, futures-util)
│   └── tauri.conf.json      # Window styling, bundler configs, and decorations override
├── README.md                # Project documentation
└── package.json             # Node dependencies and build scripts
```

---

## ⚙️ Local Development & Build Setup

### Prerequisites (Windows Host)

To compile the application on Windows without requiring system administrator rights for MSVC or Visual Studio Build Tools, we use the **Rust GNU toolchain** and **WinLibs GCC compiler**:

1. **Install the GNU Toolchain:**
   ```powershell
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup default stable-x86_64-pc-windows-gnu
   ```

2. **Setup WinLibs GCC:**
   - Download the latest standalone 64-bit UCRT zip archive from [WinLibs](https://winlibs.com/).
   - Extract it to a temporary directory in your user space (e.g. `.temp/mingw64`).
   - Ensure the compiler's `bin` folder contains `gcc.exe` and `dlltool.exe`.

### 1. Run Node.js Development Server
```bash
npm install
npm run dev
```

### 2. Compile and Run Tauri Desktop Client
To build the app in debug mode with local PATH configurations:
```powershell
# Set path for current terminal session
$env:PATH = "C:\Users\Shivam\.cargo\bin;D:\myprojects\Github\.temp\mingw64\bin;" + $env:PATH
npm run tauri dev
```

### 3. Build Production Installers
Generates the NSIS installer `.exe` and Wix MSI installer `.msi` under `src-tauri/target/x86_64-pc-windows-gnu/release/bundle/`:
```powershell
$env:PATH = "C:\Users\Shivam\.cargo\bin;D:\myprojects\Github\.temp\mingw64\bin;" + $env:PATH
npm run tauri build -- --target x86_64-pc-windows-gnu
```

---

## ☁️ CI/CD Build Pipeline (Linux & Windows)

The project includes an automatic build workflow located at `.github/workflows/build.yml`. When changes are pushed to `main` or `master`:
- **Linux Job:** Boots up an Ubuntu runner, installs linking dependencies (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libappindicator3-dev`), builds the frontend, compiles the Tauri app, and archives the output **Debian installer (`.deb`)** and **standalone executable (`.AppImage`)**.
- **Windows Job:** Launches a Windows runner, installs the MSVC toolchain, compiles, and packages the **setup executable (`.exe`)** and **MSI package (`.msi`)**.

---

## 📊 SQLite Database Schema

The client maintains a local SQLite database named `store.db` under the user application directory (`%APPDATA%/com.agneax.store/` on Windows / `~/.config/com.agneax.store/` on Linux):

| Table | Purpose |
| :--- | :--- |
| `settings` | Key-value settings repository (accent_color, github_repo, shortcuts) |
| `cached_apps` | Cached registry metadata containing platform files, icons, and description |
| `installed_apps`| Record of applications installed via the store (date, version, run path) |
| `downloads` | Download queue history, file sizes, and status (Completed, Paused) |
| `favorites` | Favorite software ID mappings |
| `installation_logs`| Real-time step-by-step execution reporting logs |
