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

#[allow(clippy::too_many_arguments)]
pub fn run_diff_xml(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    baseline_po: Option<std::path::PathBuf>,
    format: String,
    strict: bool,
    out_dir: Option<std::path::PathBuf>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "diff_xml_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, baseline_po = ?baseline_po, format = %format, out_dir = ?out_dir, game_version = ?game_version);

    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "diff_version_resolved", version = ver, path = %scan_root.display());
    }

    let units = rimloc_parsers_xml::scan_keyed_xml(&scan_root)?;

    // Resolve folders
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
    tracing::info!(event = "diff_lang_dirs", source = %src_dir, target = %trg_dir);

    use std::collections::{BTreeSet, HashMap};
    let mut src_map: HashMap<String, String> = HashMap::new();
    let mut trg_keys: BTreeSet<String> = BTreeSet::new();

    for u in &units {
        if is_under_languages_dir(&u.path, &src_dir) {
            if let Some(val) = u.source.as_deref() {
                // keep first occurrence to be stable; duplicates are not ideal but acceptable for summary
                src_map
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, &trg_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    // Presence diff
    let mut only_in_src: Vec<String> = Vec::new(); // present in mod source, missing in translation (ModData analogue)
    let mut only_in_trg: Vec<String> = Vec::new(); // present in translation, missing in mod source (TranslationData analogue)

    for k in src_map.keys() {
        if !trg_keys.contains(k) {
            only_in_src.push(k.clone());
        }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) {
            only_in_trg.push(k.clone());
        }
    }
    only_in_src.sort();
    only_in_trg.sort();

    // Changed data: requires baseline PO (msgid snapshot)
    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po.as_ref() {
        let entries = crate::po::parse_po_basic(po)?;
        let mut base: HashMap<String, String> = HashMap::new();
        for (ctx, msgid, _msgstr, _reference) in entries {
            if msgid.is_empty() {
                continue;
            }
            // ctx format: key|rel:path[:line] — take the key before '|'
            if let Some(c) = ctx {
                let key = c.split('|').next().unwrap_or("").trim();
                if !key.is_empty() {
                    base.entry(key.to_string()).or_insert(msgid);
                }
            }
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
    let any_diff = !changed.is_empty() || !only_in_trg.is_empty() || !only_in_src.is_empty();

    // Output
    if let Some(dir) = out_dir.as_ref() {
        use std::fs;
        use std::io::Write;
        fs::create_dir_all(dir)?;
        // ChangedData.txt
        if !changed.is_empty() {
            let mut f = std::fs::File::create(dir.join("ChangedData.txt"))?;
            for (k, v) in &changed {
                writeln!(f, "{}\t{}", k, v)?;
            }
        } else {
            std::fs::write(dir.join("ChangedData.txt"), "")?;
        }
        // TranslationData.txt (keys only in translation)
        {
            let mut f = std::fs::File::create(dir.join("TranslationData.txt"))?;
            for k in &only_in_trg {
                writeln!(f, "{}", k)?;
            }
        }
        // ModData.txt (keys only in mod source)
        {
            let mut f = std::fs::File::create(dir.join("ModData.txt"))?;
            for k in &only_in_src {
                writeln!(f, "{}", k)?;
            }
        }
        crate::ui_ok!("diffxml-saved", path = dir.display().to_string());
        if strict && any_diff {
            color_eyre::eyre::bail!("diffxml-nonempty");
        }
        return Ok(());
    }

    if format == "json" {
        #[derive(serde::Serialize)]
        struct DiffOut {
            changed: Vec<(String, String)>,
            only_in_translation: Vec<String>,
            only_in_mod: Vec<String>,
        }
        let out = DiffOut {
            changed: changed.clone(),
            only_in_translation: only_in_trg.clone(),
            only_in_mod: only_in_src.clone(),
        };
        serde_json::to_writer(std::io::stdout().lock(), &out)?;
        if strict && any_diff {
            color_eyre::eyre::bail!("diffxml-nonempty");
        }
        return Ok(());
    }

    // text summary
    crate::ui_out!(
        "diffxml-summary",
        changed = changed.len(),
        only_trg = only_in_trg.len(),
        only_src = only_in_src.len()
    );
    if strict && any_diff {
        color_eyre::eyre::bail!("diffxml-nonempty");
    }
    Ok(())
}
