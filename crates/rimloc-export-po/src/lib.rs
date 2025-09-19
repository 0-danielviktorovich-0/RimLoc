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
