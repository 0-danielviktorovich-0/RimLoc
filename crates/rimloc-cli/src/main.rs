//

// use rimloc_validate::validate; // moved into commands
include!(concat!(env!("OUT_DIR"), "/supported_locales.rs"));
use crate::placeholders::extract_placeholders;
use crate::po::parse_po_basic;
use clap::{Command as ClapCommand, Parser, Subcommand};
use color_eyre::eyre::Result;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageRequester;
use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;
use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use unic_langid::LanguageIdentifier;

#[derive(RustEmbed)]
#[folder = "i18n"]
#[include = "**/*.ftl"]
struct Localizations;

static LANG_LOADER: OnceCell<FluentLanguageLoader> = OnceCell::new();

// UI helpers and localization macros
#[macro_use]
mod ui;

mod commands;
mod placeholders;
mod po;
mod version;

fn init_i18n() {
    // создаём загрузчик Fluent
    let loader = FluentLanguageLoader::new("rimloc", "en".parse().expect("valid fallback lang"));

    // выбираем языки, запрошенные системой/пользователем
    let req = DesktopLanguageRequester::new();
    let requested: Vec<LanguageIdentifier> = req.requested_languages();

    // Оставляем только поддерживаемые (по языковому коду), + гарантируем fallback `en`
    let mut to_load: Vec<LanguageIdentifier> = requested
        .into_iter()
        .filter(|id| SUPPORTED_LOCALES.contains(&id.language.as_str()))
        .collect();
    let fallback: LanguageIdentifier = "en".parse().expect("valid fallback lang");
    if !to_load.iter().any(|i| i == &fallback) {
        to_load.push(fallback);
    }

    // грузим в loader ресурсы из вшитых ассетов (см. #[derive(RustEmbed)] выше)
    i18n_embed::select(&loader, &Localizations, &to_load).expect("failed to initialize i18n");

    // сохраняем глобально
    let _ = LANG_LOADER.set(loader);
}

fn set_ui_lang(lang: Option<&str>) {
    if let Some(code) = lang {
        if let Ok(id) = code.parse::<LanguageIdentifier>() {
            // проверяем, поддерживаем ли такой базовый язык
            if SUPPORTED_LOCALES.contains(&id.language.as_str()) {
                let langs = [id];
                if let Some(loader) = LANG_LOADER.get() {
                    let _ = i18n_embed::select(loader, &Localizations, &langs);
                }
            } else {
                // игнорируем неподдерживаемый код, оставляя текущую локаль
                ui_warn!("ui-lang-unsupported");
            }
        }
    }
}

/// Pre-scan CLI args to obtain --ui-lang (before we build localized clap Command)
fn pre_scan_ui_lang() -> Option<String> {
    let mut prev_is_flag = false;
    let mut found: Option<String> = None;
    for arg in std::env::args_os().skip(1) {
        if prev_is_flag {
            found = Some(arg.to_string_lossy().into_owned());
            break;
        }
        if let Some(s) = arg.to_str() {
            if s == "--ui-lang" {
                prev_is_flag = true;
                continue;
            }
            if let Some(rest) = s.strip_prefix("--ui-lang=") {
                found = Some(rest.to_string());
                break;
            }
        }
    }
    found
}

/// Pre-scan CLI args to see if --quiet is present before we build localized clap Command
fn pre_scan_quiet() -> bool {
    for arg in std::env::args_os().skip(1) {
        if let Some(s) = arg.to_str() {
            if s == "--quiet" {
                return true;
            }
        }
    }
    false
}

