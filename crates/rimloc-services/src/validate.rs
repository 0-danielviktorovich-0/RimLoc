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
    let mut units = rimloc_parsers_xml::scan_all_units_with_defs_and_fields(scan_root, defs_root, extra_fields)?;
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
    let mut units = crate::scan::scan_units_with_defs_and_dict(scan_root, defs_root, dict, extra_fields)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| crate::util::is_source_for_lang_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}
