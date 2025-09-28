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
                    let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
                    let mut vals = Vec::new();
                    super::ml::collect_values_by_path(def_node, &segs, &mut vals);
                    for v in vals {
                        let v = v.trim();
                        if v.len() < min_len {
                            continue;
                        }
                        if blacklist.iter().any(|b| path.eq_ignore_ascii_case(b)) {
                            continue;
                        }
                        out.push(Candidate {
                            def_type: def_type.clone(),
                            def_name: def_name.clone(),
                            field_path: path.clone(),
                            value: v.to_string(),
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
