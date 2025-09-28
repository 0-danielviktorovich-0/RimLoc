use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RimLocConfig {
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub game_version: Option<String>,
    pub list_limit: Option<usize>,
    pub export: Option<ExportCfg>,
    pub import: Option<ImportCfg>,
    pub build: Option<BuildCfg>,
    pub diff: Option<DiffCfg>,
    pub health: Option<HealthCfg>,
    pub annotate: Option<AnnotateCfg>,
    pub init: Option<InitCfg>,
    pub schema: Option<SchemaCfg>,
    pub scan: Option<ScanCfg>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExportCfg {
    pub source_lang_dir: Option<String>,
    pub include_all_versions: Option<bool>,
    pub tm_root: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ImportCfg {
    pub keep_empty: Option<bool>,
    pub backup: Option<bool>,
    pub single_file: Option<bool>,
    pub incremental: Option<bool>,
    pub only_diff: Option<bool>,
    pub report: Option<bool>,
    pub lang_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BuildCfg {
    pub name: Option<String>,
    pub package_id: Option<String>,
    pub rw_version: Option<String>,
    pub lang_dir: Option<String>,
    pub dedupe: Option<bool>,
    pub from_root_versions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DiffCfg {
    pub out_dir: Option<String>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HealthCfg {
    pub lang_dir: Option<String>,
    pub strict: Option<bool>,
    pub only: Option<Vec<String>>,
    pub except: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnnotateCfg {
    pub comment_prefix: Option<String>,
    pub strip: Option<bool>,
    pub backup: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InitCfg {
    pub overwrite: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SchemaCfg {
    pub out_dir: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("{0}")]
    Other(String),
}

pub fn load_config() -> Result<RimLocConfig, ConfigError> {
    // Search order: CWD/rimloc.toml, $HOME/.config/rimloc/rimloc.toml
    let mut merged = RimLocConfig::default();
    if let Ok(p) = std::env::current_dir() {
        let path = p.join("rimloc.toml");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<RimLocConfig>(&s) {
                merged = merge(merged, cfg);
            }
        }
    }
    if let Some(base) = dirs::config_dir() {
        let path = base.join("rimloc").join("rimloc.toml");
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = toml::from_str::<RimLocConfig>(&s) {
                merged = merge(merged, cfg);
            }
        }
    }
    Ok(merged)
}

fn merge(mut a: RimLocConfig, b: RimLocConfig) -> RimLocConfig {
    if a.source_lang.is_none() {
        a.source_lang = b.source_lang;
    }
    if a.target_lang.is_none() {
        a.target_lang = b.target_lang;
    }
    if a.game_version.is_none() {
        a.game_version = b.game_version;
    }
    if a.list_limit.is_none() {
        a.list_limit = b.list_limit;
    }
    a.export = merge_opt(a.export, b.export, merge_export);
    a.import = merge_opt(a.import, b.import, merge_import);
    a.build = merge_opt(a.build, b.build, merge_build);
    a.diff = merge_opt(a.diff, b.diff, merge_diff);
    a.health = merge_opt(a.health, b.health, merge_health);
    a.annotate = merge_opt(a.annotate, b.annotate, merge_annotate);
    a.init = merge_opt(a.init, b.init, merge_init);
    a.schema = merge_opt(a.schema, b.schema, merge_schema);
    a.scan = merge_opt(a.scan, b.scan, merge_scan);
    a
}

fn merge_opt<T: Default>(a: Option<T>, b: Option<T>, f: fn(T, T) -> T) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, None) => None,
    }
}

fn merge_export(mut a: ExportCfg, b: ExportCfg) -> ExportCfg {
    if a.source_lang_dir.is_none() {
        a.source_lang_dir = b.source_lang_dir;
    }
    if a.include_all_versions.is_none() {
        a.include_all_versions = b.include_all_versions;
    }
    if a.tm_root.is_none() {
        a.tm_root = b.tm_root;
    }
    a
}
fn merge_import(mut a: ImportCfg, b: ImportCfg) -> ImportCfg {
    if a.keep_empty.is_none() {
        a.keep_empty = b.keep_empty;
    }
    if a.backup.is_none() {
        a.backup = b.backup;
    }
    if a.single_file.is_none() {
        a.single_file = b.single_file;
    }
    if a.incremental.is_none() {
        a.incremental = b.incremental;
    }
    if a.only_diff.is_none() {
        a.only_diff = b.only_diff;
    }
    if a.report.is_none() {
        a.report = b.report;
    }
    if a.lang_dir.is_none() {
        a.lang_dir = b.lang_dir;
    }
    a
}
fn merge_build(mut a: BuildCfg, b: BuildCfg) -> BuildCfg {
    if a.name.is_none() {
        a.name = b.name;
    }
    if a.package_id.is_none() {
        a.package_id = b.package_id;
    }
    if a.rw_version.is_none() {
        a.rw_version = b.rw_version;
    }
    if a.lang_dir.is_none() {
        a.lang_dir = b.lang_dir;
    }
    if a.dedupe.is_none() {
        a.dedupe = b.dedupe;
    }
    if a.from_root_versions.is_none() {
        a.from_root_versions = b.from_root_versions;
    }
    a
}
fn merge_diff(mut a: DiffCfg, b: DiffCfg) -> DiffCfg {
    if a.out_dir.is_none() {
        a.out_dir = b.out_dir;
    }
    if a.strict.is_none() {
        a.strict = b.strict;
    }
    a
}
fn merge_health(mut a: HealthCfg, b: HealthCfg) -> HealthCfg {
    if a.lang_dir.is_none() {
        a.lang_dir = b.lang_dir;
    }
    if a.strict.is_none() {
        a.strict = b.strict;
    }
    if a.only.is_none() {
        a.only = b.only;
    }
    if a.except.is_none() {
        a.except = b.except;
    }
    a
}
fn merge_annotate(mut a: AnnotateCfg, b: AnnotateCfg) -> AnnotateCfg {
    if a.comment_prefix.is_none() {
        a.comment_prefix = b.comment_prefix;
    }
    if a.strip.is_none() {
        a.strip = b.strip;
    }
    if a.backup.is_none() {
        a.backup = b.backup;
    }
    a
}
fn merge_init(mut a: InitCfg, b: InitCfg) -> InitCfg {
    if a.overwrite.is_none() {
        a.overwrite = b.overwrite;
    }
    a
}
fn merge_schema(mut a: SchemaCfg, b: SchemaCfg) -> SchemaCfg {
    if a.out_dir.is_none() {
        a.out_dir = b.out_dir;
    }
    a
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ScanCfg {
    pub defs_fields: Option<Vec<String>>, // extra Defs fields to scan
    pub defs_dicts: Option<Vec<String>>,  // user dictionaries paths
}

fn merge_scan(mut a: ScanCfg, b: ScanCfg) -> ScanCfg {
    if a.defs_fields.is_none() {
        a.defs_fields = b.defs_fields;
    }
    if a.defs_dicts.is_none() {
        a.defs_dicts = b.defs_dicts;
    }
    a
}
