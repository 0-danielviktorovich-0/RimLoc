use crate::{scan::scan_units, util::is_under_languages_dir, Result};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiffOutput {
    pub changed: Vec<(String, String)>,
    pub only_in_translation: Vec<String>,
    pub only_in_mod: Vec<String>,
}

/// Compute presence/changed diffs between source and target language data.
pub fn diff_xml(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    baseline_po: Option<&Path>,
) -> Result<DiffOutput> {
    let units = scan_units(root)?;

    let mut src_map: HashMap<String, String> = HashMap::new();
    let mut trg_keys: BTreeSet<String> = BTreeSet::new();
    for u in &units {
        if is_under_languages_dir(&u.path, source_lang_dir) {
            if let Some(val) = u.source.as_deref() {
                src_map.entry(u.key.clone()).or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, target_lang_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    let mut only_in_src: Vec<String> = Vec::new();
    let mut only_in_trg: Vec<String> = Vec::new();
    for k in src_map.keys() {
        if !trg_keys.contains(k) { only_in_src.push(k.clone()); }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) { only_in_trg.push(k.clone()); }
    }
    only_in_src.sort();
    only_in_trg.sort();

    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po {
        // parse via CLI-compatible basic parser from rimloc-cli? Not available here.
        // Reuse rimloc-core minimal PO parser: it returns entries without context.
        // We need key from msgctxt; fallback: if core parser lacks context, skip changed.
        let s = std::fs::read_to_string(po)?;
        let entries = rimloc_core::parse_simple_po(&s)?;
        let mut base: HashMap<String, String> = HashMap::new();
        for e in entries {
            // We don't have ctx; use key as-is (rimloc-core::PoEntry.key is used as msgid key here)
            base.entry(e.key).or_insert(e.value);
        }
        for (k, new_src) in &src_map {
            if let Some(old_src) = base.get(k) {
                if old_src != new_src {
                    changed.push((k.clone(), new_src.clone()));
                }
            }
        }
        changed.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(DiffOutput { changed, only_in_translation: only_in_trg, only_in_mod: only_in_src })
}

pub fn write_diff_reports(dir: &Path, diff: &DiffOutput) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    // ChangedData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("ChangedData.txt"))?;
        for (k, v) in &diff.changed { writeln!(f, "{}\t{}", k, v)?; }
    }
    // TranslationData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("TranslationData.txt"))?;
        for k in &diff.only_in_translation { writeln!(f, "{}", k)?; }
    }
    // ModData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("ModData.txt"))?;
        for k in &diff.only_in_mod { writeln!(f, "{}", k)?; }
    }
    Ok(())
}

