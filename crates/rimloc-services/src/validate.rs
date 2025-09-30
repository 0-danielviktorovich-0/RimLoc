use crate::{util::is_source_for_lang_dir, Result, ValidationMessage};
use std::path::Path;

/// Validate scanned units under a root with optional filtering by language folder/code.
pub fn validate_under_root(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = rimloc_parsers_xml::scan_all_units(scan_root)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

/// Same as `validate_under_root`, but allows restricting Defs scanning to a path.
pub fn validate_under_root_with_defs(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = rimloc_parsers_xml::scan_all_units_with_defs(scan_root, defs_root)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

pub fn validate_under_root_with_defs_and_fields(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> Result<Vec<ValidationMessage>> {
    let mut units = rimloc_parsers_xml::scan_all_units_with_defs_and_fields(
        scan_root,
        defs_root,
        extra_fields,
    )?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

pub fn validate_under_root_with_defs_and_dict(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<ValidationMessage>> {
    let mut units =
        crate::scan::scan_units_with_defs_and_dict(scan_root, defs_root, dict, extra_fields)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

/// Compare placeholders between source (English) and a target language by matching on keys.
/// This is stricter than the per-string `validate` and similar in spirit to `validate-po`.
/// Not wired to CLI by default; GUI or advanced flows can opt-in.
pub fn validate_placeholders_cross_language(
    scan_root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    defs_root: Option<&Path>,
) -> Result<Vec<ValidationMessage>> {
    fn extract_placeholders_like_cli(text: &str) -> std::collections::BTreeSet<String> {
        use regex::Regex;
        use std::sync::OnceLock;
        static RE_PCT: OnceLock<Regex> = OnceLock::new();
        static RE_BRACE: OnceLock<Regex> = OnceLock::new();
        let re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
        let re_brace = RE_BRACE.get_or_init(|| Regex::new(r"\{\s*([^{}\s]+)\s*\}").unwrap());
        let mut out: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for m in re_pct.find_iter(text) {
            out.insert(m.as_str().to_string());
        }
        for cap in re_brace.captures_iter(text) {
            if let Some(name) = cap.get(1) {
                out.insert(format!("{{{}}}", name.as_str()));
            }
        }
        out
    }
    // Scan everything, then filter per language
    let mut units = if let Some(defs) = defs_root {
        rimloc_parsers_xml::scan_all_units_with_defs(scan_root, Some(defs))?
    } else {
        rimloc_parsers_xml::scan_all_units(scan_root)?
    };

    // Split into source and target
    let mut src_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut tgt_map: std::collections::HashMap<String, (String, String, Option<usize>)> =
        std::collections::HashMap::new();

    for u in units.drain(..) {
        let path = u.path.clone();
        let key = u.key.clone();
        if is_source_for_lang_dir(&path, source_lang_dir) {
            if let Some(s) = u.source.as_deref() {
                if !s.trim().is_empty() {
                    src_map.entry(key).or_insert_with(|| s.to_string());
                }
            }
        } else if is_source_for_lang_dir(&path, target_lang_dir) {
            if let Some(s) = u.source.as_deref() {
                tgt_map.insert(key, (s.to_string(), path.to_string_lossy().into_owned(), u.line));
            }
        }
    }

    // Compare placeholder sets
    let mut msgs = Vec::new();
    for (key, (tgt, path, line)) in tgt_map.into_iter() {
        if let Some(src) = src_map.get(&key) {
            let src_ph = extract_placeholders_like_cli(src);
            let tgt_ph = extract_placeholders_like_cli(&tgt);
            if src_ph != tgt_ph {
                msgs.push(ValidationMessage {
                    kind: "placeholder-check".into(),
                    key,
                    path,
                    line,
                    message: "Placeholder mismatch vs source".into(),
                });
            }
        }
    }

    Ok(msgs)
}
