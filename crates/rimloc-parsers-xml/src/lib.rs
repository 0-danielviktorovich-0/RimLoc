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
        // filter to .../Languages/<Locale>/Keyed/....xml
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Languages/") || p_str.contains("\\Languages\\")) {
            continue;
        }
        if !(p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\")) {
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
