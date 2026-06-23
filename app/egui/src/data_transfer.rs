//! 导入/导出（对齐 SwitchHosts `exportData` / `importData` / `importDataFromUrl`）。

use std::path::{Path, PathBuf};

use chrono::Local;
use rfd::FileDialog;
use switch_hosts_core::import_export::{
    export_to_file, import_backup_bytes, read_file_with_limit, ERR_INVALID_DATA,
    ERR_INVALID_DATA_KEY, ERR_INVALID_V3_DATA, ERR_NEW_VERSION, ERR_PARSE,
    MAX_IMPORT_BACKUP_BYTES,
};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;

use crate::remote_refresh::client_config_from_app;
use crate::shared_runtime;

pub fn default_export_file_name() -> String {
    format!("switchhosts_{}.json", Local::now().format("%Y%m%d_%H%M%S%.3f"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportResult {
    Cancelled,
    Failed,
    Success(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportResult {
    Cancelled,
    Success,
    /// Soft error code (`parse_error`, `invalid_data`, HTTP status, etc.).
    SoftError(String),
    HardError(String),
}

pub fn import_error_message(code: &str) -> String {
    match code {
        ERR_PARSE => "无法解析备份文件，请确认是有效的 JSON 格式。".into(),
        ERR_INVALID_DATA => "无效的备份数据。".into(),
        ERR_INVALID_V3_DATA => "无效的 v3 备份数据。".into(),
        ERR_INVALID_DATA_KEY => "无效的 v4 备份数据（缺少 data 字段）。".into(),
        ERR_NEW_VERSION => "备份版本过新，请升级 SwitchHostsRust。".into(),
        other if other.starts_with("error_") => format!("下载失败（HTTP {other}）。"),
        other => format!("导入失败 [{other}]。"),
    }
}

pub fn run_export_dialog(paths: &AppPaths) -> ExportResult {
    let picked = FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name(&default_export_file_name())
        .save_file();

    let Some(dest) = picked else {
        return ExportResult::Cancelled;
    };

    match export_to_file(&dest, paths) {
        Ok(()) => ExportResult::Success(dest),
        Err(err) => {
            tracing::warn!("export failed: {err}");
            ExportResult::Failed
        }
    }
}

pub fn run_import_dialog(paths: &AppPaths) -> ImportResult {
    let picked = FileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file();

    let Some(src) = picked else {
        return ImportResult::Cancelled;
    };

    import_backup_file(paths, &src)
}

pub fn import_backup_file(paths: &AppPaths, src: &Path) -> ImportResult {
    let bytes = match read_file_with_limit(src, MAX_IMPORT_BACKUP_BYTES) {
        Ok(b) => b,
        Err(err) => {
            tracing::warn!("import read failed ({}): {err}", src.display());
            return ImportResult::SoftError(ERR_PARSE.into());
        }
    };

    match import_backup_bytes(&bytes, paths) {
        Ok(value) => match value.as_bool() {
            Some(true) => ImportResult::Success,
            _ => ImportResult::SoftError(
                value
                    .as_str()
                    .unwrap_or(ERR_INVALID_DATA)
                    .to_string(),
            ),
        },
        Err(err) => ImportResult::HardError(err.to_string()),
    }
}

pub fn import_from_url(url: &str, paths: &AppPaths, config: &AppConfig) -> ImportResult {
    let client_cfg = client_config_from_app(config);

    let bytes = match shared_runtime::fetch(&client_cfg, url) {
        Ok(text) => text.into_bytes(),
        Err(err) => {
            tracing::warn!("import-from-url fetch failed ({url}): {err}");
            return ImportResult::SoftError(err.to_string());
        }
    };

    if bytes.len() > MAX_IMPORT_BACKUP_BYTES {
        return ImportResult::SoftError(ERR_PARSE.into());
    }

    match import_backup_bytes(&bytes, paths) {
        Ok(value) => match value.as_bool() {
            Some(true) => ImportResult::Success,
            _ => ImportResult::SoftError(
                value
                    .as_str()
                    .unwrap_or(ERR_INVALID_DATA)
                    .to_string(),
            ),
        },
        Err(err) => ImportResult::HardError(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Timelike};

    fn export_file_name_for(now: chrono::DateTime<chrono::Local>) -> String {
        format!("switchhosts_{}.json", now.format("%Y%m%d_%H%M%S%.3f"))
    }

    #[test]
    fn export_file_name_includes_millisecond_timestamp() {
        let now = chrono::Local
            .with_ymd_and_hms(2026, 5, 9, 12, 14, 36)
            .single()
            .expect("test timestamp")
            .with_nanosecond(789_000_000)
            .expect("test nanosecond");

        assert_eq!(
            export_file_name_for(now),
            "switchhosts_20260509_121436.789.json"
        );
    }
}
