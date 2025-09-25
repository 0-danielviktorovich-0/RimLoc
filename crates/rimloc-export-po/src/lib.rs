use regex::Regex;
use rimloc_core::Result;
use rimloc_core::TransUnit;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::OnceLock;

fn escape_po(s: &str) -> String {
    // Простейшее экранирование для PO-строк (достаточно для MVP):
    // \ -> \\, " -> \", \n -> \n, \r -> \r, \t -> \t
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn rel_from_languages(path_str: &str) -> Option<String> {
    // Вырезаем подстроку после .../Languages/<locale>/  (кроссплатформенно)
    // Поддерживает и '/' и '\', а также отсутствие префикса каталога.
    static REL_FROM_LANGUAGES: OnceLock<Regex> = OnceLock::new();
    let re = REL_FROM_LANGUAGES
        .get_or_init(|| Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap());
    re.captures(path_str)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Записать единый .po файл с заголовком и всеми TransUnit.
/// msgctxt = "<key>|<relative_path_from_Languages>:<line>" (уникальный контекст для Poedit)
/// msgid   = исходный текст (source), msgstr = "" (пусто, готово к переводу)
/// В комментарии `#:` пишем ссылку `path:line` (для импорта/раскладки по файлам).
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
    // Custom header with RimLoc schema version for tooling
    writeln!(
        w,
        "\"X-RimLoc-Schema: {}\\n\"",
        rimloc_core::RIMLOC_SCHEMA_VERSION
    )?;
    writeln!(w)?; // пустая строка

    // --- Entries ---
    for u in units {
        let key = &u.key;
        let msgid = u.source.as_deref().unwrap_or("");

        // #: абсолютный (или полный) путь + строка — как было
        if let Some(line) = u.line {
            writeln!(w, "#: {}:{}", u.path.display(), line)?;
        } else {
            writeln!(w, "#: {}", u.path.display())?;
        }

        // msgctxt: делаем уникальным: "<key>|<relative_path>:<line?>"
        let path_str = u.path.to_string_lossy();
        let rel = rel_from_languages(&path_str).unwrap_or_else(|| {
            u.path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown.xml")
                .to_string()
        });
        let line_suffix = u.line.map(|l| format!(":{}", l)).unwrap_or_default();
        let ctx = format!("{}|{}{}", key, rel, line_suffix);

        writeln!(w, "msgctxt \"{}\"", escape_po(&ctx))?;
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
            line: Some(line),
        }
    }

    #[test]
    fn po_contains_header_context_and_references() {
        let tmp = NamedTempFile::new().unwrap();
        let units = vec![unit("Greeting", "Hello", 3), unit("Farewell", "Bye", 7)];
        write_po(tmp.path(), &units, Some("ru")).unwrap();

        let s = fs::read_to_string(tmp.path()).unwrap();
        // заголовок
        assert!(s.contains(r#""Language: ru\n""#));
        assert!(
            s.contains(&format!(
                r#""X-RimLoc-Schema: {}\n""#,
                rimloc_core::RIMLOC_SCHEMA_VERSION
            )),
            "PO header should include X-RimLoc-Schema"
        );
        // #: ссылка
        assert!(s.contains(r#"#: /Mod/Languages/English/Keyed/A.xml:3"#));
        // msgctxt с пайпом и относительным путём
        assert!(s.contains(r#"msgctxt "Greeting|Keyed/A.xml:3""#));
        // msgid
        assert!(s.contains(r#"msgid "Hello""#));
    }
}
