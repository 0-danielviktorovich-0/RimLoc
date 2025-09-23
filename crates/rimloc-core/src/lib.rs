use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Workspace-wide result alias.
pub type Result<T> = color_eyre::eyre::Result<T>;

/// Minimal unit used across crates to represent a single translation entry
/// scanned from RimWorld XML (Keyed/DefInjected) or produced by tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransUnit {
    pub key: String,
    /// Source string (may be missing for keys detected without text)
    pub source: Option<String>,
    /// Absolute or relative path to the file where this unit comes from
    pub path: PathBuf,
    /// 1-based line number if available
    pub line: Option<usize>,
}

/// Simple PO entry used by import/export utilities and tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoEntry {
    pub key: String,
    pub value: String,
    /// Optional reference like
    /// "…/Languages/English/Keyed/Some.xml:42" used to reconstruct paths.
    pub reference: Option<String>,
}

/// Keep a lightweight error type for crates that still import it.
#[derive(Debug, Error)]
pub enum RimLocError {
    #[error("{0}")]
    Other(String),
}
