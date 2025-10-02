use crate::Result;
use rimloc_core::TransUnit;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn read_keyed_file(path: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(content) = fs::read_to_string(path) else { return map };
    let mut reader = quick_xml::Reader::from_str(&content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut cur_key: Option<String> = None;
    let mut cur_val = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                stack.push(name.clone());
                if stack.len() == 2 {
                    cur_key = Some(name);
                    cur_val.clear();
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if stack.len() == 2 {
                    let text = t.unescape().unwrap_or_default().to_string();
                    cur_val.push_str(text.trim());
                }
            }
            Ok(quick_xml::events::Event::End(_)) => {
                if stack.len() == 2 {
                    if let Some(k) = cur_key.take() {
                        map.insert(k, cur_val.trim().to_string());
                        cur_val.clear();
                    }
                }
                stack.pop();
            }
            Ok(quick_xml::events::Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if stack.len() == 1 {
                    map.insert(name, String::new());
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) | _ => {}
        }
        buf.clear();
    }
    map
}

fn write_keyed_xml(out_path: &Path, entries: &[(String, String, String)], unused: &[(String, String)]) -> Result<()> {
    if let Some(parent) = out_path.parent() { fs::create_dir_all(parent)?; }
    let mut w = File::create(out_path)?;
    writeln!(w, "<?xml version=\"1.0\" encoding=\"utf-8\" ?>")?;
    writeln!(w, "<LanguageData>")?;
    for (key, en, tr) in entries {
        if !en.is_empty() {
            writeln!(w, "  <!-- EN: {} -->", en.replace("--", "—"))?;
        }
        writeln!(w, "  <{}>{}</{}>", key, quick_xml::escape::escape(tr.as_str()), key)?;
    }
    if !unused.is_empty() {
        writeln!(w, "  <!-- UNUSED -->")?;
        for (key, tr) in unused {
            writeln!(w, "  <{}>{}</{}>", key, quick_xml::escape::escape(tr.as_str()), key)?;
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
            if s.eq_ignore_ascii_case("Languages") { seen_lang = true; }
            continue;
        }
        if s != lang_dir { continue; }
        // Next should be Keyed
        if let Some(k) = comps.next() {
            if k.as_os_str().to_string_lossy().eq_ignore_ascii_case("Keyed") {
                let rest: PathBuf = comps.as_path().to_path_buf();
                return Some(rest);
            }
        }
        break;
    }
    None
}

pub struct MergeKeyedStats {
    pub files: usize,
    pub keys_total: usize,
    pub reused: usize,
    pub unused: usize,
}

pub fn merge_keyed(root: &Path, source_lang_dir: &str, target_lang_dir: &str, out_dir: Option<&Path>) -> Result<MergeKeyedStats> {
    let mut stats = MergeKeyedStats { files: 0, keys_total: 0, reused: 0, unused: 0 };
    // Gather all English Keyed files
    let mut en_files: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|e| e.to_str()).map_or(true, |ext| !ext.eq_ignore_ascii_case("xml")) { continue; }
        let s = p.to_string_lossy();
        if !(s.contains("/Languages/")||s.contains("\\Languages\\")) { continue; }
        if !(s.contains("/Keyed/")||s.contains("\\Keyed\\")) { continue; }
        if !(s.contains(&format!("/Languages/{}/", source_lang_dir)) || s.contains(&format!("\\Languages\\{}\\", source_lang_dir))) { continue; }
        en_files.push(p.to_path_buf());
    }
    en_files.sort();

    for en_path in en_files {
        let en_map = read_keyed_file(&en_path);
        if en_map.is_empty() { continue; }
        let rel = rel_keyed_path(&en_path, source_lang_dir).unwrap_or_else(|| PathBuf::from(en_path.file_name().unwrap()));
        let trg_rel = rel.clone();
        let trg_path = if let Some(out) = out_dir { out.join(trg_rel) } else { root.join("Languages").join(target_lang_dir).join("Keyed").join(trg_rel) };
        let old_trg_path = root.join("Languages").join(target_lang_dir).join("Keyed").join(rel);
        let trg_map = read_keyed_file(&old_trg_path);

        let mut entries: Vec<(String, String, String)> = Vec::new();
        let mut used: BTreeSet<String> = BTreeSet::new();
        for (k, en) in en_map.iter() {
            stats.keys_total += 1;
            let tr = trg_map.get(k).cloned().unwrap_or_default();
            if !tr.is_empty() { stats.reused += 1; }
            used.insert(k.clone());
            entries.push((k.clone(), en.clone(), tr));
        }
        // compute unused from target
        let mut unused: Vec<(String, String)> = Vec::new();
        for (k, v) in trg_map.iter() {
            if !used.contains(k) { unused.push((k.clone(), v.clone())); }
        }
        stats.unused += unused.len();

        write_keyed_xml(&trg_path, &entries, &unused)?;
        stats.files += 1;
    }

    Ok(stats)
}

