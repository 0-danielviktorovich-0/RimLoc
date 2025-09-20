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
    
    /// Импорт .po: либо в ОДИН файл, либо раскладкой в существующий мод
    ///
    /// Варианты:
    ///   A) --out-xml <файл>                       → записывает всё в один XML
    ///   B) --mod-root <папка> [--lang/--lang-dir] → раскладывает по структуре мода
    ImportPo {
        /// Путь к входному .po
        #[arg(long)]
        po: PathBuf,

        /// (Вариант A) Путь к выходному XML (один файл, например: Languages/Russian/Keyed/_Imported.xml)
        #[arg(long, conflicts_with = "mod_root")]
        out_xml: Option<PathBuf>,

        /// (Вариант B) Корень существующего мода (где лежат About, Languages, …)
        #[arg(long, conflicts_with = "out_xml")]
        mod_root: Option<PathBuf>,

        /// Код языка (ru/ja/de). Если не задан --lang-dir, то по нему выбирается имя папки языка.
        #[arg(long)]
        lang: Option<String>,

        /// Явное имя папки языка (например: Russian). Имеет приоритет над --lang.
        #[arg(long)]
        lang_dir: Option<String>,

        /// Сохранять пустые переводы (msgstr == "" ). По умолчанию пустые пропускаем.
        #[arg(long, default_value_t = false)]
        keep_empty: bool,

        /// Сухой прогон: только показать план (куда и сколько ключей пойдёт), файлы не записывать.
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Если файл назначения существует — создать .bak рядом (до перезаписи).
        #[arg(long, default_value_t = false)]
        backup: bool,

        /// Принудительно писать всё в один файл _Imported.xml даже при --mod-root
        #[arg(long, default_value_t = false)]
        single_file: bool,
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

        /// (Необязательно) Явное имя папки языка в RimWorld (например: Russian).
        /// Если не задано — выбирается автоматически по --lang.
        #[arg(long)]
        lang_dir: Option<String>,
        
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
            use owo_colors::OwoColorize;
            use std::fs;

            // Читаем .po как записи (key/msgctxt, value/msgstr, optional reference)
            let mut entries = rimloc_import_po::read_po_entries(&po)?;

            // Фильтр пустых переводов (по умолчанию пропускаем пустые msgstr)
            if !keep_empty {
                entries.retain(|e| !e.value.trim().is_empty());
            }

            // Вариант A: явный один файл (out_xml задан)
            if let Some(out) = out_xml {
                if dry_run {
                    println!(
                        "DRY-RUN: записали бы {} ключ(ей) в {}",
                        entries.len(),
                        out.display()
                    );
                    return Ok(());
                }

                // backup при необходимости
                if backup && out.exists() {
                    let bak = out.with_extension("xml.bak");
                    fs::copy(&out, &bak)?;
                    eprintln!("backup: {} → {}", out.display(), bak.display());
                }

                // (key,value) пары
                let pairs: Vec<(String, String)> =
                    entries.into_iter().map(|e| (e.key, e.value)).collect();

                rimloc_import_po::write_language_data_xml(&out, &pairs)?;
                println!("✔ XML сохранён в {}", out.display());
                return Ok(());
            }

            // Вариант B: раскладка по структуре мода
            let Some(root) = mod_root else {
                // Ничего не задано: подскажем как правильно
                eprintln!(
                    "{} ни --out-xml, ни --mod-root не указаны.\n\
                     Укажите ОДНО из:\n  --out-xml <файл>\n  ИЛИ\n  --mod-root <путь_к_моду> [--lang ru | --lang-dir Russian]",
                    "Ошибка:".red()
                );
                std::process::exit(2);
            };

            // Определяем имя папки языка
            let lang_folder = if let Some(dir) = lang_dir {
                dir
            } else if let Some(code) = lang {
                rimloc_import_po::rimworld_lang_dir(&code)
            } else {
                // разумный дефолт
                "Russian".to_string()
            };

            // Если попросили «в один файл» даже с mod_root
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
                    eprintln!("backup: {} → {}", out.display(), bak.display());
                }

                let pairs: Vec<(String, String)> =
                    entries.into_iter().map(|e| (e.key, e.value)).collect();
                rimloc_import_po::write_language_data_xml(&out, &pairs)?;
                println!("✔ XML сохранён в {}", out.display());
                return Ok(());
            }

            // Иначе: раскладываем по reference (DefInjected/..., Keyed/..., и т.д.)
            use regex::Regex;
            use std::collections::HashMap;
            let re = Regex::new(r"/Languages/([^/]+)/(?P<rel>.+?)(?::\d+)?$").unwrap();
            let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();

            for e in entries {
                let rel = e.reference.as_ref()
                    .and_then(|r| re.captures(r))
                    .and_then(|c| c.name("rel").map(|m| PathBuf::from(m.as_str())))
                    .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml"));
                grouped.entry(rel).or_default().push((e.key, e.value));
            }

            // Сухой прогон?
            if dry_run {
                println!("DRY-RUN план:");
                let mut keys_total = 0usize;
                let mut paths: Vec<_> = grouped.keys().cloned().collect();
                paths.sort();
                for rel in paths {
                    let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
                    keys_total += n;
                    println!("  {}  ({} ключей)", root.join("Languages").join(&lang_folder).join(&rel).display(), n);
                }
                println!("ИТОГО: {} ключ(ей)", keys_total);
                return Ok(());
            }

            // Запись по файлам
            for (rel, items) in grouped {
                let out_path = root.join("Languages").join(&lang_folder).join(&rel);
                // backup, если нужно
                if backup && out_path.exists() {
                    let bak = out_path.with_extension("xml.bak");
                    fs::copy(&out_path, &bak)?;
                    eprintln!("backup: {} → {}", out_path.display(), bak.display());
                }
                rimloc_import_po::write_language_data_xml(&out_path, &items)?;
            }

            println!("✔ Импорт выполнен в {}", root.display());
        }

        Commands::BuildMod { po, out_mod, lang, name, package_id, rw_version, lang_dir } => {
            // Если пользователь задал --lang-dir, используем его; иначе — автокарта.
            let lang_folder = lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang));

            // Временно «подменим» lang для сборки:
            // добавим новый метод build_translation_mod_with_langdir, чтобы явно передать имя папки.
            rimloc_import_po::build_translation_mod_with_langdir(
                &po, &out_mod, &lang_folder, &name, &package_id, &rw_version
            )?;
            println!("✔ Translation mod built at {}", out_mod.display());
        }

    
    }

    Ok(())
}
