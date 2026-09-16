use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("数据库错误：{0}")]
    Db(#[from] rusqlite::Error),

    #[error("剪贴板错误：{0}")]
    Clipboard(#[from] arboard::Error),

    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    #[error("Tauri 错误：{0}")]
    Tauri(#[from] tauri::Error),

    #[error("序列化错误：{0}")]
    Json(#[from] serde_json::Error),

    #[error("模拟按键失败：{0}")]
    Input(String),

    #[error("找不到条目：{0}")]
    NotFound(i64),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 前端只关心一句人话，不需要看 Rust 的 Debug 结构。
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
