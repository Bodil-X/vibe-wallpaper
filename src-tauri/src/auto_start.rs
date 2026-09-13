use winreg::enums::*;
use winreg::RegKey;
use std::path::PathBuf;
use std::env;

pub struct AutoStartManager;

impl AutoStartManager {
    pub fn is_enabled() -> bool {
        match Self::get_run_key() {
            Ok(run_key) => {
                let values = run_key.enum_values();
                for value in values {
                    if let Ok((name, _)) = value {
                        if name == "VibeWallpaper" {
                            return true;
                        }
                    }
                }
            }
            Err(_) => return false,
        }
        false
    }

    pub fn enable() -> Result<(), Box<dyn std::error::Error>> {
        let exe_path = Self::get_exe_path()?;
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let (run_key, _) = hklm.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;

        run_key.set_value("VibeWallpaper", &exe_path.to_string_lossy().to_string())?;
        Ok(())
    }

    pub fn disable() -> Result<(), Box<dyn std::error::Error>> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hklm.open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_SET_VALUE)?;

        if Self::is_enabled() {
            run_key.delete_value("VibeWallpaper")?;
        }

        Ok(())
    }

    fn get_run_key() -> Result<RegKey, Box<dyn std::error::Error>> {
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hklm.open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Run", KEY_READ)?;
        Ok(run_key)
    }

    fn get_exe_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let exe_path = env::current_exe()?;
        Ok(exe_path)
    }
}