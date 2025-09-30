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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred: Option<InferredDefInjected>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InferredDefInjected {
    pub def_type: String,
    pub def_name: String,
    pub field_path: String,
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
                        let inferred = xpath
                            .as_deref()
                            .and_then(|xp| infer_definj_from_xpath(xp, &tag_path));
                        out.push(PatchTextCandidate {
                            operation_class: class.clone(),
                            xpath: xpath.clone(),
                            tag_path,
                            value: val,
                            source_file: p.to_path_buf(),
                            inferred,
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

fn infer_definj_from_xpath(xpath: &str, tag_path: &str) -> Option<InferredDefInjected> {
    // Heuristic: looking for .../Defs/<DefType>[defName='X' or @defName='X' or @Name='X']/rest/of/path
    // Then map rest/of/path + tag_path into dot path; normalize li's
    let xp = xpath.replace("\\", "/");
    // find segment after /Defs/
    let idx = xp.find("/Defs/")?;
    let after_defs = &xp[idx + "/Defs/".len()..];
    // take the first segment (DefType[...] or DefType)
    let seg_end = after_defs.find('/').unwrap_or(after_defs.len());
    let first = &after_defs[..seg_end];
    // capture def_type and condition
    // patterns like ThingDef[defName='Foo'] or ThingDef[@Name='Bar']
    let re = regex::Regex::new(r"^(?P<ty>[^\[]+)(?P<cond>\[[^\]]+\])?").ok()?;
    let caps = re.captures(first)?;
    let def_type = caps.name("ty")?.as_str().to_string();
    let cond = caps.name("cond").map(|m| m.as_str().to_string()).unwrap_or_default();
    if def_type.trim().is_empty() {
        return None;
    }
    // try extract def_name from condition
    let name_re = regex::Regex::new(r"(?i)(?:@?defName|@?Name)\s*=\s*'([^']+)'").ok()?;
    let def_name = name_re
        .captures(&cond)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())?;
    // the remainder path after the first segment
    let rest = if seg_end < after_defs.len() {
        &after_defs[seg_end + 1..]
    } else {
        ""
    };
    // Build field path: rest segments + tag_path segments
    let mut segs: Vec<String> = Vec::new();
    for part in rest.split('/') {
        if part.is_empty() {
            continue;
        }
        // normalize predicates [..] to li
        let name = part.split('[').next().unwrap_or("");
        if name.eq_ignore_ascii_case("li") {
            segs.push("li".to_string());
        } else if name.is_empty() {
            continue;
        } else {
            segs.push(name.to_string());
        }
    }
    for part in tag_path.split('.') {
        if part.is_empty() {
            continue;
        }
        let name = part.split('[').next().unwrap_or("");
        if name.eq_ignore_ascii_case("li") {
            segs.push("li".to_string());
        } else if name.is_empty() {
            continue;
        } else {
            segs.push(name.to_string());
        }
    }
    if segs.is_empty() {
        return None;
    }
    let field_path = segs.join(".");
    Some(InferredDefInjected {
        def_type,
        def_name,
        field_path,
    })
}
