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
    threshold: f32,
    classifier: &mut dyn super::ml::Classifier,
) -> Result<Vec<KeyedCandidate>> {
    let src = scan_keyed_source(root, source_lang_dir)?;
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
        writeln!(f, "  <{}></{}>", c.key, c.key)?;
    }
    writeln!(f, "</LanguageData>")?;
    Ok(())
}
