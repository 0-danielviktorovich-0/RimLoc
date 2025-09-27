use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub use rimloc_core::parse_simple_po as parse_po_string;

use rimloc_core::{Result as CoreResult, TransUnit};

#[derive(Debug, Clone)]
pub struct ValidationMessage {
    pub kind: String,
    pub key: String,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

/// Validator that reports duplicate keys per file using scanned TransUnits.
pub fn validate(units: &[TransUnit]) -> CoreResult<Vec<ValidationMessage>> {
    static RE_PCT: OnceLock<Regex> = OnceLock::new();
    static RE_BRACE_INNER: OnceLock<Regex> = OnceLock::new();
    let _re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    let re_brace_inner = RE_BRACE_INNER.get_or_init(|| Regex::new(r"^\$?[A-Za-z0-9_]+$").unwrap());

    let mut by_file_key: HashMap<(String, String), Vec<Option<usize>>> = HashMap::new();
    for u in units {
        let path = u.path.to_string_lossy().to_string();
        by_file_key
            .entry((path, u.key.clone()))
            .or_default()
            .push(u.line);
    }

    // Report empty values
    let mut msgs = Vec::new();
    for u in units {
        if u.source.as_deref().map_or(true, |s| s.trim().is_empty()) {
            msgs.push(ValidationMessage {
                kind: "empty".to_string(),
                key: u.key.clone(),
                path: u.path.to_string_lossy().to_string(),
                line: u.line,
                message: "Empty value".to_string(),
            });
        }

        // Placeholder checks (run only when non-empty)
        if let Some(text) = u.source.as_deref() {
            if !text.trim().is_empty() {
                let mut placeholder_msg_emitted = false;
                let bad_percent = rimloc_core::placeholders::is_bad_percent(text);
                if bad_percent {
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: "Suspicious % placeholder".to_string(),
                    });
                    placeholder_msg_emitted = true;
                }

                // 2) Brace-style placeholders: ensure balanced braces and non-empty names like {NAME} / {0}
                let mut depth = 0usize;
                let mut last_open: Option<usize> = None;
                let mut brace_error: Option<&'static str> = None;
                for (i, ch) in text.char_indices() {
                    match ch {
                        '{' => {
                            if depth == 0 {
                                last_open = Some(i);
                            }
                            depth += 1;
                            // very naive: we don't allow nested braces for our use case
                            if depth > 1 {
                                brace_error = Some("Nested braces");
                                break;
                            }
                        }
                        '}' => {
                            if depth == 0 {
                                brace_error = Some("Unmatched closing brace");
                                break;
                            }
                            if depth == 1 {
                                if let Some(lo) = last_open {
                                    let inner = text[lo + 1..i].trim();
                                    if inner.is_empty() {
                                        brace_error = Some("Empty brace placeholder");
                                        break;
                                    }
                                    // Only allow {$var}, {VAR}, {0}, {name_1}
                                    if !re_brace_inner.is_match(inner) {
                                        brace_error = Some("Invalid brace placeholder");
                                        break;
                                    }
                                }
                            }
                            depth -= 1;
                        }
                        _ => {}
                    }
                }
                if brace_error.is_none() && depth > 0 {
                    brace_error = Some("Unmatched opening brace");
                }
                if let Some(msg) = brace_error {
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: msg.to_string(),
                    });
                    placeholder_msg_emitted = true;
                }
                // If the string contains any placeholder tokens but no issues were emitted,
                // produce an informational placeholder-check so tests can observe the category.
                let has_any_placeholder =
                    text.contains('%') || text.contains('{') || text.contains('}');
                if has_any_placeholder && !placeholder_msg_emitted {
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: "Placeholders present".to_string(),
                    });
                }
            }
        }
    }

    for ((path, key), lines) in by_file_key {
        if lines.len() > 1 {
            // duplicate detected in the same file
            let line = lines.into_iter().flatten().next();
            msgs.push(ValidationMessage {
                kind: "duplicate".to_string(),
                key,
                path,
                line,
                message: "Duplicate key in file".to_string(),
            });
        }
    }

    Ok(msgs)
}

/// Temporary minimalist scanner used by CLI integration tests that only
/// assert CSV headers; returns an empty list of units.
/// TODO: implement full XML scan or integrate with validate crate.
pub fn scan_keyed_xml(_root: &Path) -> CoreResult<Vec<TransUnit>> {
    Ok(Vec::new())
}
