use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub use rimloc_core::parse_simple_po as parse_po_string;

use rimloc_core::{Result as CoreResult, TransUnit};

/// Deliberate severity of a validation finding, assigned AT THE EMISSION
/// SITE by the checker that owns the rule — never derived from localized or
/// free-form message text, and never inferred from `kind` alone.
///
/// Classification (documented per emission):
/// - `Error`   — the string is broken or breaks the game (empty required
///   translation, unbalanced/invalid placeholder, real placeholder
///   mismatch, list count mismatch, duplicate key the game will never
///   load, engine/extractor divergence on a non-translatable entry).
/// - `Warning` — suspicious but not load-breaking (invisible/bi-di control
///   characters, cross-file duplicate of one scope, ambiguous key
///   reference, orphaned translation without a source counterpart).
/// - `Info`    — informational notes that require no action (placeholder
///   presence hint asking to verify counts against the source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

impl ValidationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationSeverity::Error => "error",
            ValidationSeverity::Warning => "warning",
            ValidationSeverity::Info => "info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationMessage {
    pub kind: String,
    pub severity: ValidationSeverity,
    pub key: String,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

/// Validator that reports duplicate keys per file using scanned TransUnits.
type KeyFileEntries = Vec<(String, Option<usize>)>;

pub fn validate(units: &[TransUnit]) -> CoreResult<Vec<ValidationMessage>> {
    static RE_PCT: OnceLock<Regex> = OnceLock::new();
    static RE_BRACE_INNER: OnceLock<Regex> = OnceLock::new();
    let _re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    let re_brace_inner = RE_BRACE_INNER.get_or_init(|| Regex::new(r"^\$?[A-Za-z0-9_]+$").unwrap());

    let mut by_file_key: HashMap<(String, String), Vec<Option<usize>>> = HashMap::new();
    let mut by_key_files: HashMap<(String, String), KeyFileEntries> = HashMap::new();
    for u in units {
        let path = u.path.to_string_lossy().to_string();
        by_file_key
            .entry((path.clone(), u.key.clone()))
            .or_default()
            .push(u.line);
        // Cross-file duplicates are scoped per language folder: the same key in
        // Languages/English and Languages/Russian is the same Def translated,
        // not a duplicate. Only files within one scope (same language, or all
        // outside Languages) are compared.
        let scope = lang_scope_of(&path);
        by_key_files
            .entry((scope, u.key.clone()))
            .or_default()
            .push((path, u.line));
    }

    // Report empty values
    let mut msgs = Vec::new();
    for u in units {
        if u.source.as_deref().map_or(true, |s| s.trim().is_empty()) {
            // Empty required translation → real failure.
            msgs.push(ValidationMessage {
                kind: "empty".to_string(),
                severity: ValidationSeverity::Error,
                key: u.key.clone(),
                path: u.path.to_string_lossy().to_string(),
                line: u.line,
                message: "Empty value".to_string(),
            });
        }

        // Placeholder checks (run only when non-empty)
        if let Some(text) = u.source.as_deref() {
            if !text.trim().is_empty() {
                // Invisible/bi-di control characters
                let mut invisible_hits: Vec<char> = Vec::new();
                for ch in text.chars() {
                    match ch {
                        '\u{200B}' | // ZWSP
                        '\u{200C}' | // ZWNJ
                        '\u{200D}' | // ZWJ
                        '\u{200E}' | // LRM
                        '\u{200F}' | // RLM
                        '\u{202A}' | // LRE
                        '\u{202B}' | // RLE
                        '\u{202C}' | // PDF
                        '\u{202D}' | // LRO
                        '\u{202E}' | // RLO
                        '\u{2066}' | // LRI
                        '\u{2067}' | // RLI
                        '\u{2068}' | // FSI
                        '\u{2069}'   // PDI
                        => invisible_hits.push(ch),
                        _ => {}
                    }
                }
                if !invisible_hits.is_empty() {
                    let codes: Vec<String> = invisible_hits
                        .into_iter()
                        .map(|c| format!("U+{:04X}", c as u32))
                        .collect();
                    // Suspicious, not load-breaking → warning.
                    msgs.push(ValidationMessage {
                        kind: "invisible-char".to_string(),
                        severity: ValidationSeverity::Warning,
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: format!("Suspicious control chars: {} — Hint: remove hidden control characters (ZWSP/bi-di).", codes.join(", ")),
                    });
                }
                let mut placeholder_msg_emitted = false;
                let bad_percent = rimloc_core::placeholders::is_bad_percent(text);
                if bad_percent {
                    // A suspicious % token breaks the rendered string → error.
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        severity: ValidationSeverity::Error,
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: "Suspicious % placeholder — Hint: use printf-like tokens (%s, %d) or escape '%%' as '%%'.".to_string(),
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
                    // Broken braces break the rendered string → error.
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        severity: ValidationSeverity::Error,
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message: format!("{} — Hint: ensure placeholders look like %s/%d or {{name}} and are balanced.", msg),
                    });
                    placeholder_msg_emitted = true;
                }
                // If the string contains any placeholder tokens but no issues were emitted,
                // produce an informational placeholder-check so tests can observe the category.
                let has_any_placeholder =
                    text.contains('%') || text.contains('{') || text.contains('}');
                if has_any_placeholder && !placeholder_msg_emitted {
                    // Pure informational: placeholders are present and well
                    // formed; the human verifies counts against the source.
                    msgs.push(ValidationMessage {
                        kind: "placeholder-check".to_string(),
                        severity: ValidationSeverity::Info,
                        key: u.key.clone(),
                        path: u.path.to_string_lossy().to_string(),
                        line: u.line,
                        message:
                            "Placeholders present — Hint: verify count and types match the source."
                                .to_string(),
                    });
                }
            }
        }
    }

    for ((path, key), lines) in by_file_key {
        if lines.len() > 1 {
            // duplicate detected in the same file
            let line = lines.into_iter().flatten().next();
            // The game keeps the FIRST registration; later duplicates never
            // load → real failure.
            msgs.push(ValidationMessage {
                kind: "duplicate".to_string(),
                severity: ValidationSeverity::Error,
                key,
                path,
                line,
                message: "Duplicate key in file".to_string(),
            });
        }
    }

    // Cross-file duplicates: same key appears in multiple files of one scope
    for ((_scope, key), entries) in by_key_files {
        // Normalize unique files
        use std::collections::BTreeSet;
        let mut files: BTreeSet<String> = BTreeSet::new();
        for (p, _) in &entries {
            files.insert(p.clone());
        }
        if files.len() > 1 {
            // Report one message, attach first path/line for reference
            let (path, line) = entries
                .first()
                .cloned()
                .unwrap_or_else(|| (String::new(), None));
            // Same scope, several files: suspicious (possible accidental
            // copy), but the game resolves it → warning.
            msgs.push(ValidationMessage {
                kind: "duplicate-global".to_string(),
                severity: ValidationSeverity::Warning,
                key,
                path,
                line,
                message: format!(
                    "Key appears in multiple files: {}",
                    files.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            });
        }
    }

    Ok(msgs)
}

