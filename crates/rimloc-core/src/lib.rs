use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransUnit {
    pub key: String,
    pub source: Option<String>, // исходное значение/подсказка
    pub path: PathBuf,          // файл, где найден ключ
    pub line: Option<u32>,      // строка в файле, если известна
}

#[derive(Debug, thiserror::Error)]
pub enum RimLocError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML error: {0}")]
    Xml(String),
    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RimLocError>;
