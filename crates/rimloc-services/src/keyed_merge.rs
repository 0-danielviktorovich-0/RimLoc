use crate::Result;
#[allow(unused_imports)]
use rimloc_core::TransUnit;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn read_keyed_file(path: &Path) -> BTreeMap<String, String> {
    // Robust reader that supports list items (<li>) and <LineBreak/> semantics
    // under <LanguageData> and preserves multi-line entries.
    let mut map = BTreeMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };
    let mut reader = quick_xml::Reader::from_str(&content);
    // Keep whitespace inside nodes as much as possible; we will trim when storing
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    #[derive(Default)]
    struct Frame {
        name: String,
        buffer: String,
        has_text: bool,
    }

    let mut stack: Vec<Frame> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                stack.push(Frame {
                    name,
                    buffer: String::new(),
                    has_text: false,
                });
            }
            Ok(quick_xml::events::Event::End(_e)) => {
                if let Some(frame) = stack.pop() {
                    // Top-level key under LanguageData => commit
                    if stack.len() == 1 && !frame.name.is_empty() {
                        let val = if frame.has_text {
                            frame.buffer.trim().to_string()
                        } else {
                            String::new()
                        };
                        map.insert(frame.name, val);
                        continue;
                    }
                    // If closing <li> under a top-level key, fold into parent with newline handling
                    if frame.name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                        if let Some(parent) = stack.last_mut() {
                            if parent.has_text && !parent.buffer.ends_with('\n') {
                                parent.buffer.push('\n');
                            }
                            if !frame.buffer.is_empty() {
                                parent.buffer.push_str(frame.buffer.trim());
                                parent.has_text = true;
                            } else {
                                // Empty <li/> still counts as an empty line
                                parent.has_text = true;
                            }
                        }
                        continue;
                    }
                    // Otherwise bubble text up if needed
                    if let Some(parent) = stack.last_mut() {
                        if !frame.buffer.is_empty() {
                            parent.buffer.push_str(&frame.buffer);
                            parent.has_text = parent.has_text || frame.has_text;
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                // Self-closing LineBreak inside key or list item => newline
                if name.eq_ignore_ascii_case("LineBreak") {
                    if let Some(parent) = stack.last_mut() {
                        parent.buffer.push('\n');
                        parent.has_text = true;
                    }
                    buf.clear();
                    continue;
                }
                // Self-closing key directly under LanguageData => empty value
                if stack.len() == 1 {
                    map.insert(name, String::new());
                    buf.clear();
                    continue;
                }
                // Self-closing <li/> under a top-level key contributes an empty line
                if name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                    if let Some(parent) = stack.last_mut() {
                        if parent.has_text && !parent.buffer.ends_with('\n') {
                            parent.buffer.push('\n');
                        }
                        parent.has_text = true;
                    }
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if let Some(frame) = stack.last_mut() {
                    let text = t.unescape().unwrap_or_default().to_string();
                    if !text.trim().is_empty() {
                        frame.buffer.push_str(text.trim());
                        frame.has_text = true;
                    }
                }
            }
            Ok(quick_xml::events::Event::CData(t)) => {
                if let Some(frame) = stack.last_mut() {
                    let text = String::from_utf8_lossy(t.as_ref());
                    if !text.trim().is_empty() {
                        frame.buffer.push_str(text.trim());
                        frame.has_text = true;
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        buf.clear();
    }
    map
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
