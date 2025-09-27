use std::io::IsTerminal;
use walkdir::WalkDir;

#[derive(serde::Serialize)]
struct Issue {
    path: String,
    error: String,
}

pub fn run_xml_health(
    root: std::path::PathBuf,
    format: String,
    lang_dir: Option<String>,
    strict: bool,
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
                    error: format!("read error: {e}"),
                });
                continue;
            }
        };
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
            issues.push(Issue {
                path: p.display().to_string(),
                error: e,
            });
        }
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
        for it in &issues {
            crate::ui_err!(
                "xmlhealth-issue-line",
                path = it.path.as_str(),
                error = it.error.as_str()
            );
        }
        crate::ui_warn!("xmlhealth-issues",);
        if strict {
            color_eyre::eyre::bail!("xmlhealth-issues");
        }
    }
    Ok(())
}
