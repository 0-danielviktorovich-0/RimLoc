use crate::{util::def_injected_target_path, Result, TransUnit};
use rimloc_parsers_xml::DefsMetaUnit;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const DEFAULT_SOURCE_LANG_DIR: &str = "English";

fn seen_key(path: &Path, key: &str) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("{normalized}|{key}")
}

fn merge_defs_units(
    units: &mut Vec<TransUnit>,
    seen: &mut HashSet<String>,
    scan_root: &Path,
    lang_dir: &str,
    defs_meta: Vec<DefsMetaUnit>,
) {
    for meta in defs_meta {
        let mut unit = meta.unit;
        if unit
            .source
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            continue;
        }
        let target_path = def_injected_target_path(scan_root, lang_dir, &meta.def_type, &unit.path);
        unit.path = target_path;
        unit.line = None;
        let key = seen_key(&unit.path, &unit.key);
        if seen.insert(key) {
            units.push(unit);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutoDefsContext {
    pub dict: HashMap<String, Vec<String>>,
    pub extra_fields: Vec<String>,
    pub learned_sources: Vec<PathBuf>,
    pub dict_sources: Vec<PathBuf>,
}

fn merge_dict_sets(
    target: &mut HashMap<String, BTreeSet<String>>,
    source: &HashMap<String, Vec<String>>,
) {
    for (def_type, fields) in source {
        let entry = target.entry(def_type.clone()).or_default();
        for field in fields {
            entry.insert(field.clone());
        }
    }
}

fn flatten_dict_sets(map: HashMap<String, BTreeSet<String>>) -> HashMap<String, Vec<String>> {
    map.into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LearnedDefRow {
    #[serde(rename = "defType")]
    def_type: String,
    #[serde(rename = "fieldPath")]
    field_path: String,
}

fn load_learned_defs(path: &Path) -> Result<Vec<LearnedDefRow>> {
    let file = std::fs::File::open(path)?;
    let rows: Vec<LearnedDefRow> = serde_json::from_reader(file)?;
    Ok(rows)
}

fn autodiscover_learn_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    let mut push_dir = |p: PathBuf| {
        if p.is_dir() && seen.insert(p.clone()) {
            dirs.push(p);
        }
    };

    for name in ["_learn", "learn_out", "Learn", "learn"] {
        push_dir(root.join(name));
    }

    let languages_root = root.join("Languages");
    if languages_root.is_dir() {
        // Walk only shallow levels under Languages to catch nested _learn/learn_out folders.
        for entry in WalkDir::new(&languages_root)
            .min_depth(1)
            .max_depth(4)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if name.eq_ignore_ascii_case("_learn")
                || name.eq_ignore_ascii_case("learn_out")
                || name.eq_ignore_ascii_case("learn")
                || name.eq_ignore_ascii_case("Learn")
            {
                push_dir(entry.into_path());
            }
        }
    }

    dirs
}

fn load_dict_candidates(dirs: &[PathBuf]) -> (HashMap<String, Vec<String>>, Vec<PathBuf>) {
    let mut merged: HashMap<String, Vec<String>> = HashMap::new();
    let mut sources = Vec::new();
    for dir in dirs {
        if let Ok(read) = std::fs::read_dir(dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let is_json = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false);
                if !is_json {
                    continue;
                }
                let file_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if !(file_name.contains("defs_dict") || file_name.ends_with(".dict.json")) {
                    continue;
                }
                if let Ok(dict) = rimloc_parsers_xml::load_defs_dict_from_file(&path) {
                    for (def_type, fields) in dict.0 {
                        merged.entry(def_type).or_default().extend(fields);
                    }
                    sources.push(path);
                }
            }
        }
    }
    for v in merged.values_mut() {
        v.sort();
        v.dedup();
    }
    (merged, sources)
}

