use color_eyre::eyre::{eyre, Result};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Writer;
use regex::Regex;
use rimloc_core::PoEntry;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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
            let raw = parse_po_string(lt.strip_prefix("msgctxt").unwrap_or(""))?;
            // Если msgctxt имеет вид "Key|Path:Line", оставляем только "Key"
            let key = raw
                .split_once('|')
                .map(|(k, _)| k.to_string())
                .unwrap_or(raw);
            cur_ctxt = Some(key);
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
                out.push(PoEntry {
                    key: k.clone(),
                    value: v.clone(),
                    reference: cur_ref.clone(),
                });
            }

            cur_ref = None;
            cur_ctxt = None;
            cur_str = None;
            in_msgstr = false;
        }
    }

    if let (Some(k), Some(v)) = (cur_ctxt, cur_str) {
        out.push(PoEntry {
            key: k,
            value: v,
            reference: cur_ref,
        });
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

/// Group PO entries by relative RimWorld path (e.g., Keyed/_Imported.xml)
fn group_entries_by_rel_path(
    entries: Vec<PoEntry>,
) -> std::collections::HashMap<std::path::PathBuf, Vec<(String, String)>> {
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    static REL_PATH_RE: OnceLock<Regex> = OnceLock::new();
    let re = REL_PATH_RE.get_or_init(|| {
        Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](.+?)(?::\d+)?$").unwrap()
    });

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
        grouped
            .entry(rel_subpath)
            .or_default()
            .push((e.key, e.value));
    }

    grouped
}

fn dedupe_last_wins(items: &[(String, String)]) -> Vec<(String, String)> {
    use std::collections::HashSet;
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in items.iter().rev() {
        if seen.insert(k.as_str()) {
            out.push((k.clone(), v.clone()));
        }
    }
    out.reverse();
    out
}

/// Сгенерировать LanguageData XML из пар <key, value>.
pub fn write_language_data_xml(out_path: &Path, entries: &[(String, String)]) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(out_path)?;
    let mut w = Writer::new_with_indent(BufWriter::new(file), b' ', 2);

    w.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
        "1.0",
        Some("UTF-8"),
        None,
    )))?;

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
        "English",
        "Russian",
        "Japanese",
        "Korean",
        "French",
        "German",
        "Spanish",
        "SpanishLatin",
        "Portuguese",
        "PortugueseBrazilian",
        "Polish",
        "Italian",
        "Turkish",
        "Ukrainian",
        "Czech",
        "Hungarian",
        "Dutch",
        "Romanian",
        "Thai",
        "Greek",
        "ChineseSimplified",
        "ChineseTraditional",
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

fn write_about_xml(
    about_xml: &std::path::Path,
    package_id: &str,
    mod_name: &str,
    rw_version: &str,
) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut f = File::create(about_xml)?;
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
    Ok(())
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
    let grouped = group_entries_by_rel_path(entries);

    // 3) About/About.xml
    let about_dir = out_mod.join("About");
    fs::create_dir_all(&about_dir)?;
    let about_xml = about_dir.join("About.xml");
    write_about_xml(&about_xml, package_id, mod_name, rw_version)?;

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
    let grouped = group_entries_by_rel_path(entries);

    // 3) About/About.xml
    let about_dir = out_mod.join("About");
    fs::create_dir_all(&about_dir)?;
    let about_xml = about_dir.join("About.xml");
    write_about_xml(&about_xml, package_id, mod_name, rw_version)?;

    // 4) записываем файлы в указанную папку lang_dir
    for (rel, items) in grouped {
        let out_path = out_mod.join("Languages").join(lang_dir).join(rel);
        write_language_data_xml(&out_path, &items)?;
    }

    Ok(())
}

/// Build translation mod with options
pub fn build_translation_mod_with_langdir_opts(
    po_path: &Path,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<()> {
    let entries = read_po_entries(po_path)?;
    let mut grouped = group_entries_by_rel_path(entries);

    let about_dir = out_mod.join("About");
    fs::create_dir_all(&about_dir)?;
    let about_xml = about_dir.join("About.xml");
    write_about_xml(&about_xml, package_id, mod_name, rw_version)?;

    for (rel, mut items) in grouped.drain() {
        if dedupe {
            items = dedupe_last_wins(&items);
        }
        let out_path = out_mod.join("Languages").join(lang_dir).join(rel);
        write_language_data_xml(&out_path, &items)?;
    }

    Ok(())
}

/// A dry-run plan describing what would be written during build-mod
#[derive(Debug, Clone)]
pub struct DryRunPlan {
    pub mod_name: String,
    pub package_id: String,
    pub rw_version: String,
    pub out_mod: PathBuf,
    pub lang_dir: String,
    pub files: Vec<(PathBuf, usize)>,
    pub total_keys: usize,
}

/// Сухой прогон сборки мода перевода:
/// показывает, какие файлы будут созданы, и сколько ключей в каждом.
/// Ничего не записывает на диск.
pub fn build_translation_mod_dry_run(
    po_path: &Path,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<DryRunPlan> {
    let entries = read_po_entries(po_path)?;

    let grouped = group_entries_by_rel_path(entries);

    let mut total_keys = 0usize;
    let mut files = Vec::new();
    let mut paths: Vec<_> = grouped.keys().cloned().collect();
    paths.sort();
    for rel in paths {
        let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
        total_keys += n;
        let full_path = out_mod.join("Languages").join(lang_dir).join(&rel);
        files.push((full_path, n));
    }

    Ok(DryRunPlan {
        mod_name: mod_name.to_string(),
        package_id: package_id.to_string(),
        rw_version: rw_version.to_string(),
        out_mod: out_mod.to_path_buf(),
        lang_dir: lang_dir.to_string(),
        files,
        total_keys,
    })
}

/// Dry-run variant with options
pub fn build_translation_mod_dry_run_opts(
    po_path: &Path,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<DryRunPlan> {
    let entries = read_po_entries(po_path)?;
    let grouped = group_entries_by_rel_path(entries);

    let mut total_keys = 0usize;
    let mut files = Vec::new();
    let mut paths: Vec<_> = grouped.keys().cloned().collect();
    paths.sort();
    for rel in paths {
        let mut n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
        if dedupe {
            if let Some(items) = grouped.get(&rel) {
                n = dedupe_last_wins(items).len();
            }
        }
        total_keys += n;
        let full_path = out_mod.join("Languages").join(lang_dir).join(&rel);
        files.push((full_path, n));
    }

    Ok(DryRunPlan {
        mod_name: mod_name.to_string(),
        package_id: package_id.to_string(),
        rw_version: rw_version.to_string(),
        out_mod: out_mod.to_path_buf(),
        lang_dir: lang_dir.to_string(),
        files,
        total_keys,
    })
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
        assert_eq!(
            super::parse_po_string(r#""a\"b\\c\n\t\r""#).unwrap(),
            "a\"b\\c\n\t\r"
        );
    }

    #[test]
    fn read_po_entries_parses_reference_ctxt_and_str() {
        // создаём временный .po с одной записью
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, r#"#: /Mods/My/Stuff/Languages/English/Keyed/A.xml:3"#).unwrap();
        writeln!(tmp, r#"msgctxt "Greeting""#).unwrap();
        writeln!(tmp, r#"msgstr "Привет""#).unwrap();
        writeln!(tmp).unwrap(); // пустая строка завершает запись

        let entries = read_po_entries(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "Greeting");
        assert_eq!(entries[0].value, "Привет");
        assert!(entries[0]
            .reference
            .as_ref()
            .unwrap()
            .contains("Languages/English/Keyed/A.xml:3"));
    }
}
