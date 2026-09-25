use crate::Result;
#[allow(unused_imports)]
use rimloc_core::TransUnit;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn read_keyed_file(path: &Path) -> BTreeMap<String, String> {
    rimloc_parsers_xml::read_keyed_file_map(path).unwrap_or_default()
}

fn write_keyed_xml(
    out_path: &Path,
    entries: &[(String, String, String)],
    unused: &[(String, String)],
) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut w = File::create(out_path)?;
    writeln!(w, "<?xml version=\"1.0\" encoding=\"utf-8\" ?>")?;
    writeln!(w, "<LanguageData>")?;
    for (key, en, tr) in entries {
        if !en.is_empty() {
            writeln!(w, "  <!-- EN: {} -->", en.replace("--", "—"))?;
        }
        writeln!(
            w,
            "  <{}>{}</{}>",
            key,
            quick_xml::escape::escape(tr.as_str()),
            key
        )?;
    }
    if !unused.is_empty() {
        writeln!(w, "  <!-- UNUSED -->")?;
        for (key, tr) in unused {
            writeln!(
                w,
                "  <{}>{}</{}>",
                key,
                quick_xml::escape::escape(tr.as_str()),
                key
            )?;
        }
    }
    writeln!(w, "</LanguageData>")?;
    Ok(())
}

fn rel_keyed_path(path: &Path, lang_dir: &str) -> Option<PathBuf> {
    // Extract subpath under Languages/<lang_dir>/Keyed
    let mut comps = path.components();
    let mut seen_lang = false;
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if !seen_lang {
            if s.eq_ignore_ascii_case("Languages") {
                seen_lang = true;
            }
            continue;
        }
        if s != lang_dir {
            continue;
        }
        // Next should be Keyed
        if let Some(k) = comps.next() {
            if k.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case("Keyed")
            {
                let rest: PathBuf = comps.as_path().to_path_buf();
                return Some(rest);
            }
        }
        break;
    }
    None
}

#[derive(Debug)]
pub struct MergeKeyedStats {
    pub files: usize,
    pub keys_total: usize,
    pub reused: usize,
    pub unused: usize,
}

pub fn merge_keyed(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    out_dir: Option<&Path>,
) -> Result<MergeKeyedStats> {
    // H1 follow-up: the target folder is joined into read AND write paths
    // under the root — strict form + containment before anything runs.
    crate::util::ensure_lang_write_target(root, target_lang_dir)?;
    let mut stats = MergeKeyedStats {
        files: 0,
        keys_total: 0,
        reused: 0,
        unused: 0,
    };
    // Gather all English Keyed files
    let mut en_files: Vec<PathBuf> = Vec::new();
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
        if !(s.contains("/Keyed/") || s.contains("\\Keyed\\")) {
            continue;
        }
        if !(s.contains(&format!("/Languages/{}/", source_lang_dir))
            || s.contains(&format!("\\Languages\\{}\\", source_lang_dir)))
        {
            continue;
        }
        en_files.push(p.to_path_buf());
    }
    en_files.sort();

    for en_path in en_files {
        let en_map = read_keyed_file(&en_path);
        if en_map.is_empty() {
            continue;
        }
        let rel = rel_keyed_path(&en_path, source_lang_dir)
            .unwrap_or_else(|| PathBuf::from(en_path.file_name().unwrap()));
        let trg_rel = rel.clone();
        let trg_path = if let Some(out) = out_dir {
            out.join(trg_rel)
        } else {
            root.join("Languages")
                .join(target_lang_dir)
                .join("Keyed")
                .join(trg_rel)
        };
        let old_trg_path = root
            .join("Languages")
            .join(target_lang_dir)
            .join("Keyed")
            .join(rel);
        let trg_map = read_keyed_file(&old_trg_path);

        let mut entries: Vec<(String, String, String)> = Vec::new();
        let mut used: BTreeSet<String> = BTreeSet::new();
        for (k, en) in en_map.iter() {
            stats.keys_total += 1;
            let tr = trg_map.get(k).cloned().unwrap_or_default();
            if !tr.is_empty() {
                stats.reused += 1;
            }
            used.insert(k.clone());
            entries.push((k.clone(), en.clone(), tr));
        }
        // compute unused from target
        let mut unused: Vec<(String, String)> = Vec::new();
        for (k, v) in trg_map.iter() {
            if !used.contains(k) {
                unused.push((k.clone(), v.clone()));
            }
        }
        stats.unused += unused.len();

        write_keyed_xml(&trg_path, &entries, &unused)?;
        stats.files += 1;
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// H1 follow-up: merge-keyed writes under Languages/<target> — a
    /// traversal-shaped target folder is a typed refusal with nothing
    /// written; a plain folder name keeps the merge working.
    #[test]
    fn merge_keyed_refuses_traversal_target_lang_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("Languages/English/Keyed/T.xml"),
            "<LanguageData><K>hello</K></LanguageData>",
        );

        let err = merge_keyed(root, "English", "../../evil", None).unwrap_err();
        assert!(
            err.to_string().contains("malformed language folder"),
            "{err}"
        );
        assert!(!tmp.path().join("evil").exists());

        // Happy path still works with a plain folder name.
        let stats = merge_keyed(root, "English", "Russian", None).unwrap();
        assert_eq!(stats.files, 1);
        assert!(root.join("Languages/Russian/Keyed/T.xml").exists());
    }
}
