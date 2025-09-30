use crate::{
    scan::{scan_units_with_defs, scan_units_with_defs_and_dict, scan_units_with_defs_and_fields},
    util::{is_source_for_lang_dir, is_under_languages_dir},
    Result,
};
use rimloc_domain::DiffOutput;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// Compute presence/changed diffs between source and target language data.
pub fn diff_xml(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    baseline_po: Option<&Path>,
) -> Result<DiffOutput> {
    // In this flavor, we scan all Defs; a wrapper can restrict Defs via another function.
    let units = scan_units_with_defs(root, None)?;

    let mut src_map: HashMap<String, String> = HashMap::new();
    let mut trg_keys: BTreeSet<String> = BTreeSet::new();
    for u in &units {
        if is_source_for_lang_dir(&u.path, source_lang_dir) {
            if let Some(val) = u.source.as_deref() {
                src_map
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, target_lang_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    let mut only_in_src: Vec<String> = Vec::new();
    let mut only_in_trg: Vec<String> = Vec::new();
    for k in src_map.keys() {
        if !trg_keys.contains(k) {
            only_in_src.push(k.clone());
        }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) {
            only_in_trg.push(k.clone());
        }
    }
    only_in_src.sort();
    only_in_trg.sort();

    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po {
        // Parse PO header+entries with msgctxt support to extract original key from context
        let file = std::fs::File::open(po)?;
        use std::io::{BufRead, BufReader};
        let rdr = BufReader::new(file);
        let mut base: HashMap<String, String> = HashMap::new();
        let mut ctx: Option<String> = None;
        let mut id = String::new();
        let mut strv = String::new();
        enum Mode {
            None,
            InId,
            InStr,
        }
        let mut mode = Mode::None;
        fn unq(s: &str) -> String {
            let mut out = String::new();
            let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
            let mut it = raw.chars().peekable();
            while let Some(c) = it.next() {
                if c == '\\' {
                    if let Some(n) = it.next() {
                        out.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            x => x,
                        });
                    } else {
                        out.push('\\');
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }
        let mut push = |ctx: &mut Option<String>, id: &mut String, strv: &mut String| {
            if !id.is_empty() {
                // ctx format: key|relpath[:line]? — take key before '|'
                if let Some(c) = ctx.as_deref() {
                    let key = c.split('|').next().unwrap_or("").trim().to_string();
                    if !key.is_empty() {
                        base.entry(key).or_insert(std::mem::take(id));
                    }
                }
                *ctx = None;
                *strv = String::new();
            }
        };
        for line in rdr.lines() {
            let t = line?.trim().to_string();
            if t.is_empty() {
                push(&mut ctx, &mut id, &mut strv);
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgctxt ") {
                push(&mut ctx, &mut id, &mut strv);
                ctx = Some(unq(rest));
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgid ") {
                push(&mut ctx, &mut id, &mut strv);
                id = unq(rest);
                mode = Mode::InId;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgstr ") {
                strv = unq(rest);
                mode = Mode::InStr;
                continue;
            }
            if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
                let chunk = unq(&t);
                match mode {
                    Mode::InId => id.push_str(&chunk),
                    Mode::InStr => strv.push_str(&chunk),
                    Mode::None => {}
                }
            }
        }
        push(&mut ctx, &mut id, &mut strv);
        for (k, new_src) in &src_map {
            if let Some(old_src) = base.get(k) {
                if old_src != new_src {
                    changed.push((k.clone(), new_src.clone()));
                }
            }
        }
        changed.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(DiffOutput {
        changed,
        only_in_translation: only_in_trg,
        only_in_mod: only_in_src,
    })
}

/// Same as `diff_xml`, but restricts Defs scanning to a specific directory when provided.
pub fn diff_xml_with_defs(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    baseline_po: Option<&Path>,
    defs_root: Option<&Path>,
) -> Result<DiffOutput> {
    let units = scan_units_with_defs(root, defs_root)?;

    let mut src_map: HashMap<String, String> = HashMap::new();
    let mut trg_keys: BTreeSet<String> = BTreeSet::new();
    for u in &units {
        if is_source_for_lang_dir(&u.path, source_lang_dir) {
            if let Some(val) = u.source.as_deref() {
                src_map
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, target_lang_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    let mut only_in_src: Vec<String> = Vec::new();
    let mut only_in_trg: Vec<String> = Vec::new();
    for k in src_map.keys() {
        if !trg_keys.contains(k) {
            only_in_src.push(k.clone());
        }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) {
            only_in_trg.push(k.clone());
        }
    }
    only_in_src.sort();
    only_in_trg.sort();

    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po {
        // Parse PO header+entries with msgctxt support to extract original key from context
        let file = std::fs::File::open(po)?;
        use std::io::{BufRead, BufReader};
        let rdr = BufReader::new(file);
        let mut base: HashMap<String, String> = HashMap::new();
        let mut ctx: Option<String> = None;
        let mut id = String::new();
        let mut strv = String::new();
        enum Mode {
            None,
            InId,
            InStr,
        }
        let mut mode = Mode::None;
        fn unq(s: &str) -> String {
            let mut out = String::new();
            let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
            let mut it = raw.chars().peekable();
            while let Some(c) = it.next() {
                if c == '\\' {
                    if let Some(n) = it.next() {
                        out.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            x => x,
                        });
                    } else {
                        out.push('\\');
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }
        let mut push = |ctx: &mut Option<String>, id: &mut String, strv: &mut String| {
            if !id.is_empty() {
                // ctx format: key|relpath[:line]? — take key before '|'
                if let Some(c) = ctx.as_deref() {
                    let key = c.split('|').next().unwrap_or("").trim().to_string();
                    if !key.is_empty() {
                        base.entry(key).or_insert(std::mem::take(id));
                    }
                }
                *ctx = None;
                *strv = String::new();
            }
        };
        for line in rdr.lines() {
            let t = line?.trim().to_string();
            if t.is_empty() {
                push(&mut ctx, &mut id, &mut strv);
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgctxt ") {
                push(&mut ctx, &mut id, &mut strv);
                ctx = Some(unq(rest));
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgid ") {
                push(&mut ctx, &mut id, &mut strv);
                id = unq(rest);
                mode = Mode::InId;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgstr ") {
                strv = unq(rest);
                mode = Mode::InStr;
                continue;
            }
            if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
                let chunk = unq(&t);
                match mode {
                    Mode::InId => id.push_str(&chunk),
                    Mode::InStr => strv.push_str(&chunk),
                    Mode::None => {}
                }
            }
        }
        push(&mut ctx, &mut id, &mut strv);
        for (k, new_src) in &src_map {
            if let Some(old_src) = base.get(k) {
                if old_src != new_src {
                    changed.push((k.clone(), new_src.clone()));
                }
            }
        }
        changed.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(DiffOutput {
        changed,
        only_in_translation: only_in_trg,
        only_in_mod: only_in_src,
    })
}

pub fn diff_xml_with_defs_and_fields(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    baseline_po: Option<&Path>,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> Result<DiffOutput> {
    let units = scan_units_with_defs_and_fields(root, defs_root, extra_fields)?;

    let mut src_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut trg_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for u in &units {
        if is_source_for_lang_dir(&u.path, source_lang_dir) {
            if let Some(val) = u.source.as_deref() {
                src_map
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, target_lang_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    let mut only_in_src: Vec<String> = Vec::new();
    let mut only_in_trg: Vec<String> = Vec::new();
    for k in src_map.keys() {
        if !trg_keys.contains(k) {
            only_in_src.push(k.clone());
        }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) {
            only_in_trg.push(k.clone());
        }
    }
    only_in_src.sort();
    only_in_trg.sort();

    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po {
        let file = std::fs::File::open(po)?;
        use std::io::{BufRead, BufReader};
        let rdr = BufReader::new(file);
        let mut base: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut ctx: Option<String> = None;
        let mut id = String::new();
        let mut strv = String::new();
        enum Mode {
            None,
            InId,
            InStr,
        }
        let mut mode = Mode::None;
        fn unq(s: &str) -> String {
            let mut out = String::new();
            let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
            let mut it = raw.chars().peekable();
            while let Some(c) = it.next() {
                if c == '\\' {
                    if let Some(n) = it.next() {
                        out.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            x => x,
                        });
                    } else {
                        out.push('\\');
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }
        let mut push = |ctx: &mut Option<String>, id: &mut String, strv: &mut String| {
            if !id.is_empty() {
                if let Some(c) = ctx.as_deref() {
                    let key = c.split('|').next().unwrap_or("").trim().to_string();
                    if !key.is_empty() {
                        base.entry(key).or_insert(std::mem::take(id));
                    }
                }
                *ctx = None;
                *strv = String::new();
            }
        };
        for line in rdr.lines() {
            let t = line?.trim().to_string();
            if t.is_empty() {
                push(&mut ctx, &mut id, &mut strv);
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgctxt ") {
                push(&mut ctx, &mut id, &mut strv);
                ctx = Some(unq(rest));
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgid ") {
                push(&mut ctx, &mut id, &mut strv);
                id = unq(rest);
                mode = Mode::InId;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgstr ") {
                strv = unq(rest);
                mode = Mode::InStr;
                continue;
            }
            if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
                let chunk = unq(&t);
                match mode {
                    Mode::InId => id.push_str(&chunk),
                    Mode::InStr => strv.push_str(&chunk),
                    Mode::None => {}
                }
            }
        }
        push(&mut ctx, &mut id, &mut strv);
        for (k, new_src) in &src_map {
            if let Some(old_src) = base.get(k) {
                if old_src != new_src {
                    changed.push((k.clone(), new_src.clone()));
                }
            }
        }
        changed.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(DiffOutput {
        changed,
        only_in_translation: only_in_trg,
        only_in_mod: only_in_src,
    })
}

/// Same as `diff_xml_with_defs_and_fields`, but allows providing a dictionary of
/// DefType -> [field paths]. The scan will use the dictionary plus extra fields.
pub fn diff_xml_with_defs_and_dict(
    root: &Path,
    source_lang_dir: &str,
    target_lang_dir: &str,
    baseline_po: Option<&Path>,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> Result<DiffOutput> {
    let units = scan_units_with_defs_and_dict(root, defs_root, dict, extra_fields)?;

    let mut src_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut trg_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for u in &units {
        if is_source_for_lang_dir(&u.path, source_lang_dir) {
            if let Some(val) = u.source.as_deref() {
                src_map
                    .entry(u.key.clone())
                    .or_insert_with(|| val.to_string());
            }
        } else if is_under_languages_dir(&u.path, target_lang_dir) {
            trg_keys.insert(u.key.clone());
        }
    }

    let mut only_in_src: Vec<String> = Vec::new();
    let mut only_in_trg: Vec<String> = Vec::new();
    for k in src_map.keys() {
        if !trg_keys.contains(k) {
            only_in_src.push(k.clone());
        }
    }
    for k in trg_keys.iter() {
        if !src_map.contains_key(k) {
            only_in_trg.push(k.clone());
        }
    }
    only_in_src.sort();
    only_in_trg.sort();

    let mut changed: Vec<(String, String)> = Vec::new();
    if let Some(po) = baseline_po {
        let file = std::fs::File::open(po)?;
        use std::io::{BufRead, BufReader};
        let rdr = BufReader::new(file);
        let mut base: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut ctx: Option<String> = None;
        let mut id = String::new();
        let mut strv = String::new();
        enum Mode {
            None,
            InId,
            InStr,
        }
        let mut mode = Mode::None;
        fn unq(s: &str) -> String {
            let mut out = String::new();
            let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
            let mut it = raw.chars().peekable();
            while let Some(c) = it.next() {
                if c == '\\' {
                    if let Some(n) = it.next() {
                        out.push(match n {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            x => x,
                        });
                    } else {
                        out.push('\\');
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }
        let mut push = |ctx: &mut Option<String>, id: &mut String, strv: &mut String| {
            if !id.is_empty() {
                if let Some(c) = ctx.as_deref() {
                    let key = c.split('|').next().unwrap_or("").trim().to_string();
                    if !key.is_empty() {
                        base.entry(key).or_insert(std::mem::take(id));
                    }
                }
                *ctx = None;
                *strv = String::new();
            }
        };
        for line in rdr.lines() {
            let t = line?.trim().to_string();
            if t.is_empty() {
                push(&mut ctx, &mut id, &mut strv);
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgctxt ") {
                push(&mut ctx, &mut id, &mut strv);
                ctx = Some(unq(rest));
                mode = Mode::None;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgid ") {
                push(&mut ctx, &mut id, &mut strv);
                id = unq(rest);
                mode = Mode::InId;
                continue;
            }
            if let Some(rest) = t.strip_prefix("msgstr ") {
                strv = unq(rest);
                mode = Mode::InStr;
                continue;
            }
            if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
                let chunk = unq(&t);
                match mode {
                    Mode::InId => id.push_str(&chunk),
                    Mode::InStr => strv.push_str(&chunk),
                    Mode::None => {}
                }
            }
        }
        push(&mut ctx, &mut id, &mut strv);
        for (k, new_src) in &src_map {
            if let Some(old_src) = base.get(k) {
                if old_src != new_src {
                    changed.push((k.clone(), new_src.clone()));
                }
            }
        }
        changed.sort_by(|a, b| a.0.cmp(&b.0));
    }

    Ok(DiffOutput {
        changed,
        only_in_translation: only_in_trg,
        only_in_mod: only_in_src,
    })
}

pub fn write_diff_reports(dir: &Path, diff: &DiffOutput) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    // ChangedData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("ChangedData.txt"))?;
        for (k, v) in &diff.changed {
            writeln!(f, "{}\t{}", k, v)?;
        }
    }
    // TranslationData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("TranslationData.txt"))?;
        for k in &diff.only_in_translation {
            writeln!(f, "{}", k)?;
        }
    }
    // ModData.txt
    {
        use std::io::Write;
        let mut f = std::fs::File::create(dir.join("ModData.txt"))?;
        for k in &diff.only_in_mod {
            writeln!(f, "{}", k)?;
        }
    }
    Ok(())
}

/// Apply flags to translation XML files based on a DiffOutput:
/// - keys listed in `changed` are annotated with `<!-- FUZZY -->`
/// - keys listed in `only_in_translation` are annotated with `<!-- UNUSED -->`
pub fn apply_diff_flags(root: &Path, target_lang_dir: &str, diff: &DiffOutput, backup: bool) -> Result<(usize, usize)> {
    use quick_xml::{events::{Event, BytesText}, Reader, Writer};
    use walkdir::WalkDir;
    use std::collections::HashSet;

    let fuzzy: HashSet<&str> = diff.changed.iter().map(|(k, _)| k.as_str()).collect();
    let unused: HashSet<&str> = diff.only_in_translation.iter().map(|k| k.as_str()).collect();

    let mut fuzzy_count = 0usize;
    let mut unused_count = 0usize;

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|e| e.to_str()).map_or(true, |ext| !ext.eq_ignore_ascii_case("xml")) { continue; }
        // Under Languages/<target>/Keyed or DefInjected
        let s = p.to_string_lossy();
        if !(s.contains(&format!("/Languages/{target_lang}/", target_lang = target_lang_dir)) || s.contains(&format!("\\Languages\\{target_lang}\\", target_lang = target_lang_dir))) { continue; }

        let input = match std::fs::read_to_string(p) { Ok(s) => s, Err(_) => continue };
        let mut reader = Reader::from_str(&input);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut out = Writer::new_with_indent(Vec::new(), b' ', 2);
        let mut stack: Vec<String> = Vec::new();
        let mut in_language_data = false;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if name.eq_ignore_ascii_case("LanguageData") { in_language_data = true; }
                    stack.push(name.clone());
                    if in_language_data && stack.len() == 2 {
                        if fuzzy.contains(name.as_str()) {
                            out.write_event(Event::Comment(BytesText::new(" FUZZY ")))?;
                            fuzzy_count += 1;
                        } else if unused.contains(name.as_str()) {
                            out.write_event(Event::Comment(BytesText::new(" UNUSED ")))?;
                            unused_count += 1;
                        }
                    }
                    out.write_event(Event::Start(e.to_owned()))?;
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    if in_language_data && stack.len() == 1 {
                        if fuzzy.contains(name.as_str()) {
                            out.write_event(Event::Comment(BytesText::new(" FUZZY ")))?;
                            fuzzy_count += 1;
                        } else if unused.contains(name.as_str()) {
                            out.write_event(Event::Comment(BytesText::new(" UNUSED ")))?;
                            unused_count += 1;
                        }
                    }
                    out.write_event(Event::Empty(e.to_owned()))?;
                }
                Ok(Event::End(e)) => {
                    if stack.pop().as_deref() == Some("LanguageData") { in_language_data = false; }
                    out.write_event(Event::End(e.to_owned()))?;
                }
                Ok(Event::Text(t)) => out.write_event(Event::Text(t))?,
                Ok(Event::CData(t)) => out.write_event(Event::CData(t))?,
                Ok(Event::Decl(d)) => out.write_event(Event::Decl(d))?,
                Ok(Event::PI(pi)) => out.write_event(Event::PI(pi))?,
                Ok(Event::Comment(c)) => out.write_event(Event::Comment(c))?,
                Ok(Event::DocType(d)) => out.write_event(Event::DocType(d))?,
                Ok(Event::Eof) => break,
                Err(_) => break,
            }
            buf.clear();
        }
        if backup && p.exists() {
            let _ = std::fs::copy(p, p.with_extension("xml.bak"));
        }
        std::fs::write(p, out.into_inner())?;
    }

    Ok((fuzzy_count, unused_count))
}
