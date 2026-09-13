# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vibe Wallpaper is a Windows desktop application built with Tauri 2.0 + Vue 3 + TypeScript that downloads and sets desktop wallpapers from a configurable image source (defaults to an Unsplash-style URL). It supports a settings window and a separate full-size preview/picker window, system tray integration, multi-monitor handling, scheduled auto-refresh, and Windows auto-start.

## Development Commands

### Frontend / Tauri (run from repo root with pnpm)
```bash
pnpm install        # Install dependencies (pnpm)
pnpm dev            # Vite dev server only (localhost:1420)
pnpm build          # Type-check (vue-tsc --noEmit) + Vite build
pnpm preview        # Preview the built frontend
pnpm tauri dev      # Run the full app in development
pnpm tauri build    # Build the distributable app
```

### Rust backend (run from `src-tauri/`)
```bash
cd src-tauri
cargo build
cargo test          # No tests exist yet
```

## Architecture

### Technology Stack
- **Backend**: Rust + Tauri 2.0
- **Frontend**: Vue 3 (Composition API, `<script setup>`) + TypeScript
- **Build Tool**: Vite 6 (multi-page: `index.html` + `preview.html`)
- **Package Manager**: pnpm
- **Target Platform**: Windows only (uses Win32 APIs)

### Project Structure
```
src/                              # Frontend (Vue 3 + TypeScript)
├── components/
│   ├── ConfigPage.vue            # Main settings UI (mounted in main window)
│   └── WallpaperPreviewWindow.vue# Full-size wallpaper browser/picker UI
├── App.vue                       # Root component for main window (blurred wallpaper bg + ConfigPage)
├── main.ts                       # Entry for main window (index.html)
└── preview.ts                    # Entry for preview window (preview.html)

src-tauri/                        # Backend (Rust)
├── src/
│   ├── lib.rs                    # Tauri app: AppState, all #[tauri::command]s, tray, setup
│   ├── main.rs                   # Thin binary that calls tauri_app_lib::run()
│   ├── config.rs                 # AppConfig struct, load/save, interval/resolution parsing
│   ├── wallpaper.rs              # WallpaperManager: download, set, preview, base64 helpers
│   ├── scheduler.rs              # WallpaperScheduler: background refresh (std thread)
│   └── auto_start.rs             # AutoStartManager: HKCU\...\Run registry entry
├── capabilities/                 # default.json + desktop.json (permission sets)
├── Cargo.toml
└── tauri.conf.json
```
> Note: `src-tauri/src/lib.rs.backup` and `tray_icon_fix.patch` are stale artifacts, not part of the build.

### Windows (frontend)
Two webview windows are defined:
- **main** — settings window (`index.html` → `App.vue`/`ConfigPage.vue`), 650×910.
- **preview** — created on demand by the `open_preview_window` command (`preview.html` → `WallpaperPreviewWindow.vue`), 1200×800. The preview window fetches candidate images and lets the user pick one to set/save.

### Tauri Commands (registered in `lib.rs`)
- `load_config()` — load config from disk
- `save_config(config)` — persist config and update global state
- `toggle_auto_start(enabled)` — enable/disable Windows auto-start
- `refresh_wallpaper()` — download one image and set it as wallpaper
- `save_current_wallpaper()` — copy the current desktop wallpaper to the save location
- `update_scheduler()` — restart the background refresh timer from current config
- `fetch_new_wallpapers()` — download a batch (5) of preview candidates
- `fetch_single_wallpaper(index)` — download one candidate, emitting `download-progress` events (u8 percentage)
- `set_wallpaper_from_path(image_path, monitor?)` — set a specific local image as wallpaper
- `save_wallpaper_from_path(image_path)` — copy a specific image to the save location
- `get_preview_images()` — return cached preview candidates
- `open_preview_window()` — create/show the preview window
- `get_monitor_count()` — number of monitors
- `get_current_wallpaper()` — current wallpaper as a base64 data URL

### State Management
Global `AppState` (Tauri-managed) holds:
- `config: Arc<Mutex<AppConfig>>`
- `wallpaper_manager: Arc<Mutex<WallpaperManager>>`
- `scheduler: Arc<Mutex<WallpaperScheduler>>`
- `preview_images: Arc<Mutex<Vec<WallpaperPreview>>>` — cached preview candidates

### Configuration System
Stored at `%LOCALAPPDATA%\vibe-wallpaper\config.json` (see `AppConfig` in `config.rs`). Fields:
- `refresh_interval` — e.g. `"5min"`, `"1hour"`, `"1day"` (parsed by `get_refresh_duration_seconds`, defaults to 1h)
- `resolution` — `"auto"` or `"WxH"` (e.g. `"1920x1080"`); `"auto"` queries the real screen size
- `different_per_monitor` — bool
- `auto_start` — bool (reconciled with the registry on startup)
- `save_location` — defaults to `~/Pictures/Vibe`
- `proxy_enabled`, `proxy_url`, `proxy_username`, `proxy_password`
- `source_type` — `"unsplash"` or `"custom"`
- `source_url` — image URL template; default `https://source.unsplash.com/random/{{w}}x{{h}}`
- `theme` — `"system"`, `"light"`, or `"dark"`

### URL Templating
`WallpaperManager::format_image_url` replaces `{{w}}` / `{{h}}` in `source_url` with the resolved width/height. URLs without placeholders are used as-is.

### Windows Integration
- **Wallpaper Setting**: `winapi` (`SystemParametersInfo` with `SPI_SETDESKWALLPAPER`); paths converted to UTF-16
- **Screen/Monitor info**: `EnumDisplayMonitors`, `GetDeviceCaps` for resolution and monitor count
- **Auto-Start**: `winreg` writes/removes the `VibeWallpaper` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- **System Tray**: tray menu with 刷新壁纸 (refresh) / 设置 (settings) / 退出 (exit); left-click shows the main window

### External Dependencies (Rust)
- `tauri` (features: tray-icon, wry, image-png, image-ico) + `tauri-plugin-fs`, `tauri-plugin-dialog`
- `ureq` — synchronous HTTP client for image downloads
- `winapi`, `winreg`, `windows` — Windows APIs
- `serde`/`serde_json`, `chrono`, `dirs`, `base64`, `tokio` (present, but the scheduler uses `std::thread`)

### Frontend Dependencies
`@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-fs`, `vue`.

## Important Notes

1. **Windows-Only**: Uses Win32 APIs; will not build/run meaningfully on other platforms.
2. **No Test Suite**: No automated Rust or frontend tests.
3. **No Linting**: No ESLint/Prettier config.
4. **Development Port**: Fixed at localhost:1420 (Vite `strictPort`).
5. **Chinese UI/Comments**: UI text and code comments are largely in Chinese.
6. **Sync HTTP**: Downloads use synchronous `ureq` to avoid async complexity.
7. **Proxy is configured but not wired up**: `proxy_*` fields are read into config, but the actual `ureq` download path ignores them (`_proxy_config`). Proxy settings currently have no effect.
8. **Scheduler uses `std::thread`**, not `tokio`, despite the tokio dependency.
9. **Stale capability scopes**: `capabilities/*.json` `fs:scope`/protocol scopes still reference old `unsplash_wallpaper`/`unsplash-wallpaper` paths, while the code uses `%TEMP%\vibe_wallpaper` and `%LOCALAPPDATA%\vibe-wallpaper`. Update scopes if relying on plugin-fs/asset access to those dirs.
10. **Temp files**: Downloaded images are cached under `%TEMP%\vibe_wallpaper`.
