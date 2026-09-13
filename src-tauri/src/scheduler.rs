use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub struct WallpaperScheduler {
    running: Arc<AtomicBool>,
}

impl WallpaperScheduler {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(
        &self,
        interval_seconds: u64,
        wallpaper_manager: Arc<std::sync::Mutex<crate::wallpaper::WallpaperManager>>,
        config: Arc<std::sync::Mutex<crate::config::AppConfig>>,
    ) {
        self.stop(); // 确保先停止之前的任务

        let running = Arc::clone(&self.running);
        running.store(true, Ordering::SeqCst);

        // 使用标准线程而不是 tokio
        thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(interval_seconds));

                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // 执行壁纸刷新
                let config_guard = config.lock().unwrap();
                let resolution = config_guard.get_resolution();
                let _proxy_config = if config_guard.proxy_enabled {
                    Some(config_guard.clone())
                } else {
                    None
                };

                let wallpaper_manager_guard = wallpaper_manager.lock().unwrap();
                match wallpaper_manager_guard.download_and_set_wallpaper(
                    &config_guard.source_url,
                    resolution,
                    config_guard.different_per_monitor,
                    None, // proxy_config not used in current implementation
                ) {
                    Ok(_) => {
                        println!("Wallpaper refreshed automatically");
                    }
                    Err(e) => {
                        eprintln!("Failed to refresh wallpaper: {}", e);
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for WallpaperScheduler {
    fn drop(&mut self) {
        self.stop();
    }
}