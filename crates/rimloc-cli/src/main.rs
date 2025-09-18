use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rimloc", version, about = "RimWorld localization toolkit (Rust)")]
struct Cli {
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
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

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
    }

    Ok(())
}
