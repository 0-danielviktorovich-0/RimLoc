use crate::Result;
use roxmltree::Document;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub def_type: String,
    pub def_name: String,
    pub field_path: String,
    pub value: String,
    pub source_file: PathBuf,
    pub confidence: Option<f32>,
}

pub fn scan_candidates(
    root: &Path,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    min_len: usize,
    blacklist: &[String],
) -> Result<Vec<Candidate>> {
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
        if let Some(base) = defs_root {
            if !p.starts_with(base) {
                continue;
            }
        } else {
            let s = p.to_string_lossy();
            if !(s.contains("/Defs/") || s.contains("\\Defs\\")) {
                continue;
            }
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
        for def_node in root_el.children().filter(|n| n.is_element()) {
            let def_type = def_node.tag_name().name().to_string();
            let def_name = def_node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            if def_name.is_empty() {
                continue;
            }

            // primary dict-based paths
            if let Some(paths) = dict.get(&def_type) {
                for path in paths {
                    // Parse segments with optional list-handle markers like li{h}
                    let raw_segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
                    // Collect (expanded_field_path, value) pairs using indices and optional pseudo-handles
                    let mut entries: Vec<(String, String)> = Vec::new();
                    collect_entries_by_path_with_handles(def_node, &raw_segs, &mut entries);
                    for (field_path_expanded, v) in entries {
                        let v = v.trim().to_string();
                        if v.len() < min_len { continue; }
                        if blacklist.iter().any(|b| field_path_expanded.eq_ignore_ascii_case(b)) { continue; }
                        out.push(Candidate {
                            def_type: def_type.clone(),
                            def_name: def_name.clone(),
                            field_path: field_path_expanded,
                            value: v,
                            source_file: p.to_path_buf(),
                            confidence: None,
                        });
                    }
                }
            }

            // light heuristics on immediate children
            const NAMES: &[&str] = &[
                "label",
                "labelShort",
                "labelPlural",
                "description",
                "jobString",
                "inspectString",
                "flavorText",
            ];
            for child in def_node.children().filter(|n| n.is_element()) {
                let name = child.tag_name().name();
                if !NAMES.iter().any(|n| name.eq_ignore_ascii_case(n)) {
                    continue;
                }
                if let Some(val) = child.text().map(str::trim) {
                    if val.len() < min_len {
                        continue;
                    }
                    if blacklist.iter().any(|b| name.eq_ignore_ascii_case(b)) {
                        continue;
                    }
                    out.push(Candidate {
                        def_type: def_type.clone(),
                        def_name: def_name.clone(),
                        field_path: name.to_string(),
                        value: val.to_string(),
                        source_file: p.to_path_buf(),
                        confidence: None,
                    });
                }
            }
        }
    }
    Ok(out)
}

// -------------
// Helpers
// -------------

fn normalize_handle(mut s: String) -> String {
    // Trim and replace spaces/tabs/newlines with underscore
    s = s.trim().to_string();
    let mut out = String::with_capacity(s.len());
    let mut last_underscore = false;
    for ch in s.chars() {
        let mapped = match ch {
            ' ' | '\t' | '\n' | '\r' => '_',
            '.' | '-' => '_',
            '{' | '}' => continue,
            _ => ch,
        };
        if mapped.is_ascii_alphanumeric() || mapped == '_' {
            let c = if mapped.is_ascii_uppercase() { mapped } else { mapped };
            if c == '_' {
                if !last_underscore {
                    out.push('_');
                    last_underscore = true;
                }
            } else {
                out.push(c);
                last_underscore = false;
            }
        }
    }
    out.trim_matches('_').to_string()
}

fn prefer_handle_segment(seg: &str) -> bool {
    seg.eq_ignore_ascii_case("li{h}") || seg.to_ascii_lowercase().starts_with("li{h")
}

fn strip_marker(seg: &str) -> &str {
    if let Some(pos) = seg.find('{') { &seg[..pos] } else { seg }
}

