use crate::{Result, TransUnit};
use roxmltree::Document;
use std::path::Path;
use walkdir::WalkDir;

/// Built-in parser plugin that extracts Keyed-like pairs from:
/// - XmlExtensions.SettingsMenuDef (tKey/tKeyTip combined with label/text/tooltip)
/// - Any element carrying attribute TKey (value taken from element text)
pub struct XmlExtensionsSettingsPlugin;

impl super::super::plugins::ParserPlugin for XmlExtensionsSettingsPlugin {
    fn id(&self) -> &'static str {
        "xml_ext-settings"
    }

    fn scan_units(&self, root: &Path) -> Result<Vec<TransUnit>> {
        let mut out = Vec::new();

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            if !p
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xml"))
                .unwrap_or(false)
            {
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

            // XmlExtensions.SettingsMenuDef
            for def in root_el.descendants().filter(|n| {
                n.is_element()
                    && n.tag_name()
                        .name()
                        .eq_ignore_ascii_case("XmlExtensions.SettingsMenuDef")
            }) {
                let label = def
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("label"))
                    .and_then(|n| n.text())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let text_or_tip = def
                    .descendants()
                    .find(|n| {
                        n.is_element()
                            && (n.tag_name().name().eq_ignore_ascii_case("text")
                                || n.tag_name().name().eq_ignore_ascii_case("tooltip"))
                    })
                    .and_then(|n| n.text())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let tkey = def
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("tKey"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());
                let tkey_tip = def
                    .descendants()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("tKeyTip"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());

                if let Some(k) = tkey.as_ref() {
                    if !k.is_empty() && !label.is_empty() {
                        out.push(TransUnit {
                            key: k.clone(),
                            source: Some(label.clone()),
                            path: p.to_path_buf(),
                            line: Some(line_for(def.range().start).unwrap_or(1)),
                            tkey: None,
                        });
                    }
                }
                if let Some(k) = tkey_tip.as_ref() {
                    if !k.is_empty() && !text_or_tip.is_empty() {
                        out.push(TransUnit {
                            key: k.clone(),
                            source: Some(text_or_tip.clone()),
                            path: p.to_path_buf(),
                            line: Some(line_for(def.range().start).unwrap_or(1)),
                            tkey: None,
                        });
                    }
                }
            }

            // Generic TKey="..."
            for node in root_el.descendants().filter(|n| n.is_element()) {
                if let Some(k) = node.attribute("TKey") {
                    let k = k.trim();
                    if !k.is_empty() {
                        if let Some(val) = node.text().map(str::trim).filter(|t| !t.is_empty()) {
                            out.push(TransUnit {
                                key: k.to_string(),
                                source: Some(val.to_string()),
                                path: p.to_path_buf(),
                                line: Some(line_for(node.range().start).unwrap_or(1)),
                                tkey: None,
                            });
                        }
                    }
                }
            }
        }

        // Deterministic ordering
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
