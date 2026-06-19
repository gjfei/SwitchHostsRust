use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::atomic::atomic_write;
use super::error::StorageError;

pub const CONFIG_FORMAT: &str = "switchhosts-config";
pub const CONFIG_SCHEMA_VERSION: u32 = 1;
const MAX_PROXY_PORT: u32 = 65535;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub left_panel_show: bool,
    pub left_panel_width: u32,
    pub right_panel_show: bool,
    pub right_panel_width: u32,
    pub use_system_window_frame: bool,
    pub write_mode: String,
    pub history_limit: u32,
    pub locale: Option<String>,
    pub theme: String,
    pub choice_mode: u8,
    pub show_title_on_tray: bool,
    pub launch_at_login: bool,
    pub hide_at_launch: bool,
    pub send_usage_data: bool,
    pub cmd_after_hosts_apply: String,
    pub remove_duplicate_records: bool,
    pub hide_dock_icon: bool,
    pub multi_chose_folder_switch_all: bool,
    pub tray_mini_window: bool,
    pub find_is_regexp: bool,
    pub find_is_ignore_case: bool,
    pub find_result_column_widths: Vec<u32>,
    pub use_proxy: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: u32,
    pub refresh_remote_hosts_on_startup: bool,
    pub http_api_on: bool,
    pub http_api_only_local: bool,
    pub auto_check_update: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            left_panel_show: true,
            left_panel_width: 270,
            right_panel_show: false,
            right_panel_width: 240,
            use_system_window_frame: false,
            write_mode: "append".to_string(),
            history_limit: 50,
            locale: None,
            theme: "system".to_string(),
            choice_mode: 2,
            show_title_on_tray: false,
            launch_at_login: false,
            hide_at_launch: false,
            send_usage_data: false,
            cmd_after_hosts_apply: String::new(),
            remove_duplicate_records: false,
            hide_dock_icon: false,
            multi_chose_folder_switch_all: false,
            tray_mini_window: true,
            find_is_regexp: false,
            find_is_ignore_case: false,
            find_result_column_widths: Vec::new(),
            use_proxy: false,
            proxy_protocol: "http".to_string(),
            proxy_host: String::new(),
            proxy_port: 0,
            refresh_remote_hosts_on_startup: false,
            http_api_on: false,
            http_api_only_local: true,
            auto_check_update: true,
        }
    }
}

impl AppConfig {
    fn normalize(&mut self) {
        if !matches!(self.theme.as_str(), "light" | "dark" | "system") {
            self.theme = "system".to_string();
        }
        if !matches!(self.proxy_protocol.as_str(), "http" | "https" | "socks5") {
            self.proxy_protocol = "http".to_string();
        }
        if self.proxy_port > MAX_PROXY_PORT {
            self.proxy_port = MAX_PROXY_PORT;
        }
    }

    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<AppConfig>(&bytes) {
                Ok(mut cfg) => {
                    cfg.normalize();
                    cfg
                }
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let mut value = serde_json::to_value(self)
            .map_err(|e| StorageError::serialize(path.display().to_string(), e))?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("format".into(), json!(CONFIG_FORMAT));
            obj.insert("schemaVersion".into(), json!(CONFIG_SCHEMA_VERSION));
        }
        let json = serde_json::to_vec_pretty(&value)
            .map_err(|e| StorageError::serialize(path.display().to_string(), e))?;
        atomic_write(path, &json)
    }

    pub fn apply_partial(&mut self, patch: &Value) -> Result<(), StorageError> {
        let patch_obj = patch.as_object().ok_or_else(|| StorageError::InvalidConfigValue {
            key: "*".into(),
            reason: "expected object".into(),
        })?;
        let mut merged = serde_json::to_value(&*self).expect("config serializes");
        let merged_obj = merged.as_object_mut().expect("object");
        for (k, v) in patch_obj {
            if !merged_obj.contains_key(k) {
                return Err(StorageError::UnknownConfigKey { key: k.clone() });
            }
            merged_obj.insert(k.clone(), v.clone());
        }
        let mut next: AppConfig =
            serde_json::from_value(merged).map_err(|e| StorageError::InvalidConfigValue {
                key: "*".into(),
                reason: e.to_string(),
            })?;
        next.normalize();
        *self = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_write_mode_is_append() {
        assert_eq!(AppConfig::default().write_mode, "append");
    }

    #[test]
    fn save_and_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        let mut cfg = AppConfig::default();
        cfg.theme = "dark".into();
        cfg.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path).theme, "dark");
    }
}
