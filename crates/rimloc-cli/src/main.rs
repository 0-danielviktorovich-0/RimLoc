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
        Commands::Scan { root, out_csv } => {
            let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;
            if let Some(path) = out_csv {
                let file = std::fs::File::create(path)?;
                rimloc_export_csv::write_csv(file, &units)?;
            } else {
                let stdout = std::io::stdout();
                let lock = stdout.lock();
                rimloc_export_csv::write_csv(lock, &units)?;
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
        
        Commands::ExportPo { root, out_po } => {
            let units = rimloc_parsers_xml::scan_keyed_xml(&root)?;
            rimloc_export_po::write_po(&out_po, &units)?;
            println!("✔ PO saved to {}", out_po.display());
        }
    
    }

    Ok(())
}
