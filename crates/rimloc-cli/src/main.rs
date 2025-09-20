use rimloc_validate::validate;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;
use std::io::IsTerminal;
use tracing::{info, warn, error, debug};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::Layer;
use tracing_error::ErrorLayer;
use once_cell::sync::OnceCell;
use tracing_appender::non_blocking::WorkerGuard;

static LOG_GUARD: OnceCell<WorkerGuard> = OnceCell::new();


#[derive(Parser)]
#[command(name = "rimloc", version, about = "RimWorld localization toolkit (Rust)")]
struct Cli {
    /// Выключить цветной вывод
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Сканировать папку мода и извлечь Keyed XML
    Scan {
        #[arg(short, long)]
        root: PathBuf,
        #[arg(long)]
        out_csv: Option<PathBuf>,
        #[arg(long)]
        lang: Option<String>,
        /// Исходный язык по ISO-коду (en, ru, ja...)
        #[arg(long)]
        source_lang: Option<String>,
        /// Имя папки исходного языка (например: English). Приоритет выше.
        #[arg(long)]
        source_lang_dir: Option<String>,
    },

    /// Проверить строки на ошибки/замечания
    Validate {
        #[arg(short, long)]
        root: PathBuf,
        /// Исходный язык по ISO-коду
        #[arg(long)]
        source_lang: Option<String>,
        /// Имя папки исходного языка (например: English)
        #[arg(long)]
        source_lang_dir: Option<String>,
    },

    /// Экспорт извлечённых строк в единый .po файл
    ///
    /// Можно ограничить исходный язык:
    ///   --source-lang-dir English    (папка в Languages/)
    ///   или
    ///   --source-lang en             (ISO-код; будет сопоставлен к имени папки)
    ExportPo {
        /// Путь к корню мода (или Languages/<locale>)
        #[arg(short, long)]
        root: PathBuf,

        /// Путь к результирующему .po
        #[arg(long)]
        out_po: PathBuf,

        /// Язык перевода (пишем в заголовок PO), например: ru, ja, de
        #[arg(long)]
        lang: Option<String>,

        /// Исходный язык по ISO-коду (en, ru, ja...). Будет преобразован через rimworld_lang_dir.
        #[arg(long)]
        source_lang: Option<String>,

        /// Имя папки исходного языка (например: English). Приоритет выше, чем у --source-lang.
        #[arg(long)]
        source_lang_dir: Option<String>,
    },

    /// Импорт .po: либо в один XML, либо раскладкой по структуре существующего мода
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
    },

    /// Собрать отдельный мод-перевод из .po
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

trait Runnable {
    fn run(self, use_color: bool) -> Result<()>;
}

