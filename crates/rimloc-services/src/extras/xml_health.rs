use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub path: String,
    pub category: &'static str,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct HealthReport {
    pub checked: usize,
    pub issues: Vec<HealthIssue>,
}

/// Scan XML files and collect structural/encoding issues.
pub fn xml_health_scan(root: &Path, lang_dir: Option<&str>) -> crate::Result<HealthReport> {
    let mut issues: Vec<HealthIssue> = Vec::new();
    let mut checked = 0usize;
    let re_xml_decl: Regex = Regex::new(r#"(?i)<\?xml[^>]*encoding\s*=\s*['\"]([^'\"]+)['\"][^>]*\?>"#).unwrap();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()).map_or(true, |ext| !ext.eq_ignore_ascii_case("xml")) {
            continue;
        }
        if let Some(dir) = lang_dir {
            let s = p.to_string_lossy();
            if !s.contains("/Languages/") && !s.contains("\\Languages\\") {
                continue;
            }
            if !(s.contains(&format!("/Languages/{dir}/")) || s.contains(&format!("\\Languages\\{dir}\\"))) {
                continue;
            }
        }

        checked += 1;
        let content = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => {
                issues.push(HealthIssue { path: p.display().to_string(), category: "encoding", error: format!("{e}") });
                continue;
            }
        };

        // XML declaration encoding
        let head = &content.as_bytes()[..content.len().min(512)];
        if let Ok(head_str) = std::str::from_utf8(head) {
            if let Some(caps) = re_xml_decl.captures(head_str) {
                let enc = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let enc_norm = enc.to_ascii_lowercase().replace('_', "-");
                if enc_norm != "utf-8" && enc_norm != "utf8" {
                    issues.push(HealthIssue { path: p.display().to_string(), category: "encoding-detected", error: format!("XML declares encoding={enc}; expected UTF-8") });
                }
            }
            if head_str.to_ascii_lowercase().contains("<!doctype") {
                issues.push(HealthIssue { path: p.display().to_string(), category: "unexpected-doctype", error: "DOCTYPE present (not expected in LanguageData)".into() });
            }
        }

        if content.chars().any(|ch| { let c = ch as u32; c < 0x20 && c != 0x09 && c != 0x0A && c != 0x0D }) {
            issues.push(HealthIssue { path: p.display().to_string(), category: "invalid-char", error: "control character < 0x20".into() });
        }

        let mut reader = quick_xml::Reader::from_str(&content);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut err: Option<String> = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => { err = Some(format!("{e}")); break; }
            }
            buf.clear();
        }
        if let Some(e) = err {
            let el = e.to_ascii_lowercase();
            let cat = if el.contains("mismatch") { "tag-mismatch" }
                else if el.contains("doctype") || el.contains("dtd") { "unexpected-doctype" }
                else if el.contains("entity") || el.contains("ampersand") || el.contains("escape") { "invalid-entity" }
                else { "parse" };
            issues.push(HealthIssue { path: p.display().to_string(), category: cat, error: e });
        }
    }

    Ok(HealthReport { checked, issues })
}