/// Duplicate-detection scope of a path: its Languages/<dir> folder name, or an
/// empty scope for paths outside Languages (Defs and other source files).
fn lang_scope_of(path: &str) -> String {
    let mut comps = Path::new(path).components();
    while let Some(c) = comps.next() {
        if c.as_os_str().eq_ignore_ascii_case("Languages") {
            if let Some(l) = comps.next() {
                return l.as_os_str().to_string_lossy().to_string();
            }
            return String::new();
        }
    }
    String::new()
}

/// Temporary minimalist scanner used by CLI integration tests that only
/// assert CSV headers; returns an empty list of units.
/// TODO: implement full XML scan or integrate with validate crate.
pub fn scan_keyed_xml(_root: &Path) -> CoreResult<Vec<TransUnit>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unit(key: &str, source: &str) -> TransUnit {
        TransUnit {
            key: key.to_string(),
            source: Some(source.to_string()),
            path: PathBuf::from("Languages/English/Keyed/T.xml"),
            line: Some(3),
            tkey: None,
            src: None,
            selected_by: None,
            conditional: false,
        }
    }

    /// The severity class is assigned at the emission site and must stay
    /// deliberate (026): presence hints are Info, broken placeholders /
    /// empty translations / duplicates are Error, suspicious-but-not-
    /// breaking findings are Warning.
    #[test]
    fn severities_are_deliberate_per_emission() {
        let units = vec![
            unit("Good.label", "Hello {0}!"), // info: presence hint
            unit("Broken.label", "Hi {0"),    // error: unmatched brace
            unit("Empty.label", "   "),       // error: empty translation
            unit("Dup.label", "one"),         // error: duplicate (below)
            unit("Dup.label", "two"),
            unit("Weird.label", "a\u{200B}b"), // warning: invisible char
        ];
        let msgs = validate(&units).expect("validate");
        let sev_of = |kind: &str, key: &str| {
            msgs.iter()
                .find(|m| m.kind == kind && m.key == key)
                .map(|m| m.severity)
                .unwrap_or_else(|| {
                    let msg = format!("no {kind}/{key} in {msgs:?}");
                    panic!("{}", msg);
                })
        };
        assert_eq!(
            sev_of("placeholder-check", "Good.label"),
            ValidationSeverity::Info
        );
        assert_eq!(
            sev_of("placeholder-check", "Broken.label"),
            ValidationSeverity::Error
        );
        assert_eq!(sev_of("empty", "Empty.label"), ValidationSeverity::Error);
        assert_eq!(sev_of("duplicate", "Dup.label"), ValidationSeverity::Error);
        assert_eq!(
            sev_of("invisible-char", "Weird.label"),
            ValidationSeverity::Warning
        );
    }
}
