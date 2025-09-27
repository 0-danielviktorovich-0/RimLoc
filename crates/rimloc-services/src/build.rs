use crate::{Result};
use std::path::{Path, PathBuf};

/// Build translation mod from an existing Languages/<lang> tree under `from_root`.
/// Returns a list of files to write with number of keys; optionally writes when `write=true`.
pub fn build_from_root(
    from_root: &Path,
    out_mod: &Path,
    lang_folder: &str,
    versions: Option<&[String]>,
    write: bool,
    dedupe: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
    use std::collections::{BTreeMap, HashSet};

    let mut grouped: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap();
    let mut total_keys = 0usize;
    let units = rimloc_parsers_xml::scan_keyed_xml(from_root)?;
    for u in units {
        let path_str = u.path.to_string_lossy();
        if !(path_str.contains("/Languages/") || path_str.contains("\\Languages\\")) {
            continue;
        }
        if !(path_str.contains(&format!("/Languages/{}/", lang_folder))
            || path_str.contains(&format!("\\Languages\\{}\\", lang_folder)))
        {
            continue;
        }
        if let Some(vers) = versions {
            let mut matched = false;
            for ver in vers {
                if path_str.contains(&format!("/{}/", ver))
                    || path_str.contains(&format!("\\{}\\", ver))
                    || path_str.contains(&format!("/v{}/", ver))
                    || path_str.contains(&format!("\\v{}\\", ver))
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                continue;
            }
        }
        if let Some(src) = u.source.as_deref() {
            if let Some(caps) = re.captures(&path_str) {
                let rel = PathBuf::from(&caps[1]);
                grouped
                    .entry(rel)
                    .or_default()
                    .push((u.key, src.to_string()));
                total_keys += 1;
            }
        }
    }

    let mut files: Vec<(PathBuf, usize)> = Vec::new();
    for (rel, mut items) in grouped.into_iter() {
        if dedupe {
            let mut seen: HashSet<String> = HashSet::new();
            let mut outv: Vec<(String, String)> = Vec::new();
            for (k, v) in items.into_iter().rev() {
                if seen.insert(k.clone()) {
                    outv.push((k, v));
                }
            }
            outv.reverse();
            items = outv;
        }

        let full = out_mod.join("Languages").join(lang_folder).join(&rel);
        if write {
            rimloc_import_po::write_language_data_xml(&full, &items)?;
        }
        files.push((full, items.len()));
    }

    if write {
        // Ensure About/ exists but leave content generation to caller
        let _ = std::fs::create_dir_all(out_mod.join("About"));
    }

    Ok((files, total_keys))
}

/// Build a translation mod from a PO file (wrappers around importer crate).
pub struct BuildPlan {
    pub mod_name: String,
    pub package_id: String,
    pub rw_version: String,
    pub out_mod: PathBuf,
    pub lang_dir: String,
    pub files: Vec<(PathBuf, usize)>,
    pub total_keys: usize,
}

pub fn build_from_po_dry_run(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<BuildPlan> {
    let plan = rimloc_import_po::build_translation_mod_dry_run_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
    )?;
    Ok(BuildPlan {
        mod_name: plan.mod_name,
        package_id: plan.package_id,
        rw_version: plan.rw_version,
        out_mod: plan.out_mod,
        lang_dir: plan.lang_dir,
        files: plan.files,
        total_keys: plan.total_keys,
    })
}

pub fn build_from_po_execute(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<()> {
    rimloc_import_po::build_translation_mod_with_langdir_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
    )
}

