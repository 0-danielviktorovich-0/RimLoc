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
pub mod extras;

pub use build::{build_from_po_dry_run, build_from_po_execute, build_from_root, BuildPlan};
pub use export::export_po_with_tm;
pub use import::{import_po_to_file, import_po_to_mod_tree, FileStat, ImportPlan, ImportSummary};
pub use scan::{scan_units, scan_units_with_defs, scan_units_with_defs_and_fields, scan_units_with_defs_and_dict};
pub use validate::{validate_under_root, validate_under_root_with_defs, validate_under_root_with_defs_and_fields, validate_under_root_with_defs_and_dict};
pub use extras::xml_health::xml_health_scan;
pub use extras::diff::{diff_xml, diff_xml_with_defs, diff_xml_with_defs_and_fields, write_diff_reports};
pub use rimloc_domain::{DiffOutput, HealthIssue, HealthReport};
pub use extras::init::{make_init_plan, write_init_plan, InitPlan, InitFilePlan};
pub use extras::annotate::{annotate as annotate_apply, annotate_dry_run_plan, AnnotateFilePlan, AnnotatePlan, AnnotateSummary};
pub use extras::morph::{generate as morph_generate, MorphOptions, MorphProvider, MorphResult};
pub use util::is_under_languages_dir;
