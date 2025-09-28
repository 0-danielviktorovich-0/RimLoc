use crate::version::resolve_game_version_root;
use std::io::IsTerminal;

#[derive(Debug, Clone)]
pub enum MorphProvider {
    Dummy,
    MorpherApi,
    Pymorphy2,
}

impl MorphProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "morpher" | "morpher_api" => Self::MorpherApi,
            "pymorphy2" | "pymorphy" => Self::Pymorphy2,
            _ => Self::Dummy,
        }
    }
}

// helper heuristics moved to services

#[allow(clippy::too_many_arguments)]
pub fn run_morph(
    root: std::path::PathBuf,
    provider: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    filter_key_regex: Option<String>,
    limit: Option<usize>,
    game_version: Option<String>,
    timeout_ms: Option<u64>,
    cache_size: Option<usize>,
    pymorphy_url: Option<String>,
) -> color_eyre::Result<()> {
    // patterns handled in services

    let provider = MorphProvider::from_str(provider.as_deref().unwrap_or("dummy"));
    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "morph_version_resolved", version = ver, path = %scan_root.display());
    }

    let target_lang = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };

    let http_timeout = timeout_ms.unwrap_or(1500);
    let opts = rimloc_services::MorphOptions {
        provider: match provider {
            MorphProvider::MorpherApi => rimloc_services::MorphProvider::MorpherApi,
            MorphProvider::Pymorphy2 => rimloc_services::MorphProvider::Pymorphy2,
            _ => rimloc_services::MorphProvider::Dummy,
        },
        target_lang_dir: target_lang.clone(),
        filter_key_regex,
        limit,
        timeout_ms: http_timeout,
        cache_size: cache_size.unwrap_or(1024),
        pymorphy_url,
    };
    let res = rimloc_services::morph_generate(&scan_root, &opts)?;
    crate::ui_ok!(
        "morph-summary",
        processed = (res.processed as i64),
        lang = res.lang.as_str()
    );
    if res.warn_no_morpher {
        crate::ui_warn!("morph-provider-morpher-stub");
    }
    if res.warn_no_pymorphy {
        crate::ui_warn!("morph-provider-morpher-stub");
    }
    Ok(())
}
// provider-specific helpers moved to services
