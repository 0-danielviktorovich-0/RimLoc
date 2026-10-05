//! High-level orchestration layer over lower-level crates.
//! Thin public surface re-exporting operations from per-feature modules.

pub use rimloc_core::{Result, TransUnit};
pub use rimloc_export_po::PoStats as ExportPoStats;
pub use rimloc_validate::ValidationMessage;

pub mod build;
pub mod canonical_bridge;
pub mod contract;
pub mod contribution;
pub mod eligibility_engine;
pub mod export;
pub mod extras;
pub mod import;
pub mod keyed_merge;
pub mod learn;
pub mod matching;
pub mod modview;
pub mod observability;
pub mod patches_effect;
pub mod plugins;
pub mod plugins_json;
pub mod plugins_msf;
pub mod plugins_xml_ext;
pub mod plugins_yaml;
pub mod project;
pub mod project_store;
pub mod providers;
pub mod scan;
pub mod session;
pub mod ui_catalog;
mod util;
pub mod validate;

pub use build::{
    build_from_po_dry_run, build_from_po_execute, build_from_po_with_progress, build_from_root,
    build_from_root_with_progress, BuildPlan,
};
pub use contract::{
    capability_report, ui_contract_version, ApplyExistingRequest, ApplyExistingResponse,
    ApplyIntentsRequest, ApplyIntentsResponse, Capability, CapabilityReport, ContractError,
    ContractErrorCode, CreateProjectRequest, ExistingAmbiguousItem, ExistingMatchItem,
    ImportExistingRequest, ImportExistingResponse, IntentAction, JobId, PathBufDto, ProjectId,
    ProjectSnapshot, ProjectSummary, Revision, SessionEpoch, TranslationIntent,
    UnsupportedCapability, EXISTING_LIST_LIMIT, UI_CONTRACT_VERSION,
};
pub use eligibility_engine::{builtin_seed_rules, load_rule_pack, EligibilityEngine};
pub use export::export_po_with_tm;
pub use extras::annotate::{
    annotate as annotate_apply, annotate_dry_run_plan, AnnotateFilePlan, AnnotatePlan,
    AnnotateSummary,
};
pub use extras::diff::{
    apply_diff_flags, diff_xml, diff_xml_with_defs, diff_xml_with_defs_and_dict,
    diff_xml_with_defs_and_fields, write_diff_reports,
};
pub use extras::init::{make_init_plan, write_init_plan, InitFilePlan, InitPlan};
pub use extras::lang_update::{lang_update, LangUpdatePlan, LangUpdateSummary};
pub use extras::morph::{generate as morph_generate, MorphOptions, MorphProvider, MorphResult};
pub use extras::xml_health::xml_health_scan;
pub use import::{
    import_po_to_file, import_po_to_mod_tree, import_po_to_mod_tree_with_progress, FileStat,
    ImportPlan, ImportSummary,
};
pub use matching::{MatchOrigin, Resolution, SourceMatcher, TKeyRegistry};
pub use modview::{effective_view, EffectiveModView};
pub use observability::{
    collect_support_bundle_for, generate_operation_id, sha256_hex, BundleFile, ClassifyReport,
    ErrorRecord, FieldDecision, OperationLog, ProjectMeta, Sanitizer, StageRecord, SupportBundle,
    SupportBundleInputs, REDACTION_MARKER,
};
pub use rimloc_domain::{DiffOutput, HealthIssue, HealthReport};
pub use scan::{
    autodiscover_defs_context, scan_defs_with_meta, scan_patches_as_units, scan_units,
    scan_units_auto, scan_units_effective, scan_units_with_defs, scan_units_with_defs_and_dict,
    scan_units_with_defs_and_fields, AutoDefsContext,
};
pub use session::ProjectSessionManager;
pub use util::canonical_match_key;
pub use util::canonical_view;pub use util::ensure_free_output_path;
pub use util::ensure_writable_output_path;
pub use util::is_source_for_lang_dir;
pub use util::is_under_languages_dir;
pub use util::is_within;
pub use util::lang_dir_form_ok;
pub use util::normalize_lang_dir;
pub use util::package_id_slug;
pub use util::resolve_cli_out_path;
pub use util::write_atomic;
pub use util::{PathGuardError, PathGuardErrorKind};
pub use validate::validate_placeholders_cross_language;
pub use validate::{validate_lists_cross_language, validate_orphans_cross_language};
pub use validate::{
    validate_under_root, validate_under_root_with_defs, validate_under_root_with_defs_and_dict,
    validate_under_root_with_defs_and_fields,
};