impl Runnable for Commands {
    fn run(self, use_color: bool) -> Result<()> {
        let cmd_name = format!("{:?}", self);
        info!("▶ Starting command: {}", cmd_name);
        let span = tracing::info_span!("cmd", name = %cmd_name);
        let _enter = span.enter();

        let result = match self {
            Commands::Scan { root, out_csv, lang, source_lang, source_lang_dir } => {
                debug!("Scan args: root={:?} out_csv={:?} lang={:?}", root, out_csv, lang);

                let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;

                // опциональный фильтр по исходному языку (папка в Languages)
                let units = if let Some(dir) = source_lang_dir.clone() {
                    let before = units.len();
                    let filtered: Vec<_> = units
                    .into_iter()
                    .filter(|u| is_under_languages_dir(&u.path, dir.as_str()))
                    .collect();
                    info!("Scan: filtered {} -> {} by source_lang_dir={}", before, filtered.len(), dir);
                    filtered
                } else if let Some(code) = source_lang.clone() {
                    let dir = rimloc_import_po::rimworld_lang_dir(&code);
                    let before = units.len();
                    let filtered: Vec<_> = units
                        .into_iter()
                        .filter(|u| is_under_languages_dir(&u.path, dir.as_str()))
                        .collect();
                    info!("Scan: filtered by source_lang={} → dir={}: {} -> {}", code, dir, before, filtered.len());
                    filtered
                } else {
                    units
                };

                if let Some(path) = out_csv {
            let file = std::fs::File::create(path)?;
                rimloc_export_csv::write_csv(file, &units, lang.as_deref())?;
            } else {
                let stdout = std::io::stdout();
                let lock = stdout.lock();
                rimloc_export_csv::write_csv(lock, &units, lang.as_deref())?;
            }
            Ok(())
            }

            Commands::Validate { root, source_lang, source_lang_dir } => {
                debug!("Validate args: root={:?}", root);

                let mut units = rimloc_parsers_xml::scan_keyed_xml(&root)?;

                // опциональный фильтр по исходному языку
                if let Some(dir) = source_lang_dir.as_ref() {
                    units.retain(|u| is_under_languages_dir(&u.path, dir.as_str()));
                    info!("Validate: filtered by source_lang_dir={}, remaining={}", dir, units.len());
                } else if let Some(code) = source_lang.as_ref() {
                    let dir = rimloc_import_po::rimworld_lang_dir(code);
                    units.retain(|u| is_under_languages_dir(&u.path, dir.as_str()));
                    info!("Validate: filtered by source_lang={} → dir={}, remaining={}", code, dir, units.len());
                }

                let msgs = validate(&units)?;
                if msgs.is_empty() {
                    println!("✔ Всё чисто, ошибок не найдено");
                } else {
                    for m in msgs {
                        if !use_color {
                            println!(
                                "[{}] {} ({}:{}) — {}",
                                m.kind, m.key, m.path, m.line.unwrap_or(0), m.message
                            );
                        } else {
                            use owo_colors::OwoColorize;
                            let tag = match m.kind.as_str() {
                                "duplicate" => "⚠",
                                "empty" => "✖",
                                "placeholder-check" => "ℹ",
                                _ => "•",
                            };
                            let colored_kind: String = match m.kind.as_str() {
                                "duplicate" => format!("{}", m.kind.yellow()),
                                "empty" => format!("{}", m.kind.red()),
                                "placeholder-check" => format!("{}", m.kind.cyan()),
                                _ => format!("{}", m.kind.white()),
                            };
                            println!(
                                "{} [{}] {} ({}:{}) — {}",
                                tag,
                                colored_kind,
                                m.key.green(),
                                m.path.blue(),
                                m.line.unwrap_or(0).to_string().magenta(),
                                m.message
                            );
                        }
                    }
                }
                Ok(())
            }

            Commands::ExportPo { root, out_po, lang, source_lang, source_lang_dir } => {
                debug!(
                    "ExportPo args: root={:?} out_po={:?} lang={:?} source_lang={:?} source_lang_dir={:?}",
                    root, out_po, lang, source_lang, source_lang_dir
                );

                // 1) Сканируем все юниты
                let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;

                // 2) Определяем папку исходного языка:
                //    - если задан --source-lang-dir → берём его
                //    - иначе если задан --source-lang → маппим в rimworld_lang_dir(...)
                //    - иначе по умолчанию "English"
                let src_dir = if let Some(dir) = source_lang_dir {
                    dir
                } else if let Some(code) = source_lang {
                    rimloc_import_po::rimworld_lang_dir(&code)
                } else {
                    "English".to_string()
                };
                info!("Exporting from {}", src_dir);

                // 3) Фильтруем только те записи, чей путь находится под Languages/<src_dir>/
                let filtered: Vec<_> = units
                    .into_iter()
                    .filter(|u| is_under_languages_dir(&u.path, &src_dir))
                    .collect();

                info!("Exporting {} unit(s) from {}", filtered.len(), src_dir);

                // 4) Пишем PO (язык назначения как и раньше — опциональное поле в заголовке)
                rimloc_export_po::write_po(&out_po, &filtered, lang.as_deref())?;
                println!("✔ PO saved to {}", out_po.display());
                Ok(())
            }

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
            } => {
                debug!("ImportPo args: po={:?} out_xml={:?} mod_root={:?} lang={:?} lang_dir={:?} keep_empty={} dry_run={} backup={} single_file={}",
                    po, out_xml, mod_root, lang, lang_dir, keep_empty, dry_run, backup, single_file
                );
                use std::fs;

                let mut entries = rimloc_import_po::read_po_entries(&po)?;
                debug!("Loaded {} entries from PO", entries.len());

                if !keep_empty {
                    let before = entries.len();
                    entries.retain(|e| !e.value.trim().is_empty());
                    debug!("Filtered empty: {} -> {}", before, entries.len());
                    if entries.is_empty() {
                        warn!("PO содержит только пустые строки. Добавьте --keep-empty, если хотите импортировать их как заглушки.");
                    }
                }

                if let Some(out) = out_xml {
                    if dry_run {
                        println!(
                            "DRY-RUN: записали бы {} ключ(ей) в {}",
                            entries.len(),
                            out.display()
                        );
                        return Ok(());
                    }

                    if backup && out.exists() {
                        let bak = out.with_extension("xml.bak");
                        fs::copy(&out, &bak)?;
                        warn!("backup: {} → {}", out.display(), bak.display());
                    }

                    let pairs: Vec<(String, String)> =
                        entries.into_iter().map(|e| (e.key, e.value)).collect();
                    rimloc_import_po::write_language_data_xml(&out, &pairs)?;
                    println!("✔ XML сохранён в {}", out.display());
                    return Ok(());
                }

                let Some(root) = mod_root else {
                    eprintln!("Ошибка: нужно указать --out-xml или --mod-root");
                    std::process::exit(2);
                };

                let lang_folder = if let Some(dir) = lang_dir {
                    dir
                } else if let Some(code) = lang {
                    rimloc_import_po::rimworld_lang_dir(&code)
                } else {
                    "Russian".to_string()
                };
                debug!("Resolved lang folder: {}", lang_folder);

                if single_file {
                    let out = root.join("Languages").join(&lang_folder).join("Keyed").join("_Imported.xml");

                    if dry_run {
                        println!(
                            "DRY-RUN: записали бы {} ключ(ей) в {}",
                            entries.len(),
                            out.display()
                        );
                        return Ok(());
                    }

                    if backup && out.exists() {
                        let bak = out.with_extension("xml.bak");
                        fs::copy(&out, &bak)?;
                        warn!("backup: {} → {}", out.display(), bak.display());
                    }

                    let pairs: Vec<(String, String)> =
                        entries.into_iter().map(|e| (e.key, e.value)).collect();
                    rimloc_import_po::write_language_data_xml(&out, &pairs)?;
                    println!("✔ XML сохранён в {}", out.display());
                    return Ok(());
                }

                use regex::Regex;
                use std::collections::HashMap;
                let re = Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](?P<rel>.+?)(?::\d+)?$").unwrap();
                let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();

                for e in entries {
                    let rel = e.reference.as_ref()
                        .and_then(|r| re.captures(r))
                        .and_then(|c| c.name("rel").map(|m| PathBuf::from(m.as_str())))
                        .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml"));
                    grouped.entry(rel).or_default().push((e.key, e.value));
                }

                if dry_run {
                    println!("DRY-RUN план:");
                    let mut keys_total = 0usize;
                    let mut paths: Vec<_> = grouped.keys().cloned().collect();
                    paths.sort();
                    for rel in paths {
                        let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
                        keys_total += n;
                        println!(
                            "  {}  ({} ключей)",
                            root.join("Languages").join(&lang_folder).join(&rel).display(),
                            n
                        );
                    }
                    println!("ИТОГО: {} ключ(ей)", keys_total);
                    return Ok(());
                }

                for (rel, items) in grouped {
                    let out_path = root.join("Languages").join(&lang_folder).join(&rel);
                    if backup && out_path.exists() {
                        let bak = out_path.with_extension("xml.bak");
                        std::fs::copy(&out_path, &bak)?;
                        warn!("backup: {} → {}", out_path.display(), bak.display());
                    }
                    rimloc_import_po::write_language_data_xml(&out_path, &items)?;
                }

                println!("✔ Импорт выполнен в {}", root.display());
                Ok(())
            }

