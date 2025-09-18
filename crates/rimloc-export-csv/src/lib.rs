use rimloc_core::TransUnit;
use std::io::Write;
use color_eyre::eyre::Result;

pub fn write_csv<W: Write>(writer: W, units: &[TransUnit]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);
    wtr.write_record(["key", "source", "path", "line"])?;

    for u in units {
        wtr.write_record([
            &u.key,
            u.source.as_deref().unwrap_or(""),
            &u.path.to_string_lossy(),
            &u.line.map(|l| l.to_string()).unwrap_or_default(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
