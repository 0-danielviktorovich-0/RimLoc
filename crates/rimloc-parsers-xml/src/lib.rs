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
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
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
                    });
                }
                Ok(Event::End(_)) => {
                    if let Some(frame) = stack.pop() {
                        if stack.len() == 1 && !frame.has_text && !frame.name.is_empty() {
                            out.push(TransUnit {
                                key: frame.name,
                                source: Some(String::new()),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
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
                            frame.has_text = true;
                            out.push(TransUnit {
                                key: frame.name.clone(),
                                source: Some(text),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
                        }
                    }
                }
                Ok(Event::CData(t)) => {
                    let text = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                    if stack.len() == 2 {
                        if let Some(frame) = stack.last_mut() {
                            frame.has_text = true;
                            out.push(TransUnit {
                                key: frame.name.clone(),
                                source: Some(text),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
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
