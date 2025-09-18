use rimloc_core::{Result, TransUnit, RimLocError};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};

/// Рекурсивно проходит по `root`, находит `.xml` и извлекает Keyed-строки.
pub fn scan_keyed_xml(root: &Path) -> Result<Vec<TransUnit>> {
    let mut out: Vec<TransUnit> = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            if let Ok(txt) = std::fs::read_to_string(path) {
                match extract_from_xml(&txt, path) {
                    Ok(mut units) => out.append(&mut units),
                    Err(e) => {
                        // мягко логируем, не падаем на одном файле
                        eprintln!("[rimloc] WARN: {path:?}: {e}");
                    }
                }
            }
        }
    }

    Ok(out)
}

fn extract_from_xml(xml: &str, path: &Path) -> Result<Vec<TransUnit>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| RimLocError::Xml(format!("{e}")))?;

    let root = doc.root_element();
    if root.tag_name().name() != "LanguageData" {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for node in root.children().filter(|n| n.is_element()) {
        let key = node.tag_name().name().to_string();
        let value = node
            .text()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let line = None; // пока без номеров строк

        out.push(TransUnit {
            key,
            source: value,
            path: PathBuf::from(path),
            line,
        });
    }

    Ok(out)
}
