use color_eyre::eyre::{eyre, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;

/// Parse a minimal XLIFF 1.2 file and return (key -> translation) map.
pub fn read_xliff_12(path: &Path) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut cur_id: Option<String> = None;
    let mut in_target = false;
    let mut cur_val = String::new();
    let mut map = HashMap::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_vec();
                if name.as_slice() == b"trans-unit" {
                    for a in e.attributes().flatten() {
                        if a.key.as_ref() == b"id" {
                            cur_id = Some(String::from_utf8_lossy(&a.value).to_string());
                        }
                    }
                } else if name.as_slice() == b"target" { in_target = true; cur_val.clear(); }
            }
            Ok(Event::Text(t)) => {
                if in_target { cur_val.push_str(&t.unescape().unwrap_or_default()); }
            }
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_vec();
                if name.as_slice() == b"target" { in_target = false; }
                else if name.as_slice() == b"trans-unit" {
                    if let Some(id) = cur_id.take() {
                        map.insert(id, cur_val.trim().to_string());
                        cur_val.clear();
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(eyre!("{e}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(map)
}

/// Write a single LanguageData XML from an XLIFF file (Keyed style)
pub fn xliff_to_language_data(out_path: &Path, xliff_path: &Path) -> Result<()> {
    let map = read_xliff_12(xliff_path)?;
    let entries: Vec<(String, String)> = map.into_iter().collect();
    rimloc_import_po::write_language_data_xml(out_path, &entries)
}
