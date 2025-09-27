//! High-level orchestration layer over lower-level crates.
//! Thin public surface re-exporting operations from per-feature modules.

pub use rimloc_core::{Result, TransUnit};
pub use rimloc_export_po::PoStats as ExportPoStats;
pub use rimloc_validate::ValidationMessage;

mod util;
pub mod scan;
pub mod export;
pub mod validate;
pub mod import;
pub mod build;

pub use build::{build_from_po_dry_run, build_from_po_execute, build_from_root, BuildPlan};
pub use export::export_po_with_tm;
pub use import::{import_po_to_file, import_po_to_mod_tree, FileStat, ImportPlan, ImportSummary};
pub use scan::scan_units;
pub use validate::validate_under_root;
pub use util::is_under_languages_dir;
