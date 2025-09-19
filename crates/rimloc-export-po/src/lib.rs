use color_eyre::eyre::Result;
use rimloc_core::TransUnit;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;

fn escape_po(s: &str) -> String {
    // Простейшее экранирование для PO-строк (достаточно для MVP):
    // \ -> \\, " -> \", \n -> \n, \r -> \r, \t -> \t
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"'  => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _    => out.push(ch),
        }
    }
    out
}

/// Записать единый .po файл с заголовком и всеми TransUnit.
/// msgctxt = ключ, msgid = исходный текст (source), msgstr = "" (пусто, готово к переводу)
/// В комментарии `#:` пишем ссылку `path:line`.
pub fn write_po(path: &Path, units: &[TransUnit], lang: Option<&str>) -> Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // --- Header ---
    writeln!(w, "msgid \"\"")?;
    writeln!(w, "msgstr \"\"")?;
    writeln!(w, "\"Project-Id-Version: rimloc 0.1\\n\"")?;
    writeln!(w, "\"POT-Creation-Date: \\n\"")?;
    writeln!(w, "\"PO-Revision-Date: \\n\"")?;
    writeln!(w, "\"Last-Translator: \\n\"")?;
    writeln!(w, "\"Language-Team: \\n\"")?;
    if let Some(l) = lang {
        writeln!(w, "\"Language: {}\\n\"", l)?;
    } else {
        writeln!(w, "\"Language: \\n\"")?;
    }
    writeln!(w, "\"MIME-Version: 1.0\\n\"")?;
    writeln!(w, "\"Content-Type: text/plain; charset=UTF-8\\n\"")?;
    writeln!(w, "\"Content-Transfer-Encoding: 8bit\\n\"")?;
    writeln!(w)?; // пустая строка

    // --- Entries ---
    for u in units {
        let key = &u.key;
        let msgid = u.source.as_deref().unwrap_or("");

        if let Some(line) = u.line {
            writeln!(w, "#: {}:{}", u.path.display(), line)?;
        } else {
            writeln!(w, "#: {}", u.path.display())?;
        }

        writeln!(w, "msgctxt \"{}\"", escape_po(key))?;
        writeln!(w, "msgid \"{}\"", escape_po(msgid))?;
        writeln!(w, "msgstr \"\"")?;
        writeln!(w)?;
    }

    w.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rimloc_core::TransUnit;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn unit(key: &str, src: &str, line: usize) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(src.into()),
            path: PathBuf::from("/Mod/Languages/English/Keyed/A.xml"),
            line: Some(line as u32),
        }
    }

    #[test]
    fn po_contains_header_context_and_references() {
        let tmp = NamedTempFile::new().unwrap();
        let units = vec![
            unit("Greeting", "Hello", 3),
            unit("Farewell", "Bye", 7),
        ];
        write_po(tmp.path(), &units, Some("ru")).unwrap();

        let s = fs::read_to_string(tmp.path()).unwrap();
        assert!(s.contains(r#""Language: ru\n""#));
        assert!(s.contains(r#"msgctxt "Greeting""#));
        assert!(s.contains(r#"msgid "Hello""#));
        assert!(s.contains(r#"#: /Mod/Languages/English/Keyed/A.xml:3"#));
    }
}
