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
/// Stats for PO writing (useful when TM is applied)
#[derive(Debug, Clone, Copy, Default)]
pub struct PoStats {
    pub total: usize,
    pub tm_filled: usize,
}

/// Backward-compatible entry: write PO without TM
pub fn write_po(path: &Path, units: &[TransUnit], lang: Option<&str>) -> Result<()> {
    write_po_with_tm(path, units, lang, None).map(|_| ())
}

/// One project-exchange row: canonical key, source text, current
/// translation. The exchange PO is the round-trip format for external CAT
/// tools (Poedit et al.).
#[derive(Debug, Clone)]
pub struct PoExchangeRow {
    /// Canonical key — the import parser keeps the part of msgctxt
    /// before '|' so a bare key round-trips.
    pub key: String,
    /// Source-language text (msgid).
    pub source: String,
    /// Current project translation (msgstr).
    pub translation: String,
    /// Optional `#: reference` (`path` or `path:line`) — the PO import
    /// path rebuilder splits the pack back into files from it, so a
    /// RimLoc-exported PO re-imports losslessly.
    pub reference: Option<String>,
}

fn write_po_header(w: &mut impl Write, lang: Option<&str>) -> Result<()> {
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
    Ok(())
}

/// Write PO with optional translation memory (key -> translation).
/// When TM hit is found, we prefill msgstr and mark entry as fuzzy.
pub fn write_po_with_tm(
    path: &Path,
    units: &[TransUnit],
    lang: Option<&str>,
    tm: Option<&std::collections::HashMap<String, String>>,
) -> Result<PoStats> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // --- Header ---
    write_po_header(&mut w, lang)?;

    let mut stats = PoStats::default();

    // --- Entries ---
    for u in units {
        stats.total += 1;
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

        // If TM provided and has value for this key, mark fuzzy and prefill msgstr
        let tm_val = tm.and_then(|m| m.get(key));
        if let Some(val) = tm_val {
            writeln!(w, "#, fuzzy")?;
            stats.tm_filled += 1;
            writeln!(w, "msgctxt \"{}\"", escape_po(&ctx))?;
            writeln!(w, "msgid \"{}\"", escape_po(msgid))?;
            writeln!(w, "msgstr \"{}\"", escape_po(val))?;
        } else {
            writeln!(w, "msgctxt \"{}\"", escape_po(&ctx))?;
            writeln!(w, "msgid \"{}\"", escape_po(msgid))?;
            writeln!(w, "msgstr \"\"")?;
        }
        writeln!(w)?;
    }

    w.flush()?;
    Ok(stats)
}

/// Write the PROJECT EXCHANGE PO: msgstr carries the current translations
/// (never fuzzy — they are the durable project state), msgid carries the
/// source text. Pairs with `rimloc-import-po`'s tree rebuilder: when
/// `reference` is set, re-import splits the rows back into the canonical
/// file layout, so export → external tool → import round-trips.
pub fn write_po_translated(
    path: &Path,
    lang: Option<&str>,
    rows: &[PoExchangeRow],
) -> Result<PoStats> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    write_po_header(&mut w, lang)?;
    let mut stats = PoStats::default();
    for row in rows {
        stats.total += 1;
        let translated = !row.translation.trim().is_empty();
        if translated {
            stats.tm_filled += 1;
        }
        if let Some(reference) = &row.reference {
            writeln!(w, "#: {}", escape_po(reference))?;
        }
        writeln!(w, "msgctxt \"{}\"", escape_po(&row.key))?;
        writeln!(w, "msgid \"{}\"", escape_po(&row.source))?;
        writeln!(w, "msgstr \"{}\"", escape_po(&row.translation))?;
        writeln!(w)?;
    }
    w.flush()?;
    Ok(stats)
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
            tkey: None,
            ..Default::default()
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

    #[test]
    fn po_msgid_keeps_xml_markup_verbatim() {
        // MUST_FIX_BEFORE_BETA: разметка из Keyed (`&lt;b&gt;...&lt;/b&gt;` в
        // XML, `<b>...</b>` в распарсенном source) попадает в msgid как есть:
        // PO не требует экранирования угловых скобок, а двойной unescape
        // (и потеря тегов `bvalue/b`) недопустимы.
        let tmp = NamedTempFile::new().unwrap();
        let units = vec![unit(
            "HugsLib_loadOrderWarning_text",
            "<b>The HugsLib mod</b> should always be loaded after <b>Core</b>",
            79,
        )];
        write_po(tmp.path(), &units, Some("ru")).unwrap();

        let s = fs::read_to_string(tmp.path()).unwrap();
        assert!(
            s.contains(
                r#"msgid "<b>The HugsLib mod</b> should always be loaded after <b>Core</b>""#
            ),
            "msgid must keep parsed markup verbatim, got: {s}"
        );
        assert!(
            !s.contains("bvalue/b"),
            "stripped-tag corruption must not return"
        );
        assert!(
            !s.contains("&lt;") && !s.contains("&amp;"),
            "msgid must not double-escape entities"
        );
    }

    #[test]
    fn exchange_po_round_trips_through_the_import_parser() {
        let tmp = NamedTempFile::new().unwrap();
        let rows = vec![
            PoExchangeRow {
                key: "Greeting".into(),
                source: "Hello".into(),
                translation: "Привет".into(),
                reference: Some("/tmp/export/Languages/ru/Keyed/A.xml:2".into()),
            },
            PoExchangeRow {
                key: "Farewell".into(),
                source: "Bye".into(),
                translation: String::new(),
                reference: None,
            },
        ];
        let stats = write_po_translated(tmp.path(), Some("ru"), &rows).unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.tm_filled, 1, "one translated row");

        let entries = rimloc_import_po::read_po_entries(tmp.path()).unwrap();
        assert_eq!(entries.len(), 2, "{entries:?}");
        let greeting = entries.iter().find(|e| e.key == "Greeting").unwrap();
        assert_eq!(greeting.value, "Привет");
        assert!(
            greeting
                .reference
                .as_deref()
                .unwrap_or("")
                .contains("Keyed/A.xml:2"),
            "reference must survive for the tree rebuilder: {greeting:?}"
        );
        let farewell = entries.iter().find(|e| e.key == "Farewell").unwrap();
        assert_eq!(farewell.value, "", "untranslated row stays empty");
    }
}
