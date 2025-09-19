use rimloc_core::TransUnit;
use std::io::Write;
use color_eyre::eyre::Result;

pub fn write_csv<W: Write>(writer: W, units: &[TransUnit], lang: Option<&str>) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);

    // Заголовок: с колонкой lang, если указан язык
    match lang {
        Some(_) => wtr.write_record(["lang", "key", "source", "path", "line"])?,
        None => wtr.write_record(["key", "source", "path", "line"])?,
    }

    for u in units {
        let line_str = u.line.map(|l| l.to_string()).unwrap_or_default();
        let path_str = u.path.to_string_lossy();
        let source = u.source.as_deref().unwrap_or("");

        match lang {
            Some(l) => wtr.write_record([l, &u.key, source, &path_str, &line_str])?,
            None => wtr.write_record([&u.key, source, &path_str, &line_str])?,
        }
    }

    wtr.flush()?;
    Ok(())
}