            Commands::BuildMod { po, out_mod, lang, name, package_id, rw_version, lang_dir, dry_run } => {
                debug!("BuildMod args: po={:?} out_mod={:?} lang={} name={} package_id={} rw_version={} lang_dir={:?} dry_run={}",
                    po, out_mod, lang, name, package_id, rw_version, lang_dir, dry_run
                );
                let lang_folder = lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang));

                if dry_run {
                    rimloc_import_po::build_translation_mod_dry_run(
                        &po, &out_mod, &lang_folder, &name, &package_id, &rw_version
                    )?;
                } else {
                    rimloc_import_po::build_translation_mod_with_langdir(
                        &po, &out_mod, &lang_folder, &name, &package_id, &rw_version
                    )?;
                    println!("✔ Translation mod built at {}", out_mod.display());
                }
                Ok(())
            }
        };

        match &result {
            Ok(_) => info!("✔ Finished command: {}", cmd_name),
            Err(e) => error!("✖ Command {} failed: {:?}", cmd_name, e),
        }

        result
    }
}

fn init_tracing() {
    use std::fs;

    // гарантируем, что каталог есть
    let _ = fs::create_dir_all("logs");

    // Лог в файл (daily rotation в logs/rimloc.log) — всегда DEBUG и выше
    let file_appender = tracing_appender::rolling::daily("logs", "rimloc.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    // держим guard живым до завершения процесса
    let _ = LOG_GUARD.set(guard);


    // Лог в консоль — уровень управляем через RUST_LOG (по умолчанию INFO)
    let console_layer = fmt::layer()
        .with_target(false)
        .with_writer(std::io::stdout)
        .with_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
        );

    // Лог в файл — фиксированно DEBUG
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(file_writer)
        .with_filter(EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(ErrorLayer::default()) // ← вот сюда добавляем
        .init();

}

fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .capture_span_trace_by_default(true)  // писать span-трассу в отчёты об ошибках
        .install()?;

    init_tracing();

    info!("rimloc started • version={} • logdir=logs/ • RUST_LOG={:?}",
          env!("CARGO_PKG_VERSION"),
          std::env::var("RUST_LOG").ok());

    let cli = Cli::parse();

    let use_color = !cli.no_color
        && std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none();

    cli.cmd.run(use_color)
}
