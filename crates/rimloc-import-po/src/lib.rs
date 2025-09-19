use color_eyre::eyre::{Result, eyre};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Writer;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PoEntry {
    pub key: String,               // msgctxt
    pub value: String,             // msgstr
    pub reference: Option<String>, // строка после "#: " (может быть с :line)
}

/// Очень простой парсер .po: собираем reference + msgctxt + msgstr.
/// Поддерживает многострочный msgstr. Заголовки/комментарии (кроме "#:") игнорим.
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

    w.write_event(Event::Start(BytesStart::new("LanguageData")))?;

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

/// Преобразовать ISO-коды/алиасы в имя папки RimWorld (Languages/<DirName>).
pub fn rimworld_lang_dir(lang: &str) -> String {
    let l = lang.trim().to_lowercase().replace('_', "-");

    let known_names = [
        "English","Russian","Japanese","Korean","French","German","Spanish",
        "SpanishLatin","Portuguese","PortugueseBrazilian","Polish","Italian",
        "Turkish","Ukrainian","Czech","Hungarian","Dutch","Romanian","Thai",
        "Greek","ChineseSimplified","ChineseTraditional",
    ];
    if let Some(&n) = known_names.iter().find(|&&n| n.eq_ignore_ascii_case(lang)) {
        return n.to_string();
    }

    match l.as_str() {
        "en" | "en-us" | "en-gb" => "English".into(),
        "ru" | "ru-ru" => "Russian".into(),
        "ja" | "ja-jp" => "Japanese".into(),
        "ko" | "ko-kr" => "Korean".into(),
        "fr" | "fr-fr" | "fr-ca" => "French".into(),
        "de" | "de-de" => "German".into(),
        "es" | "es-es" => "Spanish".into(),
        "es-419" | "es-mx" | "es-ar" | "es-cl" | "es-co" | "es-pe" => "SpanishLatin".into(),
        "pt" | "pt-pt" => "Portuguese".into(),
        "pt-br" => "PortugueseBrazilian".into(),
        "pl" | "pl-pl" => "Polish".into(),
        "it" | "it-it" => "Italian".into(),
        "tr" | "tr-tr" => "Turkish".into(),
        "uk" | "uk-ua" => "Ukrainian".into(),
        "cs" | "cs-cz" => "Czech".into(),
        "hu" | "hu-hu" => "Hungarian".into(),
        "nl" | "nl-nl" => "Dutch".into(),
        "ro" | "ro-ro" => "Romanian".into(),
        "th" | "th-th" => "Thai".into(),
        "el" | "el-gr" => "Greek".into(),
        "zh" | "zh-cn" | "zh-sg" | "zh-hans" => "ChineseSimplified".into(),
        "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant" => "ChineseTraditional".into(),
        _ => {
            // fallback: "pt-br" -> "PtBr"
            let mut s = String::new();
            let mut upper_next = true;
            for ch in l.chars() {
                if ch == '-' {
                    upper_next = true;
                    continue;
                }
                if upper_next {
                    s.push(ch.to_ascii_uppercase());
                    upper_next = false;
                } else {
                    s.push(ch);
                }
            }
            s
        }
    }
}

/// Собрать мод-перевод (авто-выбор папки языка по --lang).
pub fn build_translation_mod(
    po_path: &Path,
    out_mod: &Path,
    lang: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<()> {
    // 1) читаем po
    let entries = read_po_entries(po_path)?;

    // 2) группируем по относительным путям
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    let re = Regex::new(r"/Languages/([^/]+)/(.+?)(?::\d+)?$").unwrap();

    for e in entries {
        let rel_subpath: PathBuf = if let Some(r) = &e.reference {
            if let Some(caps) = re.captures(r) {
                PathBuf::from(&caps[2])
            } else {
                PathBuf::from("Keyed/_Imported.xml")
            }
        } else {
            PathBuf::from("Keyed/_Imported.xml")
        };
        grouped.entry(rel_subpath).or_default().push((e.key, e.value));
    }

    // 3) About/About.xml
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

    // 4) имя папки языка
    let lang_dir = rimworld_lang_dir(lang);

    // 5) записываем файлы
    for (rel, items) in grouped {
        let out_path = out_mod.join("Languages").join(&lang_dir).join(rel);
        write_language_data_xml(&out_path, &items)?;
    }

    Ok(())
}

/// То же, что build_translation_mod, но принимает ГОТОВОЕ имя папки языка (например, "Russian").
pub fn build_translation_mod_with_langdir(
    po_path: &Path,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<()> {
    // 1) читаем po
    let entries = read_po_entries(po_path)?;

    // 2) группируем по относительным путям
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    let re = Regex::new(r"/Languages/([^/]+)/(.+?)(?::\d+)?$").unwrap();

    for e in entries {
        let rel_subpath: PathBuf = if let Some(r) = &e.reference {
            if let Some(caps) = re.captures(r) {
                PathBuf::from(&caps[2])
            } else {
                PathBuf::from("Keyed/_Imported.xml")
            }
        } else {
            PathBuf::from("Keyed/_Imported.xml")
        };
        grouped.entry(rel_subpath).or_default().push((e.key, e.value));
    }

    // 3) About/About.xml
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

    // 4) записываем файлы в указанную папку lang_dir
    for (rel, items) in grouped {
        let out_path = out_mod.join("Languages").join(lang_dir).join(rel);
        write_language_data_xml(&out_path, &items)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn maps_iso_codes_to_rimworld_dirs() {
        assert_eq!(rimworld_lang_dir("ru"), "Russian");
        assert_eq!(rimworld_lang_dir("ja"), "Japanese");
        assert_eq!(rimworld_lang_dir("en"), "English");
        assert_eq!(rimworld_lang_dir("pt-br"), "PortugueseBrazilian");
        assert_eq!(rimworld_lang_dir("zh-hant"), "ChineseTraditional");
        // уже каноническое имя
        assert_eq!(rimworld_lang_dir("Russian"), "Russian");
        // fallback для неизвестных кодов
        assert_eq!(rimworld_lang_dir("xx"), "Xx");
        assert_eq!(rimworld_lang_dir("pt-ao"), "PtAo");
    }

    #[test]
    fn parse_po_string_unescapes_sequences() {
        assert_eq!(super::parse_po_string(r#""a\"b\\c\n\t\r""#).unwrap(), "a\"b\\c\n\t\r");
    }

    #[test]
    fn read_po_entries_parses_reference_ctxt_and_str() {
        // создаём временный .po с одной записью
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, r#"#: /Mods/My/Stuff/Languages/English/Keyed/A.xml:3"#).unwrap();
        writeln!(tmp, r#"msgctxt "Greeting""#).unwrap();
        writeln!(tmp, r#"msgstr "Привет""#).unwrap();
        writeln!(tmp, "").unwrap(); // пустая строка завершает запись

        let entries = read_po_entries(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "Greeting");
        assert_eq!(entries[0].value, "Привет");
        assert!(entries[0].reference
            .as_ref()
            .unwrap()
            .contains("Languages/English/Keyed/A.xml:3"));
    }
}
