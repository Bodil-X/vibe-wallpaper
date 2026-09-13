use std::path::PathBuf;
use std::fs;
use std::io::Read;
use chrono::Local;
use winapi::um::winuser::{SPI_SETDESKWALLPAPER, SPI_GETDESKWALLPAPER, SPIF_UPDATEINIFILE, SPIF_SENDCHANGE, GetDC, EnumDisplayMonitors};
use winapi::um::wingdi::{GetDeviceCaps, HORZRES, VERTRES};
use winapi::shared::windef::{HDC, HMONITOR, LPRECT};
use winapi::shared::minwindef::{BOOL, LPARAM, MAX_PATH};
use std::ptr;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use base64::engine::general_purpose;
use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperPreview {
    pub id: String,
    pub url: String,
    pub path: String,
    pub base64: Option<String>,
}

unsafe impl Send for WallpaperManager {}
unsafe impl Sync for WallpaperManager {}

pub struct WallpaperManager {
    temp_dir: PathBuf,
    current_preview_images: Mutex<Vec<WallpaperPreview>>,
}

impl WallpaperManager {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("vibe_wallpaper");
        fs::create_dir_all(&temp_dir).ok();

        Self {
            temp_dir,
            current_preview_images: Mutex::new(Vec::new()),
        }
    }

    /// 根据 URL 模板和分辨率构建最终的图片 URL
    /// 支持 {{w}} 和 {{h}} 占位符，如: https://example.com/{{w}}x{{h}}
    /// 如果没有占位符，直接返回原 URL（不添加分辨率参数）
    pub fn format_image_url(url: &str, resolution: (u32, u32)) -> String {
        let has_width_placeholder = url.contains("{{w}}");
        let has_height_placeholder = url.contains("{{h}}");

        if has_width_placeholder || has_height_placeholder {
            // 有占位符，进行替换
            let mut result = url.to_string();
            if has_width_placeholder {
                result = result.replace("{{w}}", &resolution.0.to_string());
            }
            if has_height_placeholder {
                result = result.replace("{{h}}", &resolution.1.to_string());
            }
            result
        } else {
            // 没有占位符，直接返回原 URL
            url.to_string()
        }
    }

    pub fn download_and_set_wallpaper(
        &self,
        image_url: &str,
        resolution: (u32, u32),
        different_per_monitor: bool,
        _proxy_config: Option<&crate::config::AppConfig>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 如果是自动检测模式，获取真实屏幕分辨率
        let actual_resolution = if resolution.0 == 0 && resolution.1 == 0 {
            Self::get_screen_resolution()
        } else {
            resolution
        };

        // 使用新的 URL 格式化方法
        let final_url = Self::format_image_url(image_url, actual_resolution);

        // 下载图片
        let image_path = self.download_image(&final_url)?;

        // 设置壁纸
        self.set_wallpaper(&image_path, different_per_monitor)?;

        Ok(())
    }

    fn download_image(
        &self,
        url: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // 使用 ureq 进行 HTTP 请求（同步，避免异步复杂性）
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(30))
            .call()?;

        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        // 保存为临时文件
        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let file_name = format!("vibe_{}.jpg", timestamp);
        let file_path = self.temp_dir.join(file_name);

        fs::write(&file_path, bytes)?;

        Ok(file_path)
    }

    fn set_wallpaper(
        &self,
        image_path: &PathBuf,
        _different_per_monitor: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Windows API 需要 UTF-16 编码的路径
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let path_str: &OsStr = image_path.as_os_str();
        let wide_path: Vec<u16> = path_str.encode_wide().chain(Some(0)).collect();

        unsafe {
            // 使用 Unicode 版本的 Windows API
            use winapi::um::winuser::SystemParametersInfoW;

            let result = SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                wide_path.as_ptr() as *mut _,
                SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
            );

            if result == 0 {
                let error = std::io::Error::last_os_error();
                return Err(format!("设置壁纸失败: {}", error).into());
            }
        }

        Ok(())
    }

    pub fn save_current_wallpaper(
        &self,
        save_location: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 获取当前壁纸路径（简化实现）
        // 实际项目中可能需要读取注册表或使用Windows API获取当前壁纸

        let save_dir = PathBuf::from(save_location);
        fs::create_dir_all(&save_dir)?;

        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let file_name = format!("Vibe_{}.jpg", timestamp);
        let save_path = save_dir.join(file_name);

        // 这里需要实现获取当前壁纸的逻辑
        // 暂时创建一个占位文件
        fs::write(&save_path, b"placeholder")?;

        Ok(())
    }

    pub fn get_screen_resolution() -> (u32, u32) {
        unsafe {
            let hdc = GetDC(ptr::null_mut());
            if hdc.is_null() {
                return (1920, 1080); // 默认分辨率
            }

            let width = GetDeviceCaps(hdc, HORZRES);
            let height = GetDeviceCaps(hdc, VERTRES);

            (width as u32, height as u32)
        }
    }

    /// 获取系统中的显示器数量
    pub fn get_monitor_count() -> usize {
        unsafe extern "system" fn enum_monitor_callback(
            _hmonitor: HMONITOR,
            _hdc: HDC,
            _lprect: LPRECT,
            lparam: LPARAM,
        ) -> BOOL {
            let count = lparam as *mut usize;
            *count += 1;
            1 // 返回 TRUE 继续枚举
        }

        unsafe {
            let mut count: usize = 0;
            EnumDisplayMonitors(
                ptr::null_mut(),
                ptr::null_mut(),
                Some(enum_monitor_callback),
                &mut count as *mut usize as LPARAM,
            );

            // 如果枚举失败，至少返回1（假设有一个显示器）
            if count == 0 {
                count = 1;
            }

            count
        }
    }

    pub fn download_multiple_wallpapers(
        &self,
        image_url: &str,
        resolution: (u32, u32),
        count: u32,
        _proxy_config: Option<&crate::config::AppConfig>,
    ) -> Result<Vec<WallpaperPreview>, Box<dyn std::error::Error>> {
        let mut previews = Vec::new();

        println!("开始下载 {} 张壁纸，基础URL: {}", count, image_url);

        // 如果是自动检测模式，获取真实屏幕分辨率
        let actual_resolution = if resolution.0 == 0 && resolution.1 == 0 {
            Self::get_screen_resolution()
        } else {
            resolution
        };

        for i in 0..count {
            let timestamp = Local::now().format("%Y%m%d%H%M%S");
            let file_name = format!("preview_{}_{}.jpg", timestamp, i);
            let file_path = self.temp_dir.join(&file_name);

            // 使用新的 URL 格式化方法
            let final_url = Self::format_image_url(image_url, actual_resolution);

            println!("正在下载第 {} 张壁纸: {}", i + 1, final_url);

            match self.download_image_bytes(&final_url) {
                Ok(image_bytes) => {
                    // 保存到文件
                    if fs::write(&file_path, &image_bytes).is_err() {
                        eprintln!("保存第 {} 张壁纸失败", i + 1);
                        continue;
                    }

                    // 编码为base64
                    let base64_data = general_purpose::STANDARD.encode(&image_bytes);

                    println!("第 {} 张壁纸下载成功，保存到: {:?}", i + 1, file_path);
                    let preview = WallpaperPreview {
                        id: format!("{}_{}", timestamp, i),
                        url: final_url,
                        path: file_path.to_string_lossy().to_string(),
                        base64: Some(format!("data:image/jpeg;base64,{}", base64_data)),
                    };
                    previews.push(preview);
                }
                Err(e) => {
                    eprintln!("下载第 {} 张壁纸失败: {}", i + 1, e);
                }
            }
        }

        println!("共成功下载 {} 张壁纸", previews.len());

        *self.current_preview_images.lock().unwrap() = previews.clone();
        Ok(previews)
    }

    pub fn download_single_wallpaper(
        &self,
        image_url: &str,
        resolution: (u32, u32),
        index: u32,
        _proxy_config: Option<&crate::config::AppConfig>,
    ) -> Result<WallpaperPreview, Box<dyn std::error::Error>> {
        self.download_single_wallpaper_with_progress(image_url, resolution, index, _proxy_config, Box::new(|_, _| {}))
    }

    pub fn download_single_wallpaper_with_progress(
        &self,
        image_url: &str,
        resolution: (u32, u32),
        index: u32,
        _proxy_config: Option<&crate::config::AppConfig>,
        progress_callback: Box<dyn Fn(usize, usize) + Send>,
    ) -> Result<WallpaperPreview, Box<dyn std::error::Error>> {
        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let file_name = format!("preview_{}_{}.jpg", timestamp, index);
        let file_path = self.temp_dir.join(&file_name);

        // 如果是自动检测模式，获取真实屏幕分辨率
        let actual_resolution = if resolution.0 == 0 && resolution.1 == 0 {
            let screen_res = Self::get_screen_resolution();
            println!("自动检测屏幕分辨率: {}x{}", screen_res.0, screen_res.1);
            screen_res
        } else {
            resolution
        };

        // 使用新的 URL 格式化方法
        let final_url = Self::format_image_url(image_url, actual_resolution);

        println!("正在下载壁纸: {}", final_url);

        // 下载图片并获取字节数据，带进度回调
        let image_bytes = match self.download_image_bytes_with_progress(&final_url, Some(progress_callback)) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("下载壁纸失败: {}", e);
                return Err(e);
            }
        };

        // 保存到文件
        if let Err(e) = fs::write(&file_path, &image_bytes) {
            eprintln!("保存壁纸失败: {}", e);
            return Err(e.into());
        }

        // 编码为base64
        let base64_data = general_purpose::STANDARD.encode(&image_bytes);

        // 创建预览记录
        let preview = WallpaperPreview {
            id: format!("{}_{}", timestamp, index),
            url: final_url.clone(),
            path: file_path.to_string_lossy().to_string(),
            base64: Some(format!("data:image/jpeg;base64,{}", base64_data)),
        };

        println!("壁纸下载成功，保存到: {:?}", file_path);
        Ok(preview)
    }

    fn download_image_to_path(
        &self,
        url: &str,
        path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(30))
            .call()?;

        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        fs::write(path, bytes)?;
        Ok(())
    }

    fn download_image_bytes(
        &self,
        url: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.download_image_bytes_with_progress(url, None)
    }

    fn download_image_bytes_with_progress(
        &self,
        url: &str,
        progress_callback: Option<Box<dyn Fn(usize, usize) + Send>>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(30))
            .call()?;

        // 尝试获取内容长度
        let content_length = response
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let mut reader = response.into_reader();
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 8192]; // 8KB 缓冲区
        let mut downloaded = 0;

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
            downloaded += n;

            // 调用进度回调
            if let Some(ref callback) = progress_callback {
                callback(downloaded, content_length);
            }
        }

        Ok(bytes)
    }

    pub fn set_specific_wallpaper(
        &self,
        image_path: &str,
        monitor: Option<i32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(image_path);
        if !path.exists() {
            return Err("Image file does not exist".into());
        }

        self.set_wallpaper(&path, monitor.is_some())?;
        Ok(())
    }

    pub fn save_wallpaper_to_location(
        &self,
        image_path: &str,
        save_location: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let source_path = PathBuf::from(image_path);
        if !source_path.exists() {
            return Err("Source image does not exist".into());
        }

        let save_dir = PathBuf::from(save_location);
        fs::create_dir_all(&save_dir)?;

        let timestamp = Local::now().format("%Y%m%d%H%M%S");
        let file_name = format!("Vibe_{}.jpg", timestamp);
        let save_path = save_dir.join(file_name);

        fs::copy(&source_path, &save_path)?;
        Ok(save_path)
    }

    pub fn get_current_preview_images(&self) -> Vec<WallpaperPreview> {
        self.current_preview_images.lock().unwrap().clone()
    }

    /// 获取当前桌面壁纸路径
    pub fn get_current_wallpaper_path() -> Option<String> {
        unsafe {
            use winapi::um::winuser::SystemParametersInfoW;
            use std::ffi::OsString;
            use std::os::windows::ffi::OsStringExt;

            let mut path_buffer: [u16; MAX_PATH] = [0; MAX_PATH];

            let result = SystemParametersInfoW(
                SPI_GETDESKWALLPAPER,
                MAX_PATH as u32,
                path_buffer.as_mut_ptr() as *mut _,
                0,
            );

            if result != 0 {
                // 找到字符串结束位置
                let len = path_buffer.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
                let os_string = OsString::from_wide(&path_buffer[..len]);
                let path_str = os_string.to_string_lossy().to_string();

                if !path_str.is_empty() && PathBuf::from(&path_str).exists() {
                    return Some(path_str);
                }
            }
            None
        }
    }

    /// 获取当前桌面壁纸并转为 base64
    pub fn get_current_wallpaper_base64() -> Option<String> {
        if let Some(path) = Self::get_current_wallpaper_path() {
            if let Ok(bytes) = fs::read(&path) {
                let base64_data = general_purpose::STANDARD.encode(&bytes);
                // 根据文件扩展名判断 MIME 类型
                let mime_type = if path.to_lowercase().ends_with(".png") {
                    "image/png"
                } else if path.to_lowercase().ends_with(".bmp") {
                    "image/bmp"
                } else {
                    "image/jpeg"
                };
                return Some(format!("data:{};base64,{}", mime_type, base64_data));
            }
        }
        None
    }
}