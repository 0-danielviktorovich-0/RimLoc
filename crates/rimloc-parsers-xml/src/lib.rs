pub use rimloc_core::parse_simple_po as parse_po_string;

use quick_xml::events::Event;
use quick_xml::Reader;
use rimloc_core::{Result as CoreResult, TransUnit};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;

/// Very small XML scanner for RimWorld "Keyed" XML files.
/// It walks `root` and finds files that match `*/Languages/*/Keyed/*.xml`.
/// For every `<Key>Value</Key>` pair found, it produces a `TransUnit` with
/// the key name, text value, file path and (approximate) line number.
pub fn scan_keyed_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<TransUnit> = Vec::new();

    fn line_for_offset(offset: usize, starts: &[usize]) -> Option<usize> {
        if starts.is_empty() {
            return None;
        }
        match starts.binary_search(&offset) {
            Ok(idx) => Some(idx + 1),
            Err(idx) if idx > 0 => Some(idx),
            _ => Some(1),
        }
    }

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Languages/<Locale>/{Keyed,DefInjected}/....xml
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Languages/") || p_str.contains("\\Languages\\")) {
            continue;
        }
        let has_keyed = p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\");
        let has_definj = p_str.contains("/DefInjected/") || p_str.contains("\\DefInjected\\");
        if !(has_keyed || has_definj) {
            continue;
        }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }

        let mut reader = Reader::from_str(&content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        struct ElementFrame {
            name: String,
            line: Option<usize>,
            has_text: bool,
            buffer: String,
        }
        let mut stack: Vec<ElementFrame> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let offset = reader.buffer_position();
                    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                    let line = line_for_offset(offset, &line_starts);
                    stack.push(ElementFrame {
                        name,
                        line,
                        has_text: false,
                        buffer: String::new(),
                    });
                }
                Ok(Event::End(_)) => {
                    if let Some(frame) = stack.pop() {
                        if stack.len() == 1 && !frame.name.is_empty() {
                            let source = if frame.has_text {
                                frame.buffer
                            } else {
                                String::new()
                            };
                            out.push(TransUnit {
                                key: frame.name,
                                source: Some(source),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let offset = reader.buffer_position();
                    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                    let line = line_for_offset(offset, &line_starts);
                    if stack.len() == 1
                        && stack
                            .last()
                            .map(|frame| frame.name == "LanguageData")
                            .unwrap_or(false)
                        && !name.is_empty()
                    {
                        out.push(TransUnit {
                            key: name,
                            source: Some(String::new()),
                            path: p.to_path_buf(),
                            line,
                        });
                    } else if stack.len() == 2 {
                        if let Some(frame) = stack.last_mut() {
                            if name == "LineBreak" {
                                frame.has_text = true;
                                frame.buffer.push('\n');
                            }
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    let text = t
                        .unescape()
                        .unwrap_or_else(|_| {
                            Cow::Owned(String::from_utf8_lossy(t.as_ref()).into_owned())
                        })
                        .trim()
                        .to_string();
                    if stack.len() == 2 {
                        if let Some(frame) = stack.last_mut() {
                            if !text.is_empty() {
                                frame.buffer.push_str(&text);
                                frame.has_text = true;
                            }
                        }
                    }
                }
                Ok(Event::CData(t)) => {
                    let text = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                    if stack.len() == 2 {
                        if let Some(frame) = stack.last_mut() {
                            if !text.is_empty() {
                                frame.buffer.push_str(&text);
                                frame.has_text = true;
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    Ok(out)
}

/// Scan RimWorld Defs XML to derive implicit English source keys like
/// "<defName>.<field>" for common translatable fields, e.g. label/description.
pub fn scan_defs_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    scan_defs_xml_under(root, None)
}

/// Same as `scan_defs_xml`, but restricts scanning to a specific `defs_root` when provided.
pub fn scan_defs_xml_under(root: &Path, defs_root: Option<&Path>) -> CoreResult<Vec<TransUnit>> {
    scan_defs_xml_under_with_fields(root, defs_root, &[])
}

/// Like `scan_defs_xml_under`, but allows adding extra field names to include.
/// Matching is case-insensitive and only considers immediate child elements under a Def entry.
pub fn scan_defs_xml_under_with_fields(
    root: &Path,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<TransUnit> = Vec::new();

    fn line_for_offset(offset: usize, starts: &[usize]) -> Option<usize> {
        if starts.is_empty() {
            return None;
        }
        match starts.binary_search(&offset) {
            Ok(idx) => Some(idx + 1),
            Err(idx) if idx > 0 => Some(idx),
            _ => Some(1),
        }
    }

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Defs/....xml (including versioned folders like 1.4/Defs, v1.6/Defs)
        let p_str = p.to_string_lossy();
        if let Some(base) = defs_root {
            // When defs_root is provided, only include XML paths under it
            if !p.starts_with(base) {
                continue;
            }
        } else {
            if !(p_str.contains("/Defs/") || p_str.contains("\\Defs\\")) {
                continue;
            }
        }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Precompute line starts for approximate line mapping
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }

        // Use a lightweight DOM for comfortable traversal
        let doc = match roxmltree::Document::parse(&content) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Recognized translatable fields commonly present across Def types (conservative defaults).
        const DEFAULT_FIELDS: &[&str] = &[
            "label",
            "labelShort",
            "labelPlural",
            "description",
            "helpText",
            "reportString",
            "gerundLabel",
        ];
        let mut all_fields: Vec<String> = DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect();
        for s in extra_fields {
            if !s.trim().is_empty() {
                all_fields.push(s.trim().to_string());
            }
        }

        for node in doc.root_element().descendants().filter(|n| n.is_element()) {
            // Def entries live directly under <Defs> or nested under lists, but
            // they always contain a <defName> child with text.
            let def_name = node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(def_name) = def_name else { continue };

            // For each known field present as an immediate child element, emit a unit
            for field in &all_fields {
                if let Some(fnode) = node
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(field))
                {
                    let val = fnode.text().unwrap_or("").trim().to_string();
                    // roxmltree doesn't expose line number directly; approximate using start byte
                    let line = fnode.range().start;
                    let line = usize::try_from(line).unwrap_or(0);
                    let line = line_for_offset(line, &line_starts);
                    out.push(TransUnit {
                        key: format!("{}.{}", def_name, field),
                        source: Some(val),
                        path: p.to_path_buf(),
                        line,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Scan both Languages (Keyed/DefInjected) and Defs (implicit English) to provide
/// a complete view of translation units present in a mod.
pub fn scan_all_units(root: &Path) -> CoreResult<Vec<TransUnit>> {
    scan_all_units_with_defs(root, None)
}

/// Scan Languages (Keyed/DefInjected) and Defs with optional override of Defs root path.
pub fn scan_all_units_with_defs(root: &Path, defs_root: Option<&Path>) -> CoreResult<Vec<TransUnit>> {
    scan_all_units_with_defs_and_fields(root, defs_root, &[])
}

/// Scan with optional Defs root and extra fields for Defs extraction.
pub fn scan_all_units_with_defs_and_fields(
    root: &Path,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> CoreResult<Vec<TransUnit>> {
    let mut units = scan_keyed_xml(root)?;
    match scan_defs_xml_under_with_fields(root, defs_root, extra_fields) {
        Ok(mut defs) => units.append(&mut defs),
        Err(_) => {}
    }
    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_keyed_xml_handles_self_closing_keys() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mods/TestMod/Languages/TestLang/Keyed");
        fs::create_dir_all(&keyed_dir)?;

        let file_path = keyed_dir.join("SelfClosing.xml");
        fs::write(
            &file_path,
            r#"<LanguageData>
    <FullKey>Hello RimWorld</FullKey>
    <EmptyKey/>
    <Nested>
        <NestedEmpty/>
    </Nested>
</LanguageData>
"#,
        )?;

        let units = scan_keyed_xml(dir.path())?;

        assert!(
            units
                .iter()
                .any(|u| u.key == "FullKey" && u.source.as_deref() == Some("Hello RimWorld")),
            "FullKey should be parsed with text",
        );

        let empty = units
            .iter()
            .find(|u| u.key == "EmptyKey")
            .expect("EmptyKey should be produced for self-closing elements");
        assert_eq!(empty.source.as_deref(), Some(""));
        assert_eq!(empty.path, file_path);
        assert!(empty.line.is_some());

        assert!(
            units.iter().all(|u| u.key != "NestedEmpty"),
            "Nested self-closing keys should not be emitted",
        );

        Ok(())
    }

    #[test]
    fn scan_keyed_xml_merges_fragmented_text() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mods/TestMod/Languages/TestLang/Keyed");
        fs::create_dir_all(&keyed_dir)?;

        let file_path = keyed_dir.join("Fragments.xml");
        fs::write(
            &file_path,
            r#"<LanguageData>
    <KeyWithBreak>Part<LineBreak/>Rest</KeyWithBreak>
</LanguageData>
"#,
        )?;

        let units = scan_keyed_xml(dir.path())?;

        let unit = units
            .iter()
            .find(|u| u.key == "KeyWithBreak")
            .expect("KeyWithBreak should be parsed");

        assert_eq!(unit.source.as_deref(), Some("Part\nRest"));
        assert_eq!(unit.path, file_path);

        Ok(())
    }
}

#[cfg(test)]
mod defs_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_defs_extracts_common_fields() -> CoreResult<()> {
        let dir = tempdir()?;
        let defs_dir = dir.path().join("Mods/TestMod/Defs/ThingDefs_Items");
        fs::create_dir_all(&defs_dir)?;
        let file_path = defs_dir.join("Apparel.xml");
        fs::write(
            &file_path,
            r#"<Defs>
  <ThingDef>
    <defName>Apparel_Parka</defName>
    <label>parka</label>
    <description>A warm parka for cold climates.</description>
  </ThingDef>
</Defs>
"#,
        )?;

        let units = scan_defs_xml(dir.path())?;
        assert!(units.iter().any(|u| u.key == "Apparel_Parka.label" && u.source.as_deref() == Some("parka")));
        assert!(units.iter().any(|u| u.key == "Apparel_Parka.description" && u.source.as_deref() == Some("A warm parka for cold climates.")));
        Ok(())
    }
}
