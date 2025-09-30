use crate::Result;
use roxmltree::Document;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchTextCandidate {
    pub operation_class: String,
    pub xpath: Option<String>,
    pub tag_path: String,
    pub value: String,
    pub source_file: PathBuf,
}

fn collect_texts(node: roxmltree::Node, path_prefix: &str, out: &mut Vec<(String, String)>) {
    for child in node.children().filter(|n| n.is_element()) {
        let name = child.tag_name().name();
        let path = if path_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", path_prefix, name)
        };
        if let Some(t) = child.text().map(str::trim) {
            if !t.is_empty() {
                out.push((path.clone(), t.to_string()));
            }
        }
        collect_texts(child, &path, out);
    }
}

/// Scan PatchOperations in Patches/ and collect human-readable text values introduced via <value> nodes.
pub fn scan_patches_texts(root: &Path, min_len: usize) -> Result<Vec<PatchTextCandidate>> {
    let mut out = Vec::new();
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
        let s = p.to_string_lossy();
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
        let root_el = doc.root_element();
        for op in root_el.descendants().filter(|n| n.is_element()) {
            let tag = op.tag_name().name();
            if tag.eq_ignore_ascii_case("Operation") {
                let class = op.attribute("Class").unwrap_or("").to_string();
                if class.is_empty() {
                    continue;
                }
                let xpath = op
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("xpath"))
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string());
                if let Some(value_node) = op
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("value"))
                {
                    // Parse inner fragment as XML if possible; otherwise treat as plain text
                    let mut texts = Vec::new();
                    let frag = value_node.text().unwrap_or("").trim();
                    if !frag.is_empty() {
                        let wrapped = format!("<Root>{}</Root>", frag);
                        if let Ok(fdoc) = Document::parse(&wrapped) {
                            collect_texts(fdoc.root_element(), "", &mut texts);
                        }
                    }
                    for (tag_path, val) in texts {
                        if val.len() < min_len {
                            continue;
                        }
                        out.push(PatchTextCandidate {
                            operation_class: class.clone(),
                            xpath: xpath.clone(),
                            tag_path,
                            value: val,
                            source_file: p.to_path_buf(),
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

