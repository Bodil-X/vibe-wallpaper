use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use dirs;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub refresh_interval: String,
    pub resolution: String,
    pub different_per_monitor: bool,
    pub auto_start: bool,
    pub save_location: String,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    pub proxy_username: String,
    pub proxy_password: String,
    #[serde(default = "default_source_type")]
    pub source_type: String,  // "unsplash" 或 "custom"
    pub source_url: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_source_type() -> String {
    "unsplash".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_save_location = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Pictures")
            .join("Vibe");

        Self {
            refresh_interval: "1hour".to_string(),
            resolution: "auto".to_string(),
            different_per_monitor: true,
            auto_start: false,
            save_location: default_save_location.to_string_lossy().to_string(),
            proxy_enabled: false,
            proxy_url: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            source_type: "unsplash".to_string(),
            source_url: "https://source.unsplash.com/random/{{w}}x{{h}}".to_string(),
            theme: "system".to_string(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("vibe-wallpaper")
            .join("config.json")
    }

    pub fn load() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(config) => config,
                        Err(e) => {
                            eprintln!("Failed to parse config: {}", e);
                            Self::default()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read config: {}", e);
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();

        // 确保目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn get_refresh_duration_seconds(&self) -> u64 {
        let parts: Vec<&str> = self.refresh_interval.split("hour").collect();
        if parts.len() == 2 {
            if let Ok(hours) = parts[0].parse::<u64>() {
                return hours * 3600;
            }
        }

        let parts: Vec<&str> = self.refresh_interval.split("min").collect();
        if parts.len() == 2 {
            if let Ok(minutes) = parts[0].parse::<u64>() {
                return minutes * 60;
            }
        }

        let parts: Vec<&str> = self.refresh_interval.split("day").collect();
        if parts.len() == 2 {
            if let Ok(days) = parts[0].parse::<u64>() {
                return days * 86400;
            }
        }

        3600 // 默认1小时
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        if self.resolution == "auto" {
            return (0, 0);
        }

        let parts: Vec<&str> = self.resolution.split('x').collect();
        if parts.len() == 2 {
            if let Ok(width) = parts[0].parse::<u32>() {
                if let Ok(height) = parts[1].parse::<u32>() {
                    return (width, height);
                }
            }
        }

        (0, 0)
    }
}