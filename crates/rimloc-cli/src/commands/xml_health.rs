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

    let report = rimloc_services::xml_health_scan(&root, lang_dir.as_deref())?;
    let mut issues: Vec<Issue> = report
        .issues
        .into_iter()
        .map(|it| Issue { path: it.path, category: it.category, error: it.error })
        .collect();
    let checked = report.checked;

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
                    "tag-mismatch" => {
                        Some("Ensure opening/closing tags match and are nested correctly.")
                    }
                    "invalid-entity" => {
                        Some("Escape '&' as &amp;; use &lt;/&gt;/&amp; or numeric entities.")
                    }
                    "unexpected-doctype" => {
                        Some("Remove <!DOCTYPE>; not required for LanguageData XML.")
                    }
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
