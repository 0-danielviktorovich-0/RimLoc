use rimloc_validate::validate;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;
use std::io::IsTerminal;

#[derive(Parser)]
#[command(name = "rimloc", version, about = "RimWorld localization toolkit (Rust)")]
struct Cli {
    /// Выключить цветной вывод
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Сканировать папку мода и извлечь Keyed XML
    Scan {
        /// Путь к корню мода (или к Languages/<locale>)
        #[arg(short, long)]
        root: PathBuf,
        /// Записать результат в CSV-файл (stdout, если не указан)
        #[arg(long)]
        out_csv: Option<PathBuf>,
        /// Язык (например: ru, ja, de). Если указан, добавится колонка 'lang'
        #[arg(long)]
        lang: Option<String>,
    },

    /// Проверить строки на ошибки/замечания
    Validate {
        /// Путь к корню мода (или Languages/<locale>)
        #[arg(short, long)]
        root: PathBuf,
    },

    /// Экспорт извлечённых строк в единый .po файл
    ExportPo {
        /// Путь к корню мода (или Languages/<locale>)
        #[arg(short, long)]
        root: PathBuf,
        /// Путь к результирующему .po
        #[arg(long)]
        out_po: PathBuf,
        /// Язык перевода (например: ru, ja, de)
        #[arg(long)]
        lang: Option<String>,
    },
    
    /// Импорт из .po в LanguageData XML (MVP: один файл)
    ImportPo {
        /// Путь к входному .po
        #[arg(long)]
        po: PathBuf,
        /// Путь к выходному XML (например: Languages/ru/Keyed/_Imported.xml)
        #[arg(long)]
        out_xml: PathBuf,
    },

    /// Собрать отдельный мод-перевод из .po
    BuildMod {
        /// Входной .po
        #[arg(long)]
        po: PathBuf,
        /// Папка для выходного мода (будет создана)
        #[arg(long)]
        out_mod: PathBuf,
        /// Язык перевода (например: ru)
        #[arg(long)]
        lang: String,
        /// Имя мода (About/name)
        #[arg(long, default_value = "RimLoc Translation")]
        name: String,
        /// PackageId мода (About/packageId)
        #[arg(long, default_value = "yourname.rimloc.translation")]
        package_id: String,
        /// Версия RimWorld
        #[arg(long, default_value = "1.5")]
        rw_version: String,
    },
    
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    // Решаем, использовать ли цвета
    let use_color = !cli.no_color
        && std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none();

    match cli.cmd {
        Commands::Scan { root, out_csv, lang } => {
            let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;
            if let Some(path) = out_csv {
            let file = std::fs::File::create(path)?;
            rimloc_export_csv::write_csv(file, &units, lang.as_deref())?;
        } else {
            let stdout = std::io::stdout();
            let lock = stdout.lock();
            rimloc_export_csv::write_csv(lock, &units, lang.as_deref())?;
        }
        }


        Commands::Validate { root } => {
            let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;
            let msgs = validate(&units)?;
            if msgs.is_empty() {
                println!("✔ Всё чисто, ошибок не найдено");
            } else {
                for m in msgs {
                    if !use_color {
                        println!(
                            "[{}] {} ({}:{}) — {}",
                            m.kind,
                            m.key,
                            m.path,
                            m.line.unwrap_or(0),
                            m.message
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
        }
        
        Commands::ExportPo { root, out_po, lang } => {
            let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;
            rimloc_export_po::write_po(&out_po, &units, lang.as_deref())?;
            println!("✔ PO saved to {}", out_po.display());
        }
        
        Commands::ImportPo { po, out_xml } => {
            // читаем записи из .po (key/msgctxt, value/msgstr, reference опционально)
            let entries = rimloc_import_po::read_po_entries(&po)?;
            if entries.is_empty() {
                println!("ℹ В .po нет непустых переводов — ничего не импортировано");
            } else {
                // для XML-импорта нужен просто (key, value)
                let pairs: Vec<(String, String)> =
                    entries.into_iter().map(|e| (e.key, e.value)).collect();

                // опционально можно отсортировать:
                // let mut pairs = pairs;
                // pairs.sort_by(|a, b| a.0.cmp(&b.0));

                rimloc_import_po::write_language_data_xml(&out_xml, &pairs)?;
                println!("✔ XML сохранён в {}", out_xml.display());
            }
        }

        Commands::BuildMod { po, out_mod, lang, name, package_id, rw_version } => {
            rimloc_import_po::build_translation_mod(&po, &out_mod, &lang, &name, &package_id, &rw_version)?;
            println!("✔ Translation mod built at {}", out_mod.display());
        }
    
    }

    Ok(())
}