fn collect_entries_by_path_with_handles(
    node: roxmltree::Node,
    segs: &[&str],
    out: &mut Vec<(String, String)>,
) {
    fn walk(
        node: roxmltree::Node,
        segs: &[&str],
        acc: &mut Vec<String>,
        out: &mut Vec<(String, String)>,
    ) {
        if segs.is_empty() {
            if let Some(t) = node.text() {
                let val = t.trim();
                if !val.is_empty() {
                    let path = acc.join(".");
                    out.push((path, val.to_string()));
                }
            }
            return;
        }
        let raw_head = segs[0];
        let head = strip_marker(raw_head);
        let tail = &segs[1..];
        if head.eq_ignore_ascii_case("li") {
            // Iterate list items and append index or pseudo-handle token
            let prefer_handle = prefer_handle_segment(raw_head);
            let mut index: usize = 0;
            for child in node
                .children()
                .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
            {
                let mut token = index.to_string();
                if prefer_handle {
                    // Heuristic handle from Class attribute or well-known child fields
                    let mut handle = child.attribute("Class").map(|s| s.to_string());
                    if handle.is_none() {
                        for tag in [
                            "defName",
                            "label",
                            "name",
                            "compClass",
                            "thingDef",
                            "stat",
                            "skill",
                        ] {
                            if let Some(n) = child
                                .children()
                                .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(tag))
                            {
                                if let Some(txt) = n.text().map(str::trim) {
                                    if !txt.is_empty() {
                                        handle = Some(txt.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if let Some(h) = handle {
                        let h = if h.contains('.') {
                            h.split('.').last().unwrap_or("").to_string()
                        } else {
                            h
                        };
                        let norm = normalize_handle(h);
                        if !norm.is_empty() {
                            token = norm;
                        }
                    }
                }
                acc.push(token);
                walk(child, tail, acc, out);
                acc.pop();
                index += 1;
            }
        } else {
            for child in node
                .children()
                .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(head))
            {
                acc.push(head.to_string());
                walk(child, tail, acc, out);
                acc.pop();
            }
        }
    }

    // Start recursion at the first matching segment under current node
    if segs.is_empty() { return; }
    let raw_head = segs[0];
    let head = strip_marker(raw_head);
    let tail = &segs[1..];
    if head.eq_ignore_ascii_case("li") {
        // Promote list traversal at current level
        let mut acc: Vec<String> = Vec::new();
        let mut index: usize = 0;
        let prefer_handle = prefer_handle_segment(raw_head);
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
        {
            let mut token = index.to_string();
            if prefer_handle {
                let mut handle = child.attribute("Class").map(|s| s.to_string());
                if handle.is_none() {
                    for tag in ["defName", "label", "name", "compClass", "thingDef", "stat", "skill"] {
                        if let Some(n) = child.children().find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(tag)) {
                            if let Some(txt) = n.text().map(str::trim) { if !txt.is_empty() { handle = Some(txt.to_string()); break; } }
                        }
                    }
                }
                if let Some(h) = handle {
                    let h = if h.contains('.') { h.split('.').last().unwrap_or("").to_string() } else { h };
                    let norm = normalize_handle(h);
                    if !norm.is_empty() { token = norm; }
                }
            }
            acc.push(token);
            walk(child, tail, &mut acc, out);
            acc.pop();
            index += 1;
        }
    } else {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(head))
        {
            let mut acc: Vec<String> = vec![head.to_string()];
            walk(child, tail, &mut acc, out);
        }
    }
}

pub fn collect_existing_definj_keys(
    root: &Path,
    lang_dir: &str,
) -> Result<std::collections::BTreeSet<String>> {
    let mut set = std::collections::BTreeSet::new();
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
        if !(s.contains("/Languages/") || s.contains("\\Languages\\")) {
            continue;
        }
        if !(s.contains(&format!("/Languages/{lang_dir}/DefInjected/"))
            || s.contains(&format!("\\Languages\\{lang_dir}\\DefInjected\\")))
        {
            continue;
        }

        let content = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(doc) = Document::parse(&content) {
            let root_el = doc.root_element();
            // Direct children under LanguageData are elements whose tag names are keys like DefName.path
            for child in root_el.children().filter(|n| n.is_element()) {
                let name = child.tag_name().name().to_string();
                if !name.is_empty() {
                    set.insert(name);
                }
            }
        }
    }
    Ok(set)
}
