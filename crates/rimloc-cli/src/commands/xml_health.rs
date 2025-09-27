use std::io::IsTerminal;
use walkdir::WalkDir;

#[derive(serde::Serialize)]
struct Issue {
    path: String,
    category: &'static str,
    error: String,
}

pub fn run_xml_health(
    root: std::path::PathBuf,
    format: String,
    lang_dir: Option<String>,
    strict: bool,
    only: Option<Vec<String>>,
    except: Option<Vec<String>>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "xml_health_args", root = ?root, format = %format, lang_dir = ?lang_dir);

    let mut issues: Vec<Issue> = Vec::new();
    let mut checked = 0usize;

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
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
        // Optionally restrict to specific language folder
        if let Some(dir) = lang_dir.as_ref() {
            // simple substring match for speed (path normalization differences across OS)
            let s = p.to_string_lossy();
            if !s.contains("/Languages/") && !s.contains("\\Languages\\") {
                continue;
            }
            if !(s.contains(&format!("/Languages/{dir}/"))
                || s.contains(&format!("\\Languages\\{dir}\\")))
            {
                continue;
            }
        }

        checked += 1;
        let content = match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => {
                issues.push(Issue {
                    path: p.display().to_string(),
                    category: "encoding",
                    error: format!("{e}"),
                });
                continue;
            }
        };
        // Detect explicit non-UTF8 encoding declarations
        {
            let head = &content.as_bytes()[..content.len().min(512)];
            if let Ok(head_str) = std::str::from_utf8(head) {
                let re = regex::Regex::new(r#"(?i)<\?xml[^>]*encoding\s*=\s*['\"]([^'\"]+)['\"][^>]*\?>"#)
                    .unwrap();
                if let Some(caps) = re.captures(head_str) {
                    let enc = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let enc_norm = enc.to_ascii_lowercase().replace('_', "-");
                    if enc_norm != "utf-8" && enc_norm != "utf8" {
                        issues.push(Issue {
                            path: p.display().to_string(),
                            category: "encoding-detected",
                            error: format!("XML declares encoding={enc}; expected UTF-8"),
                        });
                    }
                }
                // Detect DOCTYPE usage
                if head_str.to_ascii_lowercase().contains("<!doctype") {
                    issues.push(Issue {
                        path: p.display().to_string(),
                        category: "unexpected-doctype",
                        error: "DOCTYPE present (not expected in LanguageData)".into(),
                    });
                }
            }
        }
        if content.chars().any(|ch| {
            let c = ch as u32;
            c < 0x20 && c != 0x09 && c != 0x0A && c != 0x0D
        }) {
            issues.push(Issue {
                path: p.display().to_string(),
                category: "invalid-char",
                error: "control character < 0x20".into(),
            });
        }
        let mut reader = quick_xml::Reader::from_str(&content);
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut err: Option<String> = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => { /* keep consuming */ }
                Err(e) => {
                    err = Some(format!("{e}"));
                    break;
                }
            }
            buf.clear();
        }
        if let Some(e) = err {
            let el = e.to_ascii_lowercase();
            let cat = if el.contains("mismatch") {
                "tag-mismatch"
            } else if el.contains("doctype") || el.contains("dtd") {
                "unexpected-doctype"
            } else if el.contains("entity") || el.contains("ampersand") || el.contains("escape") {
                "invalid-entity"
            } else {
                "parse"
            };
            issues.push(Issue {
                path: p.display().to_string(),
                category: cat,
                error: e,
            });
        }
    }

    // filter by categories
    if let Some(onlyv) = only.as_ref() {
        let set: std::collections::HashSet<&str> = onlyv.iter().map(|s| s.as_str()).collect();
        issues.retain(|it| set.contains(it.category));
    }
    if let Some(exceptv) = except.as_ref() {
        let set: std::collections::HashSet<&str> = exceptv.iter().map(|s| s.as_str()).collect();
        issues.retain(|it| !set.contains(it.category));
    }
    if format == "json" {
        #[derive(serde::Serialize)]
        struct Out {
            checked: usize,
            issues: Vec<Issue>,
        }
        let out = Out { checked, issues };
        serde_json::to_writer(std::io::stdout().lock(), &out)?;
        if strict && !out.issues.is_empty() {
            color_eyre::eyre::bail!("xmlhealth-issues");
        }
        return Ok(());
    }

    if issues.is_empty() {
        crate::ui_ok!("xmlhealth-summary",);
    } else {
        // optional short "how to fix" hints for text output
        let is_tty = std::io::stdout().is_terminal();
        for it in &issues {
            crate::ui_err!(
                "xmlhealth-issue-line",
                path = it.path.as_str(),
                error = it.error.as_str()
            );
            if is_tty && format == "text" {
                let hint = match it.category {
                    "encoding" => Some("Save as UTF-8 (no BOM)."),
                    "encoding-detected" => Some("Use UTF-8; remove or fix encoding declaration."),
                    "invalid-char" => Some("Remove control chars < 0x20; keep \t, \n, \r only."),
                    "tag-mismatch" => Some("Ensure opening/closing tags match and are nested correctly."),
                    "invalid-entity" => Some("Escape '&' as &amp;; use &lt;/&gt;/&amp; or numeric entities."),
                    "unexpected-doctype" => Some("Remove <!DOCTYPE>; not required for LanguageData XML."),
                    _ => None,
                };
                if let Some(h) = hint {
                    crate::ui_info!("xmlhealth-hint-line", hint = h);
                }
            }
        }
        crate::ui_warn!("xmlhealth-issues",);
        if strict {
            color_eyre::eyre::bail!("xmlhealth-issues");
        }
    }
    Ok(())
}
