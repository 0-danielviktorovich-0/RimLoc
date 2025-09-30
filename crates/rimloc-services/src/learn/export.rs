use super::parser::Candidate;
use crate::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn write_missing_json(path: &Path, cands: &[&Candidate]) -> Result<()> {
    #[derive(serde::Serialize)]
    #[allow(non_snake_case)]
    struct Item<'a> {
        defType: &'a str,
        defName: &'a str,
        fieldPath: &'a str,
        confidence: f32,
        sourceFile: String,
    }
    let items: Vec<Item> = cands
        .iter()
        .map(|c| Item {
            defType: &c.def_type,
            defName: &c.def_name,
            fieldPath: &c.field_path,
            confidence: c.confidence.unwrap_or(1.0),
            sourceFile: c.source_file.display().to_string(),
        })
        .collect();
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &items)?;
    Ok(())
}

pub fn write_suggested_xml(path: &Path, cands: &[&Candidate]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "<LanguageData>")?;
    let mut items: Vec<&Candidate> = cands.to_vec();
    items.sort_by(|a, b| {
        (
            a.def_type.as_str(),
            a.def_name.as_str(),
            a.field_path.as_str(),
        )
            .cmp(&(
                b.def_type.as_str(),
                b.def_name.as_str(),
                b.field_path.as_str(),
            ))
    });
    for c in items {
        let key = format!("{}.{}", c.def_name, c.field_path);
        // EN: comment as translator hint
        writeln!(f, "  <!-- EN: {} -->", escape_xml_comment(&c.value))?;
        writeln!(f, "  <{key}>{}</{key}>", escape_xml(&c.value))?;
    }
    writeln!(f, "</LanguageData>")?;
    Ok(())
}

pub fn write_suggested_tree(base: &Path, lang_dir: &str, cands: &[&Candidate]) -> Result<()> {
    let mut groups: BTreeMap<(String, PathBuf), Vec<&Candidate>> = BTreeMap::new();
    for c in cands {
        let file_name = c
            .source_file
            .file_name()
            .map(|s| s.to_os_string())
            .unwrap_or_else(|| std::ffi::OsString::from("Defs.xml"));
        let file_path = PathBuf::from("DefInjected")
            .join(&c.def_type)
            .join(file_name);
        groups
            .entry((c.def_type.clone(), file_path))
            .or_default()
            .push(*c);
    }
    for ((_, rel_path), list) in groups {
        let target = base.join("Languages").join(lang_dir).join(&rel_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_language_file(&target, &list)?;
    }
    Ok(())
}

fn write_language_file(path: &Path, cands: &[&Candidate]) -> Result<()> {
    use std::io::Write;
    let mut items: Vec<&Candidate> = cands.to_vec();
    items.sort_by(|a, b| {
        (a.def_name.as_str(), a.field_path.as_str())
            .cmp(&(b.def_name.as_str(), b.field_path.as_str()))
    });
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "<LanguageData>")?;
    for c in items {
        let key = format!("{}.{}", c.def_name, c.field_path);
        writeln!(f, "  <!-- EN: {} -->", escape_xml_comment(&c.value))?;
        writeln!(f, "  <{key}>{}</{key}>", escape_xml(&c.value))?;
    }
    writeln!(f, "</LanguageData>")?;
    Ok(())
}

fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn escape_xml_comment(text: &str) -> String {
    // Minimal sanitizer for XML comments: avoid "--" and ensure no closing marker
    let mut s = text.replace("--", "- -");
    if s.ends_with('-') {
        s.push(' ');
    }
    s
}
