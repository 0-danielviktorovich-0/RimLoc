// Clippy: simplify complex tuple types
type PoEntry = (Option<String>, String, String, Option<String>);

// use rimloc_validate::validate; // moved into commands
include!(concat!(env!("OUT_DIR"), "/supported_locales.rs"));
use clap::{Command as ClapCommand, Parser, Subcommand};
use color_eyre::eyre::Result;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageRequester;
use once_cell::sync::OnceCell;
use regex::Regex;
use rust_embed::RustEmbed;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::io::{ErrorKind, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VersionEntry {
    name: String,
    components: Vec<u32>,
    path: PathBuf,
}

#[allow(dead_code)]
fn parse_version_components(name: &str) -> Option<Vec<u32>> {
    let trimmed = name.trim_start_matches('v');
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for part in trimmed.split('.') {
        if part.is_empty() {
            return None;
        }
        let value: u32 = part.parse().ok()?;
        parts.push(value);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

#[allow(dead_code)]
fn normalize_version_input(raw: &str) -> String {
    raw.trim_start_matches('v').to_string()
}

#[allow(dead_code)]
fn find_version_directory(base: &Path, requested: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let normalized = normalize_version_input(requested);
    if requested.starts_with('v') {
        candidates.push(requested.trim().to_string());
        candidates.push(normalized.clone());
    } else {
        candidates.push(normalized.clone());
        candidates.push(format!("v{}", normalized));
    }
    for name in candidates.into_iter() {
        if name.is_empty() {
            continue;
        }
        let candidate = base.join(&name);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

#[allow(dead_code)]
fn list_version_directories(base: &Path) -> color_eyre::Result<Vec<VersionEntry>> {
    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(base) {
        Ok(iter) => iter,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(entries),
        Err(err) => return Err(err.into()),
    };
    for entry in read_dir {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name_os = entry.file_name();
        let name = match name_os.to_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(components) = parse_version_components(name) {
            entries.push(VersionEntry {
                name: name.to_string(),
                components,
                path: entry.path(),
            });
        }
    }
    Ok(entries)
}

#[allow(dead_code)]
fn is_version_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .and_then(parse_version_components)
        .is_some()
}

#[allow(dead_code)]
fn resolve_game_version_root(
    base: &Path,
    requested: Option<&str>,
) -> color_eyre::Result<(PathBuf, Option<String>)> {
    if is_version_directory(base) {
        let name = base
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        return Ok((base.to_path_buf(), name));
    }

    let mut entries = list_version_directories(base)?;

    if let Some(req) = requested {
        if let Some(path) = find_version_directory(base, req) {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok((path, name));
        } else {
            return Err(color_eyre::eyre::eyre!(
                "Requested version '{}' not found under {}",
                req,
                base.display()
            ));
        }
    }

    if entries.is_empty() {
        return Ok((base.to_path_buf(), None));
    }

    entries.sort_by(|a, b| {
        let len_cmp = a.components.len().cmp(&b.components.len());
        if len_cmp != Ordering::Equal {
            return len_cmp;
        }
        a.components.cmp(&b.components)
    });

    if let Some(entry) = entries.last() {
        return Ok((entry.path.clone(), Some(entry.name.clone())));
    }

    Ok((base.to_path_buf(), None))
}

macro_rules! tr {
    ($msg:literal $(, $k:ident = $v:expr )* $(,)?) => {{
        let loader = $crate::LANG_LOADER.get().expect("i18n not initialized");
        i18n_embed_fl::fl!(loader, $msg $(, $k = $v )* )
    }};
    ($msg:literal) => {{
        let loader = $crate::LANG_LOADER.get().expect("i18n not initialized");
        i18n_embed_fl::fl!(loader, $msg)
    }}
}

// Centralized, localized UI output helpers
macro_rules! ui_ok {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        println!("✔ {}", tr!($k $(, $n = $v )* ));
    }};
}
macro_rules! ui_info {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        eprintln!("ℹ {}", tr!($k $(, $n = $v )* ));
    }};
}
macro_rules! ui_warn {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        // Show icon only in interactive terminals and when not explicitly disabled.
        let show_icon = std::io::stdout().is_terminal() && std::env::var_os("NO_ICONS").is_none();
        if show_icon {
            eprintln!("⚠ {}", tr!($k $(, $n = $v )* ));
        } else {
            eprintln!("{}", tr!($k $(, $n = $v )* ));
        }
    }};
}
#[allow(unused_macros)]
macro_rules! ui_err {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        eprintln!("✖ {}", tr!($k $(, $n = $v )* ));
    }};
}
macro_rules! ui_out {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        println!("{}", tr!($k $(, $n = $v )* ));
    }};
}

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

/// Apply localized texts (about/help) to the clap Command using tr!()
fn localize_command(mut cmd: ClapCommand) -> ClapCommand {
    // Top-level about
    // Expect FTL key: help-about
    cmd = cmd.about(tr!("help-about"));

    // Top-level args: --no-color, --ui-lang
    cmd = cmd.mut_arg("no_color", |a| a.help(tr!("help-no-color")));
    cmd = cmd.mut_arg("ui_lang", |a| a.help(tr!("help-ui-lang")));

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
            "export-po" => {
                let mut owned = std::mem::take(sc);
                owned = owned.about(tr!("help-exportpo-about"));
                owned = owned.mut_arg("root", |a| a.help(tr!("help-exportpo-root")));
                owned = owned.mut_arg("out_po", |a| a.help(tr!("help-exportpo-out-po")));
                owned = owned.mut_arg("lang", |a| a.help(tr!("help-exportpo-lang")));
                owned = owned.mut_arg("source_lang", |a| a.help(tr!("help-exportpo-source-lang")));
                owned = owned.mut_arg("source_lang_dir", |a| {
                    a.help(tr!("help-exportpo-source-lang-dir"))
                });
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
                *sc = owned;
            }
            _ => {}
        }
    }

    cmd
}

static LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();
const DEFAULT_LOGDIR: &str = "logs";

fn resolve_log_dir() -> std::path::PathBuf {
    if let Ok(val) = std::env::var("RIMLOC_LOGDIR") {
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

        /// Source language by ISO code (mapped via rimworld_lang_dir).
        #[arg(long)]
        source_lang: Option<String>,

        /// Source language folder name (e.g., "English"). Takes precedence over --source-lang.
        #[arg(long)]
        source_lang_dir: Option<String>,
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
    },

    /// Build a standalone translation mod from a .po file (help via FTL).
    BuildMod {
        #[arg(long)]
        po: PathBuf,
        #[arg(long)]
        out_mod: PathBuf,
        #[arg(long)]
        lang: String,
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

/// Извлечь плейсхолдеры: %d, %s, %1$d, %02d, а также {NAME}/{0}
fn extract_placeholders(s: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();

    // %d, %s, %1$d, %02d, %i, %f — базового набора достаточно
    static RE_PCT: OnceLock<Regex> = OnceLock::new();
    let re_pct = RE_PCT.get_or_init(|| Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    for m in re_pct.find_iter(s) {
        set.insert(m.as_str().to_string());
    }

    // {NAME}, {0}, {Any-Thing}
    static RE_BRACE: OnceLock<Regex> = OnceLock::new();
    let re_brace = RE_BRACE.get_or_init(|| Regex::new(r"\{[^}]+\}").unwrap());
    for m in re_brace.find_iter(s) {
        set.insert(m.as_str().to_string());
    }

    set
}

/// Простой парсер .po только для msgid/msgstr (+ msgctxt и #: reference по возможности).
/// Возвращает вектор кортежей: (msgctxt, msgid, msgstr, reference)
fn parse_po_basic(path: &std::path::Path) -> color_eyre::eyre::Result<Vec<PoEntry>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let f = File::open(path)?;
    let rdr = BufReader::new(f);

    let mut entries = Vec::new();
    let mut ctx: Option<String> = None;
    let mut id = String::new();
    let mut strv = String::new();
    let mut refv: Option<String> = None;

    enum Mode {
        None,
        InId,
        InStr,
    }
    let mut mode = Mode::None;

    fn unquote_po(s: &str) -> String {
        // снимаем кавычки и экранирование \" \\ \n \t \r
        let mut out = String::new();
        let raw = s.trim();
        let raw = raw.strip_prefix('"').unwrap_or(raw);
        let raw = raw.strip_suffix('"').unwrap_or(raw);
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    match n {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '\\' => out.push('\\'),
                        '"' => out.push('"'),
                        _ => {
                            out.push('\\');
                            out.push(n);
                        }
                    }
                } else {
                    out.push('\\');
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    let mut push_if_complete = |ctx: &mut Option<String>,
                                id: &mut String,
                                strv: &mut String,
                                refv: &mut Option<String>| {
        if !id.is_empty() || !strv.is_empty() {
            entries.push((
                ctx.clone(),
                std::mem::take(id),
                std::mem::take(strv),
                refv.clone(),
            ));
            *ctx = None;
            *refv = None;
        }
    };

    for line in rdr.lines() {
        let line = line?;
        let t = line.trim();

        if t.is_empty() {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            mode = Mode::None;
            continue;
        }

        if let Some(rest) = t.strip_prefix("msgctxt ") {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            ctx = Some(unquote_po(rest));
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgid ") {
            push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
            id = unquote_po(rest);
            mode = Mode::InId;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgstr ") {
            strv = unquote_po(rest);
            mode = Mode::InStr;
            continue;
        }
        if let Some(rest) = t.strip_prefix("#: ") {
            refv = Some(rest.to_string());
            continue;
        }

        match mode {
            Mode::InId | Mode::InStr => {
                if t.starts_with('"') {
                    let chunk = unquote_po(t);
                    match mode {
                        Mode::InId => id.push_str(&chunk),
                        Mode::InStr => strv.push_str(&chunk),
                        _ => {}
                    }
                }
            }
            Mode::None => {}
        }
    }

    push_if_complete(&mut ctx, &mut id, &mut strv, &mut refv);
    Ok(entries)
}

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

            Commands::ExportPo {
                root,
                out_po,
                lang,
                source_lang,
                source_lang_dir,
                game_version,
                include_all_versions,
            } => commands::export_po::run_export_po(
                root,
                out_po,
                lang,
                source_lang,
                source_lang_dir,
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
            ),

            Commands::BuildMod {
                po,
                out_mod,
                lang,
                name,
                package_id,
                rw_version,
                lang_dir,
                dry_run,
            } => commands::build_mod::run_build_mod(
                po, out_mod, lang, name, package_id, rw_version, lang_dir, dry_run,
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

    // Лог в файл — фиксированно DEBUG
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(file_writer)
        .with_filter(EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(ErrorLayer::default())
        .init();
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

    // --- Startup banner (должен быть только один раз!) ---
    let version = env!("CARGO_PKG_VERSION");
    let rustlog = std::env::var("RUST_LOG").unwrap_or_else(|_| "None".to_string());
    let rustlog_ref = rustlog.as_str();
    let logdir = resolve_log_dir();

    ui_out!(
        "app-started",
        version = version,
        logdir = resolve_log_dir().display().to_string(),
        rustlog = rustlog_ref
    );

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

    cli.cmd.run(use_color)
}