/// Apply localized texts (about/help) to the clap Command using tr!()
fn localize_command(mut cmd: ClapCommand) -> ClapCommand {
    // Top-level about
    // Expect FTL key: help-about
    cmd = cmd.about(tr!("help-about"));

    // Top-level args: --no-color, --ui-lang
    cmd = cmd.mut_arg("no_color", |a| a.help(tr!("help-no-color")));
    cmd = cmd.mut_arg("ui_lang", |a| a.help(tr!("help-ui-lang")));
    cmd = cmd.mut_arg("quiet", |a| a.help(tr!("help-quiet")));

    // Subcommands
    for sc in cmd.get_subcommands_mut() {
        match sc.get_name() {
            "scan" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-scan-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-scan-root")));
                owned = owned.mut_arg("out_csv", |a| a.help(tr!("help-scan-out-csv")));
                owned = owned.mut_arg("out_json", |a| a.help(tr!("help-scan-out-json")));
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-scan-lang")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-scan-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-scan-source-lang-dir"))
                });
                owned = owned.mut_arg("format", |a| a.help(tr!("help-scan-format")));
                owned = owned.mut_arg("game_version", |a| a.help(tr!("help-scan-game-version")));
                owned = owned.mut_arg("include_all_versions", |a| {
                    a.help(tr!("help-scan-include-all"))
                });
                *sc = owned;
            }
            "validate" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-validate-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-validate-root")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-validate-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-validate-source-lang-dir"))
                });
                owned = owned.mut_arg("format", |a| a.help(tr!("help-validate-format")));
                owned = owned.mut_arg("game_version", |a| {
                    a.help(tr!("help-validate-game-version"))
                });
                owned = owned.mut_arg("include_all_versions", |a| {
                    a.help(tr!("help-validate-include-all"))
                });
                *sc = owned;
            }
            "validate-po" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-validatepo-about"));
                owned = owned.mut_arg("po", |a| a.help(tr!("help-validatepo-po")));
                owned = owned.mut_arg("strict", |a| a.help(tr!("help-validatepo-strict")));
                owned = owned.mut_arg("format", |a| a.help(tr!("help-validatepo-format")));
                *sc = owned;
            }
            "diff-xml" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-diffxml-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-diffxml-root")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-diffxml-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-diffxml-source-lang-dir"))
                });
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-diffxml-lang")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-diffxml-lang-dir")));
                owned = owned.mut_arg("baseline_po", |a| a.help(tr!("help-diffxml-baseline-po")));
                owned = owned.mut_arg("format", |a| a.help(tr!("help-diffxml-format")));
                owned = owned.mut_arg("out_dir", |a| a.help(tr!("help-diffxml-out-dir")));
                owned = owned.mut_arg("game_version", |a| a.help(tr!("help-diffxml-game-version")));
                owned = owned.mut_arg("strict", |a| a.help(tr!("help-diffxml-strict")));
                *sc = owned;
            }
            "export-po" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-exportpo-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-exportpo-root")));
                owned = owned.mut_arg("out_po", |a| a.help(tr!("help-exportpo-out-po")));
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-exportpo-lang")));
                owned = owned.mut_arg("pot", |a| a.help(tr!("help-exportpo-pot")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-exportpo-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-exportpo-source-lang-dir"))
                });
                owned = owned.mut_arg("tm_root", |a| a.help(tr!("help-exportpo-tm-root")));
                owned = owned.mut_arg("game_version", |a| {
                    a.help(tr!("help-exportpo-game-version"))
                });
                owned = owned.mut_arg("include_all_versions", |a| {
                    a.help(tr!("help-exportpo-include-all"))
                });
                *sc = owned;
            }
            "import-po" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-importpo-about"));
                owned = owned.mut_arg("po", |a| a.help(tr!("help-importpo-po")));
                owned = owned.mut_arg("out_xml", |a| a.help(tr!("help-importpo-out-xml")));
                owned = owned.mut_arg("mod_root", |a| a.help(tr!("help-importpo-mod-root")));
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-importpo-lang")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-importpo-lang-dir")));
                owned = owned.mut_arg("keep_empty", |a| a.help(tr!("help-importpo-keep-empty")));
                owned = owned.mut_arg("dry_run", |a| a.help(tr!("help-importpo-dry-run")));
                owned = owned.mut_arg("backup", |a| a.help(tr!("help-importpo-backup")));
                owned = owned.mut_arg("single_file", |a| a.help(tr!("help-importpo-single-file")));
                owned = owned.mut_arg("game_version", |a| {
                    a.help(tr!("help-importpo-game-version"))
                });
                *sc = owned;
            }
            "build-mod" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-buildmod-about"));
                owned = owned.mut_arg("po", |a| a.help(tr!("help-buildmod-po")));
                owned = owned.mut_arg("out_mod", |a| a.help(tr!("help-buildmod-out-mod")));
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-buildmod-lang")));
                owned = owned.mut_arg("name", |a| a.help(tr!("help-buildmod-name")));
                owned = owned.mut_arg("package_id", |a| a.help(tr!("help-buildmod-package-id")));
                owned = owned.mut_arg("rw_version", |a| a.help(tr!("help-buildmod-rw-version")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-buildmod-lang-dir")));
                owned = owned.mut_arg("dry_run", |a| a.help(tr!("help-buildmod-dry-run")));
                owned = owned.mut_arg("dedupe", |a| a.help(tr!("help-buildmod-dedupe")));
                *sc = owned;
            }
            "annotate" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-annotate-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-annotate-root")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-annotate-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-annotate-source-lang-dir"))
                });
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-annotate-lang")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-annotate-lang-dir")));
                owned = owned.mut_arg("dry_run", |a| a.help(tr!("help-annotate-dry-run")));
                owned = owned.mut_arg("backup", |a| a.help(tr!("help-annotate-backup")));
                owned = owned.mut_arg("strip", |a| a.help(tr!("help-annotate-strip")));
                owned = owned.mut_arg("game_version", |a| {
                    a.help(tr!("help-annotate-game-version"))
                });
                *sc = owned;
            }
            "xml-health" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-xmlhealth-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-xmlhealth-root")));
                owned = owned.mut_arg("format", |a| a.help(tr!("help-xmlhealth-format")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-xmlhealth-lang-dir")));
                *sc = owned;
            }
            "init" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-init-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-init-root")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-init-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-init-source-lang-dir"))
                });
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-init-lang")));
                owned = owned.mut_arg("lang_dir", |a| a.help(tr!("help-init-lang-dir")));
                owned = owned.mut_arg("overwrite", |a| a.help(tr!("help-init-overwrite")));
                owned = owned.mut_arg("dry_run", |a| a.help(tr!("help-init-dry-run")));
                owned = owned.mut_arg("game_version", |a| a.help(tr!("help-init-game-version")));
                *sc = owned;
            }
            _ => {}
        }
    }

    cmd
}

static LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();
const DEFAULT_LOGDIR: &str = "logs";
// Public schema version for structured outputs (JSON) produced by the CLI.
pub(crate) const OUTPUT_SCHEMA_VERSION: u32 = rimloc_core::RIMLOC_SCHEMA_VERSION;

fn resolve_log_dir() -> std::path::PathBuf {
    // Prefer RIMLOC_LOG_DIR (underscore). This matches init_tracing.
    if let Ok(val) = std::env::var("RIMLOC_LOG_DIR") {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return std::path::PathBuf::from(trimmed);
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(DEFAULT_LOGDIR)
}

#[derive(Parser)]
#[command(name = "rimloc", version)]
struct Cli {
    /// Disable colored output (help text is localized via FTL at runtime).
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    cmd: Commands,

    /// UI language override (e.g., "ru" or "en"); help text localized via FTL.
    #[arg(long, global = true)]
    ui_lang: Option<String>,

    /// Suppress startup banner and non-essential stdout messages.
    #[arg(long, global = true, visible_alias = "no-banner")]
    quiet: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan a mod folder and extract Keyed XML (help localized via FTL).
    Scan {
        #[arg(short, long)]
        root: PathBuf,
        #[arg(long)]
        out_csv: Option<PathBuf>,
        #[arg(long, conflicts_with = "out_csv")]
        out_json: Option<PathBuf>,
        #[arg(long)]
        lang: Option<String>,
        /// Source language by ISO code (e.g., en, ru, ja).
        #[arg(long)]
        source_lang: Option<String>,
        /// Source language folder name (e.g., "English"). Takes precedence.
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Output format: "csv" (default), or "json".
        #[arg(long, default_value = "csv", value_parser = ["csv", "json"])]
        format: String,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
        /// Include all version subfolders (disable auto-pick of latest).
        #[arg(long, default_value_t = false)]
        include_all_versions: bool,
    },

    /// Validate strings and report issues (help localized via FTL).
    Validate {
        #[arg(short, long)]
        root: PathBuf,
        /// Source language by ISO code.
        #[arg(long)]
        source_lang: Option<String>,
        /// Source language folder name (e.g., "English").
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Output format: "text" (default) or "json".
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
        /// Include all version subfolders (disable auto-pick of latest).
        #[arg(long, default_value_t = false)]
        include_all_versions: bool,
    },

    /// Validate .po placeholder consistency (msgid vs msgstr); help via FTL.
    ValidatePo {
        /// Path to .po file.
        #[arg(long)]
        po: PathBuf,
        /// Strict mode: return non-zero exit if mismatches are found.
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Output format for results: "text" (default) or "json".
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
    },

    /// Diff source vs translation presence, optionally against a baseline PO to detect changed source strings.
    DiffXml {
        /// Path to mod root to analyze.
        #[arg(short, long)]
        root: PathBuf,
        /// Source language ISO code (e.g., en); maps to RimWorld folder name.
        #[arg(long)]
        source_lang: Option<String>,
        /// Source language folder name (e.g., "English").
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Target translation language ISO code (e.g., ru); maps to RimWorld folder.
        #[arg(long)]
        lang: Option<String>,
        /// Target translation language folder name (e.g., "Russian").
        #[arg(long)]
        lang_dir: Option<String>,
        /// Optional baseline PO to detect changed source strings since last export.
        #[arg(long)]
        baseline_po: Option<PathBuf>,
        /// Output format: "text" (default) or "json".
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
        /// Optional directory to write Text files (ChangedData.txt, TranslationData.txt, ModData.txt)
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
        /// Strict mode: return non-zero exit if any difference is found
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Annotate translation XML with source text comments (or strip them)
    Annotate {
        /// Path to mod root.
        #[arg(short, long)]
        root: PathBuf,
        /// Source language ISO code (maps to folder name).
        #[arg(long)]
        source_lang: Option<String>,
        /// Source language folder name (e.g., "English").
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Target translation ISO code.
        #[arg(long)]
        lang: Option<String>,
        /// Target translation folder name (e.g., "Russian").
        #[arg(long)]
        lang_dir: Option<String>,
        /// Comment prefix to use before the original text (default: "EN:")
        #[arg(long)]
        comment_prefix: Option<String>,
        /// Do not write files; only print planned changes.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Create .bak before overwriting XML files.
        #[arg(long, default_value_t = false)]
        backup: bool,
        /// Remove existing comments instead of adding.
        #[arg(long, default_value_t = false)]
        strip: bool,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
    },

    /// Scan XML for structural issues under Languages/
    XmlHealth {
        /// Path to RimWorld mod root.
        #[arg(short, long)]
        root: PathBuf,
        /// Output format: "text" (default) or "json".
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
        /// Restrict scan to specific language folder name.
        #[arg(long)]
        lang_dir: Option<String>,
        /// Strict mode: return non-zero exit if issues are found
        #[arg(long, default_value_t = false)]
        strict: bool,
    },

    /// Generate Case/Plural/Gender files using a morph provider
    Morph {
        /// Path to mod root.
        #[arg(short, long)]
        root: PathBuf,
        /// Provider name: dummy (default) or morpher
        #[arg(long)]
        provider: Option<String>,
        /// Target translation language ISO code
        #[arg(long)]
        lang: Option<String>,
        /// Target translation folder name
        #[arg(long)]
        lang_dir: Option<String>,
        /// Regex to filter Keyed keys (default: ".*")
        #[arg(long)]
        filter_key_regex: Option<String>,
        /// Limit number of keys to process
        #[arg(long)]
        limit: Option<usize>,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
    },

    /// Initialize translation skeleton under Languages/<target>
    Init {
        /// Path to mod root.
        #[arg(short, long)]
        root: PathBuf,
        /// Source language ISO code.
        #[arg(long)]
        source_lang: Option<String>,
        /// Source language folder name (e.g., "English").
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Target translation ISO code.
        #[arg(long)]
        lang: Option<String>,
        /// Target translation folder name.
        #[arg(long)]
        lang_dir: Option<String>,
        /// Overwrite existing files if present.
        #[arg(long, default_value_t = false)]
        overwrite: bool,
        /// Dry-run: do not write files, print plan only.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
    },

    /// Export extracted strings to a single .po file (help localized via FTL).
    ExportPo {
        /// Path to mod root (or Languages/<locale>).
        #[arg(short, long)]
        root: PathBuf,

        /// Output .po path.
        #[arg(long)]
        out_po: PathBuf,

        /// Target translation language for PO header, e.g., ru, ja, de.
        #[arg(long)]
        lang: Option<String>,
        /// Write a POT template (empty Language header) instead of a localized PO.
        #[arg(long, default_value_t = false)]
        pot: bool,

        /// Source language by ISO code (mapped via rimworld_lang_dir).
        #[arg(long)]
        source_lang: Option<String>,

        /// Source language folder name (e.g., "English"). Takes precedence over --source-lang.
        #[arg(long)]
        source_lang_dir: Option<String>,
        /// Optional path to a TM root (e.g., Languages/Russian or a mod root) to prefill msgstr and mark fuzzy.
        #[arg(long)]
        tm_root: Option<PathBuf>,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
        /// Include all version subfolders (disable auto-pick of latest).
        #[arg(long, default_value_t = false)]
        include_all_versions: bool,
    },

    /// Import .po into a single XML or into an existing mod's structure (help via FTL).
    ImportPo {
        #[arg(long)]
        po: PathBuf,
        #[arg(long, conflicts_with = "mod_root")]
        out_xml: Option<PathBuf>,
        #[arg(long, conflicts_with = "out_xml")]
        mod_root: Option<PathBuf>,
        #[arg(long)]
        lang: Option<String>,
        #[arg(long)]
        lang_dir: Option<String>,
        #[arg(long, default_value_t = false)]
        keep_empty: bool,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        backup: bool,
        #[arg(long, default_value_t = false)]
        single_file: bool,
        /// Game version folder to operate on (e.g., 1.6 or v1.6).
        #[arg(long)]
        game_version: Option<String>,
        /// Output format for reports or dry-run: "text" (default) or "json".
        #[arg(long, default_value = "text", value_parser = ["text", "json"])]
        format: String,
        /// Print a summary of created/updated/skipped files and total keys written.
        #[arg(long, default_value_t = false)]
        report: bool,
        /// Skip writing files whose content would be identical.
        #[arg(long, default_value_t = false)]
        incremental: bool,
    },

    /// Build a standalone translation mod from a .po file (help via FTL).
    BuildMod {
        #[arg(long)]
        po: PathBuf,
        #[arg(long)]
        out_mod: PathBuf,
        #[arg(long)]
        lang: String,
        /// Optional: build from existing Languages/<lang> under this root instead of a .po
        #[arg(long)]
        from_root: Option<PathBuf>,
        /// Optional filter for --from-root: only include files under this game version subfolder
        #[arg(long)]
        from_game_version: Option<String>,
        #[arg(long, default_value = "RimLoc Translation")]
        name: String,
        #[arg(long, default_value = "yourname.rimloc.translation")]
        package_id: String,
        #[arg(long, default_value = "1.5")]
        rw_version: String,
        #[arg(long)]
        lang_dir: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Remove duplicate keys within a single XML file (last wins)
        #[arg(long, default_value_t = false)]
        dedupe: bool,
    },
}

#[allow(dead_code)]
fn is_under_languages_dir(path: &std::path::Path, lang_dir: &str) -> bool {
    let mut comps = path.components();

    // Ищем компонент "Languages" (без учёта регистра)
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            // следующий компонент должен быть <lang_dir> (чувствительно к регистру как в FS)
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

// placeholder and po helpers moved to modules

trait Runnable {
    fn run(self, use_color: bool) -> Result<()>;
}

impl Runnable for Commands {
    fn run(self, use_color: bool) -> Result<()> {
        let cmd_name = format!("{:?}", self);
        info!(event = "cmd_start", cmd = %cmd_name);
        let span = tracing::info_span!("cmd", name = %cmd_name);
        let _enter = span.enter();

        let result = match self {
            Commands::Scan {
                root,
                out_csv,
                out_json,
                lang,
                source_lang,
                source_lang_dir,
                format,
                game_version,
                include_all_versions,
            } => commands::scan::run_scan(
                root,
                out_csv,
                out_json,
                lang,
                source_lang,
                source_lang_dir,
                format,
                game_version,
                include_all_versions,
            ),

            Commands::Validate {
                root,
                source_lang,
                source_lang_dir,
                format,
                game_version,
                include_all_versions,
            } => commands::validate::run_validate(
                root,
                source_lang,
                source_lang_dir,
                format,
                game_version,
                include_all_versions,
                use_color,
            ),

            Commands::ValidatePo { po, strict, format } => {
                debug!(event = "validate_po_args", po = ?po, strict = strict);

                let entries = parse_po_basic(&po)?;
                let mut mismatches = Vec::new();
                let mut checked = 0usize;

                for (ctx, msgid, msgstr, reference) in entries {
                    // пропускаем заголовок PO (msgid "") и пустые переводы
                    if msgid.is_empty() {
                        continue;
                    }
                    if msgstr.trim().is_empty() {
                        continue;
                    }
                    checked += 1;

                    let src_ph = extract_placeholders(&msgid);
                    let dst_ph = extract_placeholders(&msgstr);

                    if src_ph != dst_ph {
                        mismatches.push((ctx, reference, msgid, msgstr, src_ph, dst_ph));
                    }
                }

                if format == "json" {
                    #[derive(serde::Serialize)]
                    struct PoMismatch<'a> {
                        context: Option<&'a str>,
                        reference: Option<&'a str>,
                        msgid: &'a str,
                        msgstr: &'a str,
                        expected_placeholders: Vec<String>,
                        got_placeholders: Vec<String>,
                    }
                    let items: Vec<PoMismatch> = mismatches
                        .iter()
                        .map(|(ctx, reference, id, strv, src_ph, dst_ph)| PoMismatch {
                            context: ctx.as_deref(),
                            reference: reference.as_deref(),
                            msgid: id.as_str(),
                            msgstr: strv.as_str(),
                            expected_placeholders: src_ph.iter().cloned().collect(),
                            got_placeholders: dst_ph.iter().cloned().collect(),
                        })
                        .collect();
                    serde_json::to_writer(std::io::stdout().lock(), &items)?;
                    if strict && !items.is_empty() {
                        color_eyre::eyre::bail!(tr!("validate-po-error"));
                    }
                    return Ok(());
                }
                if mismatches.is_empty() {
                    if use_color {
                        use owo_colors::OwoColorize;
                        println!("{} {}", "✔".green(), tr!("validate-po-ok", count = checked));
                    } else {
                        println!("✔ {}", tr!("validate-po-ok", count = checked));
                    }
                    Ok(())
                } else {
                    for (ctx, reference, id, strv, src_ph, dst_ph) in &mismatches {
                        let ctxt_s = ctx.as_deref().unwrap_or("");
                        let ref_s = reference.as_deref().unwrap_or("");

                        if use_color {
                            use owo_colors::OwoColorize;
                            println!(
                                "{} {}",
                                "✖".red(),
                                tr!(
                                    "validate-po-mismatch",
                                    ctxt = ctxt_s.to_string(),
                                    reference = ref_s.to_string()
                                )
                            );
                            println!("    {}", tr!("validate-po-msgid", value = id));
                            println!("    {}", tr!("validate-po-msgstr", value = strv));
                            println!(
                                "{}",
                                tr!("validate-po-expected", ph = format!("{:?}", src_ph))
                            );
                            println!("{}", tr!("validate-po-got", ph = format!("{:?}", dst_ph)));
                        } else {
                            println!(
                                "✖ {}",
                                tr!(
                                    "validate-po-mismatch",
                                    ctxt = ctxt_s.to_string(),
                                    reference = ref_s.to_string()
                                )
                            );
                            println!("    {}", tr!("validate-po-msgid", value = id));
                            println!("    {}", tr!("validate-po-msgstr", value = strv));
                            println!(
                                "{}",
                                tr!("validate-po-expected", ph = format!("{:?}", src_ph))
                            );
                            println!("{}", tr!("validate-po-got", ph = format!("{:?}", dst_ph)));
                        }
                    }
                    if use_color {
                        use owo_colors::OwoColorize;
                        println!(
                            "{} {}",
                            "✖".red(),
                            tr!("validate-po-total-mismatches", count = mismatches.len())
                        );
                    } else {
                        println!(
                            "✖ {}",
                            tr!("validate-po-total-mismatches", count = mismatches.len())
                        );
                    }

                    if strict {
                        color_eyre::eyre::bail!(tr!("validate-po-error"));
                    } else {
                        Ok(())
                    }
                }
            }

            Commands::DiffXml {
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                baseline_po,
                format,
                strict,
                out_dir,
                game_version,
            } => commands::diff_xml::run_diff_xml(
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                baseline_po,
                format,
                strict,
                out_dir,
                game_version,
            ),

            Commands::Annotate {
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                comment_prefix,
                dry_run,
                backup,
                strip,
                game_version,
            } => commands::annotate::run_annotate(
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                comment_prefix,
                dry_run,
                backup,
                strip,
                game_version,
            ),

            Commands::ExportPo {
                root,
                out_po,
                lang,
                pot,
                source_lang,
                source_lang_dir,
                tm_root,
                game_version,
                include_all_versions,
            } => commands::export_po::run_export_po(
                root,
                out_po,
                if pot { None } else { lang },
                source_lang,
                source_lang_dir,
                tm_root,
                game_version,
                include_all_versions,
            ),

            Commands::ImportPo {
                po,
                out_xml,
                mod_root,
                lang,
                lang_dir,
                keep_empty,
                dry_run,
                backup,
                single_file,
                game_version,
                format,
                report,
                incremental,
            } => commands::import_po::run_import_po(
                po,
                out_xml,
                mod_root,
                lang,
                lang_dir,
                keep_empty,
                dry_run,
                backup,
                single_file,
                game_version,
                format,
                report,
                incremental,
            ),

            Commands::BuildMod {
                po,
                out_mod,
                lang,
                from_root,
                from_game_version,
                name,
                package_id,
                rw_version,
                lang_dir,
                dry_run,
                dedupe,
            } => commands::build_mod::run_build_mod(
                po,
                out_mod,
                lang,
                from_root,
                from_game_version,
                name,
                package_id,
                rw_version,
                lang_dir,
                dry_run,
                dedupe,
            ),

            Commands::XmlHealth {
                root,
                format,
                lang_dir,
                strict,
            } => commands::xml_health::run_xml_health(root, format, lang_dir, strict),

            Commands::Init {
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                overwrite,
                dry_run,
                game_version,
            } => commands::init::run_init(
                root,
                source_lang,
                source_lang_dir,
                lang,
                lang_dir,
                overwrite,
                dry_run,
                game_version,
            ),

            Commands::Morph {
                root,
                provider,
                lang,
                lang_dir,
                filter_key_regex,
                limit,
                game_version,
            } => commands::morph::run_morph(
                root,
                provider,
                lang,
                lang_dir,
                filter_key_regex,
                limit,
                game_version,
            ),
        };

        match &result {
            Ok(_) => info!(event = "cmd_ok", cmd = %cmd_name),
            Err(e) => error!(event = "cmd_error", cmd = %cmd_name, error = ?e),
        }

        result
    }
}

fn init_tracing() {
    use std::fs;

    let log_dir: String = std::env::var("RIMLOC_LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    // гарантируем, что каталог есть
    let _ = fs::create_dir_all(&log_dir);

    // Лог в файл (daily rotation в logs/rimloc.log) — всегда DEBUG и выше
    let file_appender = tracing_appender::rolling::daily(&log_dir, "rimloc.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    // держим guard живым до завершения процесса
    let _ = LOG_GUARD.set(guard);

    // Лог в консоль — уровень управляем через RUST_LOG (по умолчанию INFO)
    let console_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

    // Лог в файл — формат настраиваем через RIMLOC_LOG_FORMAT=json|text (по умолчанию text), уровень DEBUG
    let file_fmt = std::env::var("RIMLOC_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    if file_fmt.eq_ignore_ascii_case("json") {
        tracing_subscriber::registry()
            .with(console_layer)
            .with(
                fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_target(true)
                    .with_writer(file_writer)
                    .with_filter(EnvFilter::new("debug")),
            )
            .with(ErrorLayer::default())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(console_layer)
            .with(
                fmt::layer()
                    .with_ansi(false)
                    .with_target(true)
                    .with_writer(file_writer)
                    .with_filter(EnvFilter::new("debug")),
            )
            .with(ErrorLayer::default())
            .init();
    }
}

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .capture_span_trace_by_default(true)
        .install()?;

    init_tracing();
    init_i18n();

    // Pre-read --ui-lang from raw args so that help can be localized
    let pre_ui_lang = pre_scan_ui_lang();
    set_ui_lang(pre_ui_lang.as_deref());

    // --- Startup banner (stdout только в интерактивном терминале и если не quiet/no-banner) ---
    let version = env!("CARGO_PKG_VERSION");
    let rustlog = std::env::var("RUST_LOG").unwrap_or_else(|_| "None".to_string());
    let rustlog_ref = rustlog.as_str();
    let logdir = resolve_log_dir();

    let quiet_pre = pre_scan_quiet();

    let show_banner =
        std::io::stdout().is_terminal() && std::env::var_os("NO_BANNER").is_none() && !quiet_pre;
    if show_banner {
        ui_out!(
            "app-started",
            version = version,
            logdir = resolve_log_dir().display().to_string(),
            rustlog = rustlog_ref
        );
    }

    // Always mirror startup to stderr unless quiet
    if !quiet_pre {
        ui_info!(
            "app-started",
            version = version,
            logdir = resolve_log_dir().display().to_string(),
            rustlog = rustlog_ref
        );
    }

    info!(
        event = "app_started",
        version = version,
        logdir = %logdir.display(),
        rustlog = %rustlog
    );
    // --- End of startup banner ---

    // Build localized clap::Command and parse
    let mut cmd = <Cli as clap::CommandFactory>::command();
    cmd = localize_command(cmd);

    let matches = cmd.get_matches();
    let cli =
        <Cli as clap::FromArgMatches>::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    let use_color =
        !cli.no_color && std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    // Suppress banner if --quiet was passed (runtime check post-parse)
    if cli.quiet {
        // No additional action needed; banner already gated by terminal check.
        // Future non-essential prints should honor this flag as needed.
    }

    cli.cmd.run(use_color)
}
