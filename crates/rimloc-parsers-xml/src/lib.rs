pub use rimloc_core::parse_simple_po as parse_po_string;

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
