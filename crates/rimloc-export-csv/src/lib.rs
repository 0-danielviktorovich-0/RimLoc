use rimloc_core::{Result, TransUnit};
use std::io::Write;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unit(key: &str, src: &str) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(src.into()),
            path: PathBuf::from("/mod/Languages/English/Keyed/X.xml"),
            line: Some(42),
        }
    }

    #[test]
    fn csv_without_lang_column() {
        let units = vec![unit("A", "Hello"), unit("B", "World")];
        let mut buf: Vec<u8> = Vec::new();
        write_csv(&mut buf, &units, None).unwrap();
        let s = String::from_utf8(buf).unwrap();

        assert!(s
            .lines()
            .next()
            .unwrap()
            .starts_with("key,source,path,line"));
        assert!(s.contains("A,Hello,/mod/Languages/English/Keyed/X.xml,42"));
    }

    #[test]
    fn csv_with_lang_column() {
        let units = vec![unit("A", "Hello")];
        let mut buf: Vec<u8> = Vec::new();
        write_csv(&mut buf, &units, Some("ru")).unwrap();
        let s = String::from_utf8(buf).unwrap();

        assert!(s
            .lines()
            .next()
            .unwrap()
            .starts_with("lang,key,source,path,line"));
        assert!(s.contains("ru,A,Hello,/mod/Languages/English/Keyed/X.xml,42"));
    }
}
