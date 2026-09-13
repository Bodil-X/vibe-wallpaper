mod config;
mod wallpaper;
mod scheduler;
mod auto_start;

use std::sync::{Arc, Mutex};
use tauri::{
    Manager, Emitter,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};

// 全局状态
struct AppState {
    config: Arc<Mutex<config::AppConfig>>,
    wallpaper_manager: Arc<Mutex<wallpaper::WallpaperManager>>,
    scheduler: Arc<Mutex<scheduler::WallpaperScheduler>>,
    preview_images: Arc<Mutex<Vec<wallpaper::WallpaperPreview>>>,
}

#[tauri::command]
fn load_config() -> Result<config::AppConfig, String> {
    let config = config::AppConfig::load();
    println!("加载配置: source_url={}, refresh_interval={}", config.source_url, config.refresh_interval);
    Ok(config)
}

#[tauri::command]
fn save_config(config: config::AppConfig, state: tauri::State<AppState>) -> Result<(), String> {
    // 保存到文件
    config.save().map_err(|e| e.to_string())?;

    // 更新全局状态
    let mut state_config = state.config.lock().unwrap();
    *state_config = config.clone();

    println!("配置已更新: source_url={}, refresh_interval={}", config.source_url, config.refresh_interval);

    Ok(())
}

#[tauri::command]
fn toggle_auto_start(enabled: bool) -> Result<(), String> {
    if enabled {
        auto_start::AutoStartManager::enable().map_err(|e| e.to_string())
    } else {
        auto_start::AutoStartManager::disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn refresh_wallpaper(state: tauri::State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap();
    let resolution = config.get_resolution();
    let proxy_config = if config.proxy_enabled {
        Some(config.clone())
    } else {
        None
    };

    println!("刷新壁纸使用配置: source_url={}, resolution={:?}", config.source_url, resolution);

    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();
    wallpaper_manager
        .download_and_set_wallpaper(
            &config.source_url,
            resolution,
            config.different_per_monitor,
            proxy_config.as_ref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_current_wallpaper(state: tauri::State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap();
    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();
    wallpaper_manager
        .save_current_wallpaper(&config.save_location)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_scheduler(state: tauri::State<AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap();
    let interval_seconds = config.get_refresh_duration_seconds();

    let scheduler = state.scheduler.lock().unwrap();
    scheduler.start(
        interval_seconds,
        Arc::clone(&state.wallpaper_manager),
        Arc::clone(&state.config),
    );

    Ok(())
}

#[tauri::command]
fn fetch_new_wallpapers(state: tauri::State<AppState>) -> Result<Vec<wallpaper::WallpaperPreview>, String> {
    let config = state.config.lock().unwrap();
    let resolution = config.get_resolution();
    let proxy_config = if config.proxy_enabled {
        Some(config.clone())
    } else {
        None
    };

    // 更好的分辨率显示
    let resolution_str = if resolution.0 > 0 && resolution.1 > 0 {
        format!("{}x{}", resolution.0, resolution.1)
    } else {
        "自动检测".to_string()
    };

    println!("获取新壁纸 - URL: {}, 分辨率: {}, 代理: {}",
        config.source_url,
        resolution_str,
        if proxy_config.is_some() { "启用" } else { "禁用" }
    );

    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();
    match wallpaper_manager
        .download_multiple_wallpapers(
            &config.source_url,
            resolution,
            5, // 下载5张壁纸供预览
            proxy_config.as_ref(),
        ) {
        Ok(previews) => {
            println!("成功获取 {} 张壁纸", previews.len());
            Ok(previews)
        },
        Err(e) => {
            println!("获取壁纸失败: {}", e);
            Err(e.to_string())
        },
    }
}

#[tauri::command]
fn fetch_single_wallpaper(index: u32, state: tauri::State<AppState>, app: tauri::AppHandle) -> Result<wallpaper::WallpaperPreview, String> {
    let config = state.config.lock().unwrap();
    let resolution = config.get_resolution();
    let proxy_config = if config.proxy_enabled {
        Some(config.clone())
    } else {
        None
    };

    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();

    // 发送进度事件
    let app_clone = app.clone();
    let app_clone_final = app.clone(); // 用于发送最终 100% 进度
    let last_progress = Arc::new(Mutex::new(0u8));
    let result = wallpaper_manager
        .download_single_wallpaper_with_progress(
            &config.source_url,
            resolution,
            index,
            proxy_config.as_ref(),
            Box::new(move |downloaded, total| {
                let progress = if total > 0 {
                    // 有 Content-Length，使用实际进度
                    let p = (downloaded as f64 / total as f64 * 100.0) as u8;
                    p.min(100)
                } else {
                    // 没有 Content-Length，使用估算进度
                    // 假设图片大小约 2-5 MB
                    let estimated_total = 3_000_000; // 3MB
                    let p = (downloaded as f64 / estimated_total as f64 * 100.0) as u8;
                    p.min(95) // 最多到 95%，等真正完成时会到 100%
                };

                // 只在进度变化时发送事件，避免发送太频繁
                let mut last = last_progress.lock().unwrap();
                if progress != *last && progress > *last {
                    // 只在跨越10%边界或达到100%时打印日志
                    let current_ten = progress / 10;
                    let last_ten = *last / 10;
                    if current_ten > last_ten || progress == 100 {
                        println!("下载进度: {}% (已下载: {} bytes, 总大小: {} bytes)", progress, downloaded, total);
                    }
                    *last = progress;
                    let _ = app_clone.emit("download-progress", progress);
                }
            }),
        );

    match result {
        Ok(preview) => {
            // 下载完成，确保进度显示为 100%
            let _ = app_clone_final.emit("download-progress", 100u8);

            // 保存到预览列表
            let mut preview_images = state.preview_images.lock().unwrap();
            preview_images.push(preview.clone());
            Ok(preview)
        },
        Err(e) => {
            println!("获取单张壁纸失败: {}", e);
            Err(e.to_string())
        },
    }
}

#[tauri::command]
fn set_wallpaper_from_path(image_path: String, monitor: Option<i32>, state: tauri::State<AppState>) -> Result<(), String> {
    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();
    wallpaper_manager
        .set_specific_wallpaper(&image_path, monitor)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_wallpaper_from_path(image_path: String, state: tauri::State<AppState>) -> Result<String, String> {
    let config = state.config.lock().unwrap();
    let wallpaper_manager = state.wallpaper_manager.lock().unwrap();
    let result = wallpaper_manager
        .save_wallpaper_to_location(&image_path, &config.save_location)
        .map_err(|e| e.to_string())?;
    Ok(result.to_string_lossy().to_string())
}

#[tauri::command]
fn get_preview_images(state: tauri::State<AppState>) -> Result<Vec<wallpaper::WallpaperPreview>, String> {
    let preview_images = state.preview_images.lock().unwrap();
    Ok(preview_images.clone())
}

#[tauri::command]
async fn open_preview_window(app: tauri::AppHandle) -> Result<(), String> {
    // 检查是否已经存在预览窗口
    if let Some(existing) = app.get_webview_window("preview") {
        // 如果窗口已存在，只是显示它
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }

    // 使用 create_window 创建新窗口（这是Tauri 2.0的稳定API）
    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        "preview",
        tauri::WebviewUrl::App("preview.html".into())
    )
    .title("壁纸预览")
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn get_monitor_count() -> Result<usize, String> {
    Ok(wallpaper::WallpaperManager::get_monitor_count())
}

#[tauri::command]
fn get_current_wallpaper() -> Result<Option<String>, String> {
    Ok(wallpaper::WallpaperManager::get_current_wallpaper_base64())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 加载配置
    let mut config = config::AppConfig::load();

    // 确保自动启动状态正确
    config.auto_start = auto_start::AutoStartManager::is_enabled();

    let config = Arc::new(Mutex::new(config));
    let wallpaper_manager = Arc::new(Mutex::new(wallpaper::WallpaperManager::new()));
    let scheduler = Arc::new(Mutex::new(scheduler::WallpaperScheduler::new()));
    let preview_images = Arc::new(Mutex::new(Vec::new()));

    // 创建应用状态
    let app_state = AppState {
        config: Arc::clone(&config),
        wallpaper_manager: Arc::clone(&wallpaper_manager),
        scheduler: Arc::clone(&scheduler),
        preview_images: Arc::clone(&preview_images),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            toggle_auto_start,
            refresh_wallpaper,
            save_current_wallpaper,
            update_scheduler,
            fetch_new_wallpapers,
            fetch_single_wallpaper,
            set_wallpaper_from_path,
            save_wallpaper_from_path,
            get_preview_images,
            open_preview_window,
            get_monitor_count,
            get_current_wallpaper
        ])
        .setup(|app| {
            // 启动时开始定时器
            let state = app.state::<AppState>();
            let config = state.config.lock().unwrap();
            let interval_seconds = config.get_refresh_duration_seconds();

            let scheduler = state.scheduler.lock().unwrap();
            scheduler.start(
                interval_seconds,
                Arc::clone(&state.wallpaper_manager),
                Arc::clone(&state.config),
            );

            // 创建系统托盘 - 使用正确的 Tauri 2.0 API
            let tray_menu = Menu::new(app)?;

            let refresh_item = MenuItem::new(app, "刷新壁纸", true, None::<&str>)?;
            let settings_item = MenuItem::new(app, "设置", true, None::<&str>)?;
            let exit_item = MenuItem::new(app, "退出", true, None::<&str>)?;

            tray_menu.append(&refresh_item)?;
            tray_menu.append(&settings_item)?;
            tray_menu.append(&PredefinedMenuItem::separator(app)?)?;
            tray_menu.append(&exit_item)?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
                    // 如果图标加载失败，使用默认图标
                    Image::new(&[255; 32*32*4], 32, 32)
                }))
                .menu(&tray_menu)
                .on_menu_event(move |app, event| {
                    if event.id == refresh_item.id() {
                        let state = app.state::<AppState>();
                        let wallpaper_manager = Arc::clone(&state.wallpaper_manager);
                        let config = state.config.lock().unwrap();
                        let resolution = config.get_resolution();
                        let proxy_config = if config.proxy_enabled {
                            Some(config.clone())
                        } else {
                            None
                        };

                        let wallpaper_manager = wallpaper_manager.lock().unwrap();
                        match wallpaper_manager.download_and_set_wallpaper(
                            &config.source_url,
                            resolution,
                            config.different_per_monitor,
                            proxy_config.as_ref(),
                        ) {
                            Ok(_) => println!("Wallpaper refreshed from tray"),
                            Err(e) => eprintln!("Failed to refresh wallpaper: {}", e),
                        }
                    } else if event.id == settings_item.id() {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if event.id == exit_item.id() {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}