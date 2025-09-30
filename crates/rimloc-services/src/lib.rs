//! High-level orchestration layer over lower-level crates.
//! Thin public surface re-exporting operations from per-feature modules.

pub use rimloc_core::{Result, TransUnit};
pub use rimloc_export_po::PoStats as ExportPoStats;
pub use rimloc_validate::ValidationMessage;

pub mod build;
pub mod export;
pub mod extras;
pub mod import;
pub mod learn;
pub mod scan;
mod util;
pub mod validate;

pub use build::{
    build_from_po_dry_run, build_from_po_execute, build_from_po_with_progress, build_from_root,
    build_from_root_with_progress, BuildPlan,
};
pub use export::export_po_with_tm;
pub use extras::annotate::{
    annotate as annotate_apply, annotate_dry_run_plan, AnnotateFilePlan, AnnotatePlan,
    AnnotateSummary,
};
pub use extras::diff::{
    diff_xml, diff_xml_with_defs, diff_xml_with_defs_and_dict, diff_xml_with_defs_and_fields,
    write_diff_reports, apply_diff_flags,
};
pub use extras::init::{make_init_plan, write_init_plan, InitFilePlan, InitPlan};
pub use extras::lang_update::{lang_update, LangUpdatePlan, LangUpdateSummary};
pub use extras::morph::{generate as morph_generate, MorphOptions, MorphProvider, MorphResult};
pub use extras::xml_health::xml_health_scan;
pub use import::{
    import_po_to_file, import_po_to_mod_tree, import_po_to_mod_tree_with_progress, FileStat,
    ImportPlan, ImportSummary,
};
pub use rimloc_domain::{DiffOutput, HealthIssue, HealthReport};
pub use scan::{
    autodiscover_defs_context, scan_defs_with_meta, scan_units, scan_units_auto,
    scan_units_with_defs, scan_units_with_defs_and_dict, scan_units_with_defs_and_fields,
    AutoDefsContext,
};
pub use util::is_under_languages_dir;
pub use validate::{
    validate_under_root, validate_under_root_with_defs, validate_under_root_with_defs_and_dict,
    validate_under_root_with_defs_and_fields,
};
