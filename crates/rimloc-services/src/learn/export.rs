use crate::Result;
use super::parser::Candidate;
use std::path::Path;

pub fn write_missing_json(path: &Path, cands: &[&Candidate]) -> Result<()> {
    #[derive(serde::Serialize)]
    #[allow(non_snake_case)]
    struct Item<'a> { defType: &'a str, defName: &'a str, fieldPath: &'a str, confidence: f32, sourceFile: String }
    let items: Vec<Item> = cands.iter().map(|c| Item { defType: &c.def_type, defName: &c.def_name, fieldPath: &c.field_path, confidence: c.confidence.unwrap_or(1.0), sourceFile: c.source_file.display().to_string() }).collect();
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &items)?;
    Ok(())
}

pub fn write_suggested_xml(path: &Path, cands: &[&Candidate]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "<LanguageData>")?;
    for c in cands {
        let key = format!("{}.{}", c.def_name, c.field_path);
        writeln!(f, "  <{}></{}>", key, key)?;
    }
    writeln!(f, "</LanguageData>")?;
    Ok(())
}
