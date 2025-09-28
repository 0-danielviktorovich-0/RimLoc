use crate::{scan::scan_units, util::is_under_languages_dir, Result};
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer};
use rimloc_domain::{AnnotateFilePlan as DFilePlan, AnnotatePlan as DPlan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct AnnotateSummary {
    pub processed: usize,
    pub annotated: usize,
}

fn sanitize_comment(s: &str) -> String {
    s.replace("--", "—")
}

pub fn annotate(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    comment_prefix: &str,
    strip: bool,
    dry_run: bool,
    backup: bool,
) -> Result<AnnotateSummary> {
    // Build source map: key -> original text
    let units = scan_units(root)?;
    let mut src_map: HashMap<String, String> = HashMap::new();
    for u in &units {
        if is_under_languages_dir(&u.path, source_lang_dir) {
            if let Some(val) = &u.source {
                src_map.entry(u.key.clone()).or_insert_with(|| val.clone());
            }
        }
    }

    // Collect target files (Keyed only)
    let mut files: Vec<PathBuf> = Vec::new();
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
        if !is_under_languages_dir(p, target_lang_dir) {
            continue;
        }
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\")) {
            continue;
        }
        files.push(p.to_path_buf());
    }
    files.sort();

    let mut processed = 0usize;
    let mut annotated = 0usize;

    for path in files {
        processed += 1;
        let input = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut out = Writer::new_with_indent(Vec::new(), b' ', 2);
        let mut stack: Vec<String> = Vec::new();
        let mut in_language_data = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if name == "LanguageData" {
                        in_language_data = true;
                    }
                    stack.push(name.clone());
                    if in_language_data && stack.len() == 2 && !strip && src_map.contains_key(&name)
                    {
                        let comment = format!(
                            " {} {} ",
                            comment_prefix,
                            sanitize_comment(src_map.get(&name).unwrap())
                        );
                        out.write_event(Event::Comment(BytesText::new(&comment)))?;
                        annotated += 1;
                    }
                    out.write_event(Event::Start(e.to_owned()))?;
                }
                Ok(Event::End(e)) => {
                    let name = stack.pop();
                    if name.as_deref() == Some("LanguageData") {
                        in_language_data = false;
                    }
                    out.write_event(Event::End(e.to_owned()))?;
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if in_language_data && stack.len() == 1 && !strip && src_map.contains_key(&name)
                    {
                        let comment = format!(
                            " {} {} ",
                            comment_prefix,
                            sanitize_comment(src_map.get(&name).unwrap())
                        );
                        out.write_event(Event::Comment(BytesText::new(&comment)))?;
                        annotated += 1;
                    }
                    out.write_event(Event::Empty(e.to_owned()))?;
                }
                Ok(Event::Text(t)) => {
                    out.write_event(Event::Text(t))?;
                }
                Ok(Event::CData(t)) => {
                    out.write_event(Event::CData(t))?;
                }
                Ok(Event::Decl(d)) => {
                    out.write_event(Event::Decl(d))?;
                }
                Ok(Event::PI(p)) => {
                    out.write_event(Event::PI(p))?;
                }
                Ok(Event::Comment(c)) => {
                    if !strip {
                        out.write_event(Event::Comment(c.to_owned()))?;
                    }
                }
                Ok(Event::DocType(d)) => {
                    out.write_event(Event::DocType(d))?;
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
            }
            buf.clear();
        }

        if dry_run {
            continue;
        }
        if backup && path.exists() {
            let _ = std::fs::copy(&path, path.with_extension("xml.bak"));
        }
        std::fs::write(&path, out.into_inner())?;
    }

    Ok(AnnotateSummary {
        processed,
        annotated,
    })
}

pub type AnnotateFilePlan = DFilePlan;
pub type AnnotatePlan = DPlan;

/// Build a dry-run plan for annotate without modifying files.
pub fn annotate_dry_run_plan(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    _comment_prefix: &str,
    strip: bool,
) -> Result<AnnotatePlan> {
    // Build source map: key -> original text
    let units = scan_units(root)?;
    let mut src_map: HashMap<String, String> = HashMap::new();
    for u in &units {
        if is_under_languages_dir(&u.path, source_lang_dir) {
            if let Some(val) = &u.source {
                src_map.entry(u.key.clone()).or_insert_with(|| val.clone());
            }
        }
    }

    let mut files: Vec<PathBuf> = Vec::new();
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
        if !is_under_languages_dir(p, target_lang_dir) {
            continue;
        }
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\")) {
            continue;
        }
        files.push(p.to_path_buf());
    }
    files.sort();

    let mut out_files: Vec<AnnotateFilePlan> = Vec::new();
    let mut total_add = 0usize;
    let mut total_strip = 0usize;
    let mut processed = 0usize;

    for path in files {
        processed += 1;
        let input = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut stack: Vec<String> = Vec::new();
        let mut in_language_data = false;
        let mut add_cnt = 0usize;
        let mut strip_cnt = 0usize;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if name == "LanguageData" {
                        in_language_data = true;
                    }
                    stack.push(name.clone());
                    if in_language_data && stack.len() == 2 && !strip && src_map.contains_key(&name)
                    {
                        add_cnt += 1;
                    }
                }
                Ok(Event::End(_)) => {
                    let name = stack.pop();
                    if name.as_deref() == Some("LanguageData") {
                        in_language_data = false;
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if in_language_data && stack.len() == 1 && !strip && src_map.contains_key(&name)
                    {
                        add_cnt += 1;
                    }
                }
                Ok(Event::Comment(_c)) => {
                    if strip {
                        strip_cnt += 1;
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(_) => break,
            }
            buf.clear();
        }
        out_files.push(AnnotateFilePlan {
            path: path.display().to_string(),
            add: add_cnt,
            strip: strip_cnt,
        });
        total_add += add_cnt;
        total_strip += strip_cnt;
    }

    Ok(AnnotatePlan {
        files: out_files,
        total_add,
        total_strip,
        processed,
    })
}
