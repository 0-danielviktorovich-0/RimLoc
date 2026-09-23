use crate::{plugins, Result, TransUnit};
use roxmltree::Document;
use std::path::Path;
use walkdir::WalkDir;

/// Built-in parser plugin for ModSettingsFramework operations in Patches/.
/// For each `<Operation Class="ModSettingsFramework.*">` it extracts:
/// - `<id>`: used as base key
/// - `<label>`: produces key `<id>` with this value
/// - `<tooltip>`: produces key `<id>Tooltip` with this value
pub struct ModSettingsFrameworkPlugin;

impl plugins::ParserPlugin for ModSettingsFrameworkPlugin {
    fn id(&self) -> &'static str {
        "msf-ops"
    }

    fn scan_units(&self, root: &Path) -> Result<Vec<TransUnit>> {
        let mut out = Vec::new();
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let is_xml = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xml"))
                .unwrap_or(false);
            if !is_xml {
                continue;
            }
            let s = p.to_string_lossy();
            // Keep scope to Patches/ to avoid noise
            if !(s.contains("/Patches/") || s.contains("\\Patches\\")) {
                continue;
            }
            let content = match std::fs::read_to_string(p) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let doc = match Document::parse(&content) {
                Ok(d) => d,
                Err(_) => continue,
            };
            // line mapping helper (approximate)
            let mut line_starts = Vec::with_capacity(256);
            line_starts.push(0usize);
            for (idx, _) in content.match_indices('\n') {
                line_starts.push(idx + 1);
            }
            let line_for = |ofs: usize| -> Option<usize> {
                if line_starts.is_empty() {
                    return None;
                }
                match line_starts.binary_search(&ofs) {
                    Ok(i) => Some(i + 1),
                    Err(i) if i > 0 => Some(i),
                    _ => Some(1),
                }
            };

            let root_el = doc.root_element();
            for op in root_el.descendants().filter(|n| n.is_element()) {
                let class = op.attribute("Class").unwrap_or("");
                if !class.starts_with("ModSettingsFramework.") {
                    continue;
                }
                let id = op
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("id"))
                    .and_then(|n| n.text())
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() {
                    continue;
                }
                let label = op
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("label"))
                    .and_then(|n| n.text())
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string();
                let tooltip = op
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("tooltip"))
                    .and_then(|n| n.text())
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string();

                let line = Some(line_for(op.range().start).unwrap_or(1));
                if !label.is_empty() {
                    out.push(TransUnit {
                        tkey: None,
                        key: id.clone(),
                        source: Some(label),
                        path: p.to_path_buf(),
                        line,
                    });
                }
                if !tooltip.is_empty() {
                    out.push(TransUnit {
                        tkey: None,
                        key: format!("{}{}", id, "Tooltip"),
                        source: Some(tooltip),
                        path: p.to_path_buf(),
                        line,
                    });
                }
            }
        }
        out.sort_by(|a, b| {
            (
                a.path.to_string_lossy(),
                a.line.unwrap_or(0),
                a.key.as_str(),
            )
                .cmp(&(
                    b.path.to_string_lossy(),
                    b.line.unwrap_or(0),
                    b.key.as_str(),
                ))
        });
        Ok(out)
    }
}
