use crate::{scan::scan_units, util::is_under_languages_dir, Result};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn rel_from_languages(path_str: &str) -> Option<String> {
    static REL_FROM_LANGUAGES: OnceCell<Regex> = OnceCell::new();
    let re = REL_FROM_LANGUAGES
        .get_or_init(|| Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap());
    re.captures(path_str)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

#[derive(Debug, Clone)]
pub struct InitFilePlan {
    pub path: PathBuf,
    pub keys: usize,
}

#[derive(Debug, Clone)]
pub struct InitPlan {
    pub files: Vec<InitFilePlan>,
    pub language: String,
}

pub fn make_init_plan(root: &Path, source_lang_dir: &str, target_lang_dir: &str) -> Result<InitPlan> {
    let units = scan_units(root)?;
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for u in &units {
        if is_under_languages_dir(&u.path, source_lang_dir) {
            let p = u.path.to_string_lossy().to_string();
            let rel = rel_from_languages(&p).unwrap_or_else(|| {
                u.path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Keys.xml")
                    .to_string()
            });
            grouped.entry(rel).or_default().insert(u.key.clone());
        }
    }
    let mut files = Vec::new();
    for (rel, keys) in grouped {
        let out = root.join("Languages").join(target_lang_dir).join(rel);
        files.push(InitFilePlan { path: out, keys: keys.len() });
    }
    Ok(InitPlan { files, language: target_lang_dir.to_string() })
}

pub fn write_init_plan(plan: &InitPlan, overwrite: bool, dry_run: bool) -> Result<usize> {
    let mut files_written = 0usize;
    for f in &plan.files {
        if f.path.exists() && !overwrite {
            continue;
        }
        if dry_run {
            continue;
        }
        // Create empty entries with given count cannot be reconstructed; instead, skip count
        // The caller should regenerate exact key list if needed; here we only write if needed.
        // For correctness, we leave writing to CLI that has full key lists; however, we can write empty file.
        rimloc_import_po::write_language_data_xml(&f.path, &Vec::<(String, String)>::new())?;
        files_written += 1;
    }
    Ok(files_written)
}

