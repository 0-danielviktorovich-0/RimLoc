mod po {
    use color_eyre::eyre::{eyre, Result};
    use rimloc_core::PoEntry;

    /// Minimal PO parser sufficient for our test fixtures.
    /// Supports single-line msgid/msgstr and optional reference lines starting with `#: `.
    pub fn parse_po_string(s: &str) -> Result<Vec<PoEntry>> {
        let mut out = Vec::new();
        let mut cur_ref: Option<String> = None;
        let mut cur_id: Option<String> = None;

        fn unquote(q: &str) -> String {
            let t = q.trim();
            if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
                t[1..t.len() - 1].to_string()
            } else {
                t.to_string()
            }
        }

        for line in s.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if let Some(rest) = l.strip_prefix("#:") {
                cur_ref = Some(rest.trim().to_string());
                continue;
            }
            if let Some(rest) = l.strip_prefix("msgid") {
                let eq = rest
                    .trim_start()
                    .strip_prefix(' ')
                    .unwrap_or(rest)
                    .trim_start_matches('=');
                cur_id = Some(unquote(eq.trim()));
                continue;
            }
            if let Some(rest) = l.strip_prefix("msgstr") {
                let eq = rest
                    .trim_start()
                    .strip_prefix(' ')
                    .unwrap_or(rest)
                    .trim_start_matches('=');
                let val = unquote(eq.trim());
                // finalize entry when we have both id and str
                if let Some(id) = cur_id.take() {
                    out.push(PoEntry {
                        key: id,
                        value: val,
                        reference: cur_ref.take(),
                    });
                } else {
                    return Err(eyre!("Malformed PO entry: msgstr without msgid"));
                }
                continue;
            }
        }
        Ok(out)
    }
}

pub use po::parse_po_string;

use rimloc_core::{Result as CoreResult, TransUnit};
use std::fs;
use std::path::Path;

/// Very small XML scanner for RimWorld "Keyed" XML files.
/// It walks `root` and finds files that match `*/Languages/*/Keyed/*.xml`.
/// For every `<Key>Value</Key>` pair found, it produces a `TransUnit` with
/// the key name, text value, file path and (approximate) line number.
pub fn scan_keyed_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<TransUnit> = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Languages/<Locale>/Keyed/....xml
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Languages/") || p_str.contains("\\Languages\\")) {
            continue;
        }
        if !(p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\")) {
            continue;
        }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for (idx, line) in content.lines().enumerate() {
            let t = line.trim();
            if !t.starts_with('<') {
                continue;
            }
            // Find first '>' after '<'
            let gt = t.find('>');
            if gt.is_none() {
                continue;
            }
            let gt = gt.unwrap();
            let tag = t[1..gt].trim();
            if tag.is_empty() {
                continue;
            }
            if !tag
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
            {
                continue;
            }
            // Find last "</" in the line
            let close_start = t.rfind("</");
            if close_start.is_none() {
                continue;
            }
            let close_start = close_start.unwrap();
            let close_gt = t[close_start..].find('>');
            if close_gt.is_none() {
                continue;
            }
            let close_gt = close_start + close_gt.unwrap();
            let end_tag = t[close_start + 2..close_gt].trim();
            if end_tag != tag {
                continue;
            }
            // Extract inner text
            let text = t[gt + 1..close_start].trim();
            out.push(TransUnit {
                key: tag.to_string(),
                source: Some(text.to_string()),
                path: p.to_path_buf(),
                line: Some(idx + 1),
            });
        }
    }

    Ok(out)
}
