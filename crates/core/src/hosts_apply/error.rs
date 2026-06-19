use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error(transparent)]
    Storage(#[from] crate::storage::error::StorageError),
    #[error("elevation failed: {0}")]
    Elevation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cancelled")]
    Cancelled,
}

impl ApplyError {
    /// 面向用户的错误说明（GUI / CLI 提示）。
    pub fn user_message(&self) -> String {
        match self {
            Self::Cancelled => "已取消管理员授权，hosts 文件未写入。".to_string(),
            Self::Elevation(msg) => {
                let lower = msg.to_ascii_lowercase();
                if lower.contains("user canceled")
                    || lower.contains("cancelled")
                    || msg.contains("-128")
                {
                    "已取消管理员授权，hosts 文件未写入。".to_string()
                } else {
                    format!("写入 hosts 需要管理员权限，但提权失败：{msg}")
                }
            }
            Self::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                "没有权限写入 hosts 文件。macOS 上应弹出密码框；若未弹出，请用管理员权限运行应用。"
                    .to_string()
            }
            Self::Io(e) => format!("写入 hosts 文件失败：{e}"),
            Self::Storage(e) => format!("读写数据失败：{e}"),
            Self::Json(e) => format!("解析数据失败：{e}"),
        }
    }
}
