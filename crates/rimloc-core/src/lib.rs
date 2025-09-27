use color_eyre::eyre::eyre;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Workspace-wide result alias.
pub type Result<T> = color_eyre::eyre::Result<T>;

/// Schema version for RimLoc data outputs (JSON/PO headers).
pub const RIMLOC_SCHEMA_VERSION: u32 = 1;

pub mod placeholders {
    /// Return true if a percent placeholder looks suspicious (e.g., single '%' not matching printf pattern).
    pub fn is_bad_percent(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                // literal '%%' is ok
                if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                    i += 2;
                    continue;
                }
                // Accept printf-like tokens: %d, %s, %i, %f with optional position/zero/width
                // This is a light check consistent with the validator logic.
                let mut j = i + 1;
                // optional positional like 1$
                while j < bytes.len() && bytes[j].is_ascii_digit() { j += 1; }
                if j < bytes.len() && bytes[j] == b'$' { j += 1; }
                // optional zero or width digits
                while j < bytes.len() && (bytes[j] == b'0' || bytes[j].is_ascii_digit()) { j += 1; }
                if j < bytes.len() && matches!(bytes[j] as char, 'd'|'s'|'i'|'f') {
                    i = j + 1;
                    continue;
                }
                return true;
            }
            i += 1;
        }
        false
    }
}

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

/// Parse a minimal subset of PO syntax used across the workspace.
/// Supports single-line `msgid`/`msgstr` pairs and optional reference lines (`#: ...`).
pub fn parse_simple_po(input: &str) -> Result<Vec<PoEntry>> {
    let mut entries = Vec::new();
    let mut cur_ref: Option<String> = None;
    let mut cur_id: Option<String> = None;

    fn unquote(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#:") {
            cur_ref = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("msgid") {
            let eq = rest
                .trim_start()
                .strip_prefix(' ')
                .unwrap_or(rest)
                .trim_start_matches('=');
            cur_id = Some(unquote(eq.trim()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("msgstr") {
            let eq = rest
                .trim_start()
                .strip_prefix(' ')
                .unwrap_or(rest)
                .trim_start_matches('=');
            let val = unquote(eq.trim());
            if let Some(id) = cur_id.take() {
                entries.push(PoEntry {
                    key: id,
                    value: val,
                    reference: cur_ref.take(),
                });
            } else {
                return Err(eyre!("Malformed PO entry: msgstr without msgid"));
            }
        }
    }

    Ok(entries)
}
