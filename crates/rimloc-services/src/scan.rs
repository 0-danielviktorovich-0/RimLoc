use crate::{Result, TransUnit};
use std::path::Path;

/// Scan a RimWorld mod folder and return discovered translation units.
/// This wraps `rimloc_parsers_xml::scan_keyed_xml` to provide a stable entrypoint
/// for higher-level clients (CLI, GUI, LSP) without importing parser crates.
pub fn scan_units(root: &Path) -> Result<Vec<TransUnit>> {
    // Include both LanguageData (Keyed/DefInjected) and implicit English from Defs
    Ok(rimloc_parsers_xml::scan_all_units(root)?)
}

/// Like `scan_units`, but restrict Defs scanning to a particular directory when provided.
pub fn scan_units_with_defs(root: &Path, defs_root: Option<&std::path::Path>) -> Result<Vec<TransUnit>> {
    Ok(rimloc_parsers_xml::scan_all_units_with_defs(root, defs_root)?)
}

pub fn scan_units_with_defs_and_fields(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    Ok(rimloc_parsers_xml::scan_all_units_with_defs_and_fields(root, defs_root, extra_fields)?)
}

pub fn scan_units_with_defs_and_dict(
    root: &Path,
    defs_root: Option<&std::path::Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<Vec<TransUnit>> {
    let units = rimloc_parsers_xml::scan_keyed_xml(root)?;
    let defs = rimloc_parsers_xml::scan_defs_with_dict(root, defs_root, dict, extra_fields)?;
    Ok({ let mut u = units; u.extend(defs); u })
}