pub fn autodiscover_defs_context(root: &Path) -> Result<AutoDefsContext> {
    let mut dict_sets: HashMap<String, BTreeSet<String>> = HashMap::new();
    merge_dict_sets(
        &mut dict_sets,
        &rimloc_parsers_xml::load_embedded_defs_dict().0,
    );

    let learn_dirs = autodiscover_learn_dirs(root);
    let mut learned_sources = Vec::new();
    let mut extra_fields: BTreeSet<String> = BTreeSet::new();

    for dir in &learn_dirs {
        let candidate = dir.join("learned_defs.json");
        if candidate.is_file() {
            if let Ok(rows) = load_learned_defs(&candidate) {
                learned_sources.push(candidate.clone());
                for row in rows {
                    if row.field_path.contains('.') {
                        dict_sets
                            .entry(row.def_type.clone())
                            .or_default()
                            .insert(row.field_path.clone());
                    } else {
                        extra_fields.insert(row.field_path.clone());
                    }
                }
            }
        }
    }

    let (mut discovered_dicts, dict_sources) = load_dict_candidates(&learn_dirs);
    for (k, v) in discovered_dicts.drain() {
        dict_sets.entry(k).or_default().extend(v);
    }

    Ok(AutoDefsContext {
        dict: flatten_dict_sets(dict_sets),
        extra_fields: extra_fields.into_iter().collect(),
        learned_sources,
        dict_sources,
    })
}

pub fn scan_units_auto(root: &Path) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    let mut units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let mut seen: HashSet<String> = units.iter().map(|u| seen_key(&u.path, &u.key)).collect();
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, None, &auto.dict, &auto.extra_fields)?;
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
    );
    Ok(units)
}

/// Scan a RimWorld mod folder and return discovered translation units.
/// This wraps `rimloc_parsers_xml::scan_keyed_xml` to provide a stable entrypoint
/// for higher-level clients (CLI, GUI, LSP) without importing parser crates.
pub fn scan_units(root: &Path) -> Result<Vec<TransUnit>> {
    // Include both LanguageData (Keyed/DefInjected) and implicit English from Defs
    scan_units_auto(root)
}

/// Like `scan_units`, but restrict Defs scanning to a particular directory when provided.
pub fn scan_units_with_defs(
    root: &Path,
    defs_root: Option<&std::path::Path>,
) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    scan_units_with_defs_and_dict(root, defs_root, &auto.dict, &auto.extra_fields)
}

pub fn scan_units_with_defs_and_fields(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    let auto = autodiscover_defs_context(root)?;
    let mut merged_fields: Vec<String> = auto.extra_fields.clone();
    merged_fields.extend(extra_fields.iter().cloned());
    merged_fields.sort();
    merged_fields.dedup();
    scan_units_with_defs_and_dict(root, defs_root, &auto.dict, &merged_fields)
}

pub fn scan_units_with_defs_and_dict(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    let mut units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let mut seen: HashSet<String> = units.iter().map(|u| seen_key(&u.path, &u.key)).collect();
    let defs_meta =
        rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?;
    merge_defs_units(
        &mut units,
        &mut seen,
        root,
        DEFAULT_SOURCE_LANG_DIR,
        defs_meta,
    );
    Ok(units)
}

pub fn scan_defs_with_meta(
    root: &Path,
    defs_root: Option<&Path>,
    dict: &HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<DefsMetaUnit>> {
    rimloc_parsers_xml::scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn autodiscover_learn_dirs_finds_nested_under_languages() -> Result<()> {
        let dir = tempdir()?;
        let nested = dir
            .path()
            .join("Languages")
            .join("English")
            .join("DefInjected")
            .join("_learn");
        fs::create_dir_all(&nested)?;
        let learned = nested.join("learned_defs.json");
        fs::write(
            &learned,
            r#"[{"defType":"ThingDef","fieldPath":"description"}]"#,
        )?;

        let ctx = autodiscover_defs_context(dir.path())?;
        assert!(ctx
            .learned_sources
            .iter()
            .any(|p| p.file_name().and_then(|s| s.to_str()) == Some("learned_defs.json")));
        assert!(ctx
            .dict
            .get("ThingDef")
            .map(|fields| fields.iter().any(|f| f == "description"))
            .unwrap_or(false));
        Ok(())
    }
}
