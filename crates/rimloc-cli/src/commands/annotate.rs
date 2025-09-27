use crate::version::resolve_game_version_root;

fn is_under_languages_dir(path: &std::path::Path, lang_dir: &str) -> bool {
    let mut comps = path.components();
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

fn sanitize_comment(s: &str) -> String {
    // XML comments cannot contain "--" sequences; replace them safely.
    s.replace("--", "—")
}

#[allow(clippy::too_many_arguments)]
pub fn run_annotate(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    comment_prefix: Option<String>,
    dry_run: bool,
    backup: bool,
    strip: bool,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "annotate_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, dry_run = dry_run, backup = backup, strip = strip, game_version = ?game_version);

    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "annotate_version_resolved", version = ver, path = %scan_root.display());
    }

    // Build source map: key -> original text
    let units = rimloc_parsers_xml::scan_keyed_xml(&scan_root)?;
    let src_dir = if let Some(dir) = source_lang_dir {
        dir
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "English".to_string()
    };
    let trg_dir = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };

    use std::collections::HashMap;
    let mut src_map: HashMap<String, String> = HashMap::new();
    for u in &units {
        if is_under_languages_dir(&u.path, &src_dir) {
            if let Some(val) = &u.source {
                src_map.entry(u.key.clone()).or_insert_with(|| val.clone());
            }
        }
    }

    // Collect target files
    use walkdir::WalkDir;
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for entry in WalkDir::new(&scan_root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        if !is_under_languages_dir(p, &trg_dir) {
            continue;
        }
        // Restrict to Keyed for now
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\")) {
            continue;
        }
        files.push(p.to_path_buf());
    }
    files.sort();

    let mut processed = 0usize;
    let mut annotated = 0usize;

    let prefix = comment_prefix.unwrap_or_else(|| "EN:".to_string());
    for path in files {
        processed += 1;
        let input = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Parse and rewrite with comments or strip comments
        use quick_xml::events::{BytesText, Event};
        use quick_xml::Reader;
        use quick_xml::Writer;

        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut out = Writer::new_with_indent(Vec::new(), b' ', 2);

        // Track current top-level key within LanguageData
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

                    // If adding comments: when entering a direct child of LanguageData (a key)
                    if in_language_data && stack.len() == 2 && !strip {
                        if let Some(orig) = src_map.get(&name) {
                            let comment = format!(" {} {} ", prefix, sanitize_comment(orig));
                            out.write_event(Event::Comment(BytesText::new(&comment)))?;
                            annotated += 1;
                        }
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
                    // Empty key under LanguageData
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if in_language_data && stack.len() == 1 && !strip {
                        if let Some(orig) = src_map.get(&name) {
                            let comment = format!(" {} {} ", prefix, sanitize_comment(orig));
                            out.write_event(Event::Comment(BytesText::new(&comment)))?;
                            annotated += 1;
                        }
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
                Ok(Event::Comment(_c)) => {
                    // Strip existing comments if requested; else preserve
                    if !strip {
                        out.write_event(Event::Comment(_c.to_owned()))?;
                    }
                }
                Ok(Event::DocType(d)) => {
                    out.write_event(Event::DocType(d))?;
                }
                Ok(Event::Eof) => break,
                Err(_e) => break,
            }
            buf.clear();
        }

        if dry_run {
            crate::ui_out!("annotate-would-write", path = path.display().to_string());
            continue;
        }

        if backup && path.exists() {
            let bak = path.with_extension("xml.bak");
            let _ = std::fs::copy(&path, &bak);
        }

        std::fs::write(&path, out.into_inner())?;
        crate::ui_out!("xml-saved", path = path.display().to_string());
    }

    crate::ui_out!(
        "annotate-summary",
        processed = processed,
        annotated = annotated
    );
    Ok(())
}
