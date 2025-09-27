use crate::{util::is_under_languages_dir, Result, ValidationMessage};
use std::path::Path;

/// Validate scanned units under a root with optional filtering by language folder/code.
pub fn validate_under_root(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = rimloc_parsers_xml::scan_keyed_xml(scan_root)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_under_languages_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_under_languages_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

