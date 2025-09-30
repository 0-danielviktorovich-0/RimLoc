use crate::Result;
use roxmltree::Document;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct KeyedCandidate {
    pub key: String,
    pub value: String,
    pub source_file: PathBuf,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct KeyedDict {
    pub include: Option<Vec<String>>, // regexes
    pub exclude: Option<Vec<String>>, // regexes
}

pub fn load_keyed_dict_from_file(path: &Path) -> Result<KeyedDict> {
    let s = std::fs::read_to_string(path)?;
    let d: KeyedDict = serde_json::from_str(&s)?;
    Ok(d)
}

/// Extract Keyed-like pairs from Defs for specific schemas used by popular frameworks.
/// - XmlExtensions.SettingsMenuDef: tKey/tKeyTip combined with label/text/tooltip
/// - QuestScriptDef: any element with attribute TKey → candidate(key=TKey, value=text)
pub fn scan_keyed_from_defs_special(root: &Path) -> Result<Vec<(String, String, PathBuf)>> {
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
        if !(s.contains("/Defs/") || s.contains("\\Defs\\")) {
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
        // XmlExtensions.SettingsMenuDef
        for def in root_el.children().filter(|n| n.is_element()) {
            let tag = def.tag_name().name();
            if tag.eq_ignore_ascii_case("XmlExtensions.SettingsMenuDef") {
                if let Some(settings) = def
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("settings"))
                {
                    for li in settings
                        .children()
                        .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
                    {
                        // Collect possible pairs
                        let mut tkey: Option<String> = None;
                        let mut tkey_text: Option<String> = None;
                        let mut tkey_tip: Option<String> = None;
                        let mut tkey_tip_text: Option<String> = None;
                        for child in li.children().filter(|c| c.is_element()) {
                            let nm = child.tag_name().name();
                            match nm {
                                "tKey" => tkey = child.text().map(|s| s.trim().to_string()),
                                "label" | "text" => {
                                    if tkey_text.is_none() {
                                        tkey_text = child.text().map(|s| s.trim().to_string())
                                    }
                                }
                                "tKeyTip" => tkey_tip = child.text().map(|s| s.trim().to_string()),
                                "tooltip" => {
                                    if tkey_tip_text.is_none() {
                                        tkey_tip_text = child.text().map(|s| s.trim().to_string())
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let (Some(k), Some(v)) = (tkey.as_ref(), tkey_text.as_ref()) {
                            if !k.is_empty() && !v.is_empty() {
                                out.push((k.clone(), v.clone(), p.to_path_buf()));
                            }
                        }
                        if let (Some(k), Some(v)) = (tkey_tip.as_ref(), tkey_tip_text.as_ref()) {
                            if !k.is_empty() && !v.is_empty() {
                                out.push((k.clone(), v.clone(), p.to_path_buf()));
                            }
                        }
                    }
                }
            }
            // QuestScriptDef: any descendant with attribute TKey
            if tag.eq_ignore_ascii_case("QuestScriptDef") {
                for node in def.descendants().filter(|n| n.is_element()) {
                    if let Some(k) = node.attribute("TKey") {
                        if let Some(v) = node.text().map(|s| s.trim()) {
                            if !k.trim().is_empty() && !v.is_empty() {
                                out.push((k.trim().to_string(), v.to_string(), p.to_path_buf()));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn scan_keyed_source(
    root: &Path,
    source_lang_dir: &str,
) -> Result<Vec<(String, String, PathBuf)>> {
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
        if !(s.contains("/Languages/") || s.contains("\\Languages\\")) {
            continue;
        }
        if !(s.contains(&format!("/Languages/{}/Keyed/", source_lang_dir))
            || s.contains(&format!("\\Languages\\{}\\Keyed\\", source_lang_dir)))
        {
            continue;
        }
        let content = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(doc) = Document::parse(&content) {
            let root_el = doc.root_element();
            for child in root_el.children().filter(|n| n.is_element()) {
                let name = child.tag_name().name().to_string();
                let val = child.text().map(str::trim).unwrap_or("").to_string();
                out.push((name, val, p.to_path_buf()));
            }
        }
    }
    Ok(out)
}

pub fn collect_existing_keyed(
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
        if !(s.contains(&format!("/Languages/{}/Keyed/", lang_dir))
            || s.contains(&format!("\\Languages\\{}\\Keyed\\", lang_dir)))
        {
            continue;
        }
        let content = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(doc) = Document::parse(&content) {
            for child in doc.root_element().children().filter(|n| n.is_element()) {
                let name = child.tag_name().name().to_string();
                if !name.is_empty() {
                    set.insert(name);
                }
            }
        }
    }
    Ok(set)
}

#[allow(clippy::too_many_arguments)]
pub fn learn_keyed(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    dicts: &[KeyedDict],
    min_len: usize,
    blacklist: &[String],
    must_contain_letter: bool,
    exclude_substr: &[String],
    threshold: f32,
    classifier: &mut dyn super::ml::Classifier,
) -> Result<Vec<KeyedCandidate>> {
    let mut src = scan_keyed_source(root, source_lang_dir)?;
    // Merge special Defs-derived keyed pairs
    if std::env::var("RIMLOC_LEARN_KEYED_FROM_DEFS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        src.extend(scan_keyed_from_defs_special(root)?);
    }
    let existing = collect_existing_keyed(root, target_lang_dir)?;
    // compile dict regexes
    let mut includes: Vec<regex::Regex> = Vec::new();
    let mut excludes: Vec<regex::Regex> = Vec::new();
    for d in dicts {
        if let Some(v) = &d.include {
            for pat in v {
                if let Ok(r) = regex::Regex::new(pat) {
                    includes.push(r);
                }
            }
        }
        if let Some(v) = &d.exclude {
            for pat in v {
                if let Ok(r) = regex::Regex::new(pat) {
                    excludes.push(r);
                }
            }
        }
    }
    let mut out = Vec::new();
    'outer: for (key, val, file) in src {
        if existing.contains(&key) {
            continue;
        }
        if val.trim().len() < min_len {
            continue;
        }
        if blacklist.iter().any(|b| key.eq_ignore_ascii_case(b)) {
            continue;
        }
        if !includes.is_empty() && !includes.iter().any(|r| r.is_match(&key)) {
            continue;
        }
        if excludes.iter().any(|r| r.is_match(&key)) {
            continue;
        }
        if must_contain_letter && !val.chars().any(|c| c.is_alphabetic()) {
            continue;
        }
        if exclude_substr.iter().any(|s| !s.is_empty() && key.contains(s)) {
            continue;
        }
        let mut cand = KeyedCandidate {
            key: key.clone(),
            value: val.clone(),
            source_file: file.clone(),
            confidence: None,
        };
        let score = classifier.score(&super::parser::Candidate {
            def_type: "Keyed".into(),
            def_name: key.clone(),
            field_path: String::new(),
            value: val.clone(),
            source_file: file.clone(),
            confidence: None,
        })?;
        if score < threshold {
            continue 'outer;
        }
        cand.confidence = Some(score);
        out.push(cand);
    }
    Ok(out)
}

pub fn write_keyed_missing_json(path: &Path, cands: &[KeyedCandidate]) -> Result<()> {
    #[derive(serde::Serialize)]
    #[allow(non_snake_case)]
    struct Item<'a> {
        key: &'a str,
        value: &'a str,
        confidence: f32,
        sourceFile: String,
    }
    let items: Vec<Item> = cands
        .iter()
        .map(|c| Item {
            key: &c.key,
            value: &c.value,
            confidence: c.confidence.unwrap_or(1.0),
            sourceFile: c.source_file.display().to_string(),
        })
        .collect();
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &items)?;
    Ok(())
}

pub fn write_keyed_suggested_xml(path: &Path, cands: &[KeyedCandidate]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "<LanguageData>")?;
    for c in cands {
        // Put original English value as a translator hint
        let en = super::export::escape_xml_comment(&c.value);
        writeln!(f, "  <!-- EN: {} -->", en)?;
        writeln!(f, "  <{}></{}>", c.key, c.key)?;
    }
    writeln!(f, "</LanguageData>")?;
    Ok(())
}
