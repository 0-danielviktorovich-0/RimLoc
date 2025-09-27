//! High-level orchestration layer over lower-level crates.
//! Intentionally thin: exposes stable functions used by CLI/GUI/LSP.

use std::path::Path;

pub use rimloc_core::{Result, TransUnit};

/// Scan a RimWorld mod folder and return discovered translation units.
/// This wraps `rimloc_parsers_xml::scan_keyed_xml` to provide a stable entrypoint
/// for higher-level clients (CLI, GUI, LSP) without importing parser crates.
pub fn scan_units(root: &Path) -> Result<Vec<TransUnit>> {
    rimloc_parsers_xml::scan_keyed_xml(root)
}

