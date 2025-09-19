use color_eyre::eyre::{Result, eyre};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Writer;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PoEntry {
    pub key: String,                 // msgctxt
    pub value: String,               // msgstr
    pub reference: Option<String>,   // строка после "#: " (может быть с :line)
}

/// Очень простой парсер .po: собираем reference + msgctxt + msgstr.
/// Поддерживает многострочный msgstr. Заголовки/комментарии кроме "#:" игнорим.
pub fn read_po_entries(po_path: &Path) -> Result<Vec<PoEntry>> {
    let file = File::open(po_path)?;
    let reader = BufReader::new(file);

    let mut out = Vec::new();
    let mut cur_ref: Option<String> = None;
    let mut cur_ctxt: Option<String> = None;
    let mut cur_str: Option<String> = None;
    let mut in_msgstr = false;

    for line in reader.lines() {
        let l = line?;
        let lt = l.trim();

        if lt.starts_with("#:") {
            // Берём первую ссылку в блоке (хватает для восстановления пути)
            if cur_ref.is_none() {
                let r = lt.trim_start_matches("#:").trim().to_string();
                cur_ref = Some(r);
            }
            continue;
        }

        if lt.starts_with("msgctxt") {
            cur_ctxt = Some(parse_po_string(lt.strip_prefix("msgctxt").unwrap_or(""))?);
            in_msgstr = false;
            continue;
        }

        if lt.starts_with("msgstr") {
            cur_str = Some(parse_po_string(lt.strip_prefix("msgstr").unwrap_or(""))?);
            in_msgstr = true;
            continue;
        }

        if lt.starts_with('"') && in_msgstr {
            let val = parse_po_string(lt)?;
            if let Some(ref mut s) = cur_str {
                s.push_str(&val);
            }
            continue;
        }

        if lt.is_empty() {
            // Конец блока
            if let (Some(k), Some(v)) = (&cur_ctxt, &cur_str) {
                if !v.trim().is_empty() {
                    out.push(PoEntry {
                        key: k.clone(),
                        value: v.clone(),
                        reference: cur_ref.clone(),
                    });
                }
            }
            cur_ref = None;
            cur_ctxt = None;
            cur_str = None;
            in_msgstr = false;
        }
    }

    // Хвост
    if let (Some(k), Some(v)) = (cur_ctxt, cur_str) {
        if !v.trim().is_empty() {
            out.push(PoEntry {
                key: k,
                value: v,
                reference: cur_ref,
            });
        }
    }

    Ok(out)
}

fn parse_po_string(s: &str) -> Result<String> {
    let s = s.trim();
    if !s.starts_with('"') || !s.ends_with('"') {
        return Err(eyre!("invalid po string: {s}"));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    other => out.push(other),
                }
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

/// Сгенерировать LanguageData XML из пар <key, value>.
pub fn write_language_data_xml(out_path: &Path, entries: &[(String, String)]) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(out_path)?;
    let mut w = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

    w.write_event(Event::Decl(
        quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None),
    ))?;

    let start = BytesStart::new("LanguageData");
    w.write_event(Event::Start(start))?;

    for (key, value) in entries {
        let tag = BytesStart::new(key.as_str());
        w.write_event(Event::Start(tag))?;
        w.write_event(Event::Text(quick_xml::events::BytesText::new(value)))?;
        w.write_event(Event::End(BytesEnd::new(key.as_str())))?;
    }

    w.write_event(Event::End(BytesEnd::new("LanguageData")))?;
    w.into_inner().flush()?;
    Ok(())
}

/// Построить мод-перевод:
/// - создаёт About/About.xml
/// - раскладывает записи по файлам в Languages/<lang>/<subpath>
///   где <subpath> берётся из reference "#: .../Languages/<srcLang>/<subpath>:line"
pub fn build_translation_mod(po_path: &Path, out_mod: &Path, lang: &str, mod_name: &str, package_id: &str, rw_version: &str) -> Result<()> {
    // 1) читаем po
    let entries = read_po_entries(po_path)?;

    // 2) группируем по относительным путям
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    let re = Regex::new(r"/Languages/([^/]+)/(.+?)(?::\d+)?$").unwrap();

    for e in entries {
        let rel_subpath: PathBuf = if let Some(r) = &e.reference {
            if let Some(caps) = re.captures(r) {
                // caps[1] = исходный язык (можно игнорировать), caps[2] = относительный путь внутри Languages/<srcLang>/
                PathBuf::from(&caps[2])
            } else {
                // нет совпадения — шлём в Keyed/_Imported.xml
                PathBuf::from("Keyed/_Imported.xml")
            }
        } else {
            PathBuf::from("Keyed/_Imported.xml")
        };

        grouped.entry(rel_subpath).or_default().push((e.key, e.value));
    }

    // 3) создаём About/About.xml
    let about_dir = out_mod.join("About");
    fs::create_dir_all(&about_dir)?;
    let about_xml = about_dir.join("About.xml");
    let mut f = File::create(&about_xml)?;
    write!(
        f,
        r#"<ModMetaData>
  <packageId>{}</packageId>
  <name>{}</name>
  <description>Auto-generated translation mod</description>
  <supportedVersions>
    <li>{}</li>
  </supportedVersions>
</ModMetaData>
"#,
        package_id, mod_name, rw_version
    )?;

    // 4) пишем файлы в Languages/<lang>/...
    for (rel, items) in grouped {
        let out_path = out_mod.join("Languages").join(lang).join(rel);
        write_language_data_xml(&out_path, &items)?;
    }

    Ok(())
}
