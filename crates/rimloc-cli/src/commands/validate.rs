use crate::version::resolve_game_version_root;

fn is_under_languages_dir(path: &std::path::Path, lang_dir: &str) -> bool {
    let mut comps = path.components();
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn run_validate(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    format: String,
    game_version: Option<String>,
    include_all_versions: bool,
    use_color: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "validate_args", root = ?root, game_version = ?game_version, include_all_versions = include_all_versions);

    let (scan_root, selected_version) = if include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, game_version.as_deref())?
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "validate_version_resolved", version = ver, path = %scan_root.display());
    }

    let mut units = rimloc_parsers_xml::scan_keyed_xml(&scan_root)?;

    if let Some(dir) = source_lang_dir.as_ref() {
        units.retain(|u| is_under_languages_dir(&u.path, dir.as_str()));
        tracing::info!(event = "validate_filtered_by_dir", source_lang_dir = %dir, remaining = units.len());
    } else if let Some(code) = source_lang.as_ref() {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_under_languages_dir(&u.path, dir.as_str()));
        tracing::info!(event = "validate_filtered_by_code", source_lang = %code, source_dir = %dir, remaining = units.len());
    }

    let msgs = rimloc_validate::validate(&units)?;
    if format == "json" {
        #[derive(serde::Serialize)]
        struct JsonMsg<'a> {
            schema_version: u32,
            kind: &'a str,
            key: &'a str,
            path: &'a str,
            line: Option<usize>,
            message: &'a str,
        }
        let items: Vec<JsonMsg> = msgs
            .iter()
            .map(|m| JsonMsg {
                schema_version: crate::OUTPUT_SCHEMA_VERSION,
                kind: m.kind.as_str(),
                key: m.key.as_str(),
                path: m.path.as_str(),
                line: m.line,
                message: m.message.as_str(),
            })
            .collect();
        serde_json::to_writer(std::io::stdout().lock(), &items)?;
        return Ok(());
    }
    if msgs.is_empty() {
        if use_color {
            use owo_colors::OwoColorize;
            println!("{} {}", "✔".green(), tr!("validate-clean"));
        } else {
            println!("✔ {}", tr!("validate-clean"));
        }
    } else {
        for m in msgs {
            if !use_color {
                println!(
                    "[{}] {} ({}:{}) — {}",
                    m.kind,
                    m.key,
                    m.path,
                    m.line.unwrap_or(0),
                    m.message
                );
            } else {
                use owo_colors::OwoColorize;
                let tag = match m.kind.as_str() {
                    "duplicate" => "⚠",
                    "empty" => "✖",
                    "placeholder-check" => "ℹ",
                    _ => "•",
                };
                let plain_kind_token = m.kind.as_str();
                println!(
                    "{} [{}] {} ({}:{}) — {}",
                    tag,
                    plain_kind_token,
                    m.key.green(),
                    m.path.blue(),
                    m.line.unwrap_or(0).to_string().magenta(),
                    m.message
                );
            }
        }
    }
    Ok(())
}
