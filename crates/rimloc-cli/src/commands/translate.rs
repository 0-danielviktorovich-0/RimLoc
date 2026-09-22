use crate::version::resolve_game_version_root;
use rimloc_llm::{
    glossary::Glossary, mock::MockProvider, provider::ProviderPreset, EngineOptions, KeySource,
    TranslationEngine,
};
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
pub fn run_translate(
    root: std::path::PathBuf,
    provider: String,
    model: Option<String>,
    base_url: Option<String>,
    source_lang: String,
    target_lang: String,
    glossary_file: Option<std::path::PathBuf>,
    checkpoint: Option<std::path::PathBuf>,
    out_po: std::path::PathBuf,
    batch_budget: usize,
    dry_run: bool,
    no_strict_placeholders: bool,
    key_env: Option<String>,
) -> color_eyre::Result<()> {
    use rimloc_llm::provider::Provider;

    let cfg = rimloc_config::load_config().unwrap_or_default();
    let effective_version = cfg.game_version.clone();
    let (scan_root, _ver) = resolve_game_version_root(&root, effective_version.as_deref())?;

    // 1) Extract source strings (same path as `scan --lang en`).
    let auto = rimloc_services::autodiscover_defs_context(&scan_root)?;
    let units = rimloc_services::scan_units_with_defs_and_dict(
        &scan_root,
        None,
        &auto
            .dict
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect(),
        &auto.extra_fields,
    )?;
    let source_dir = rimloc_import_po::rimworld_lang_dir(&source_lang);
    let src_units: Vec<rimloc_core::TransUnit> = units
        .into_iter()
        .filter(|u| rimloc_services::is_source_for_lang_dir(&u.path, &source_dir))
        .collect();

    // Deduplicate by key (same key may appear in Defs + mirror files).
    let mut by_key: BTreeMap<String, rimloc_core::TransUnit> = BTreeMap::new();
    for u in src_units {
        if u.source.as_deref().is_some_and(|s| !s.trim().is_empty()) {
            by_key.entry(u.key.clone()).or_insert(u);
        }
    }
    let llm_units: Vec<rimloc_llm::provider::TranslateUnit> = by_key
        .values()
        .map(|u| rimloc_llm::provider::TranslateUnit {
            id: u.key.clone(),
            source: u.source.clone().unwrap_or_default(),
            context: Some(
                u.path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        })
        .collect();

    ui_info!(
        "translate-extracted",
        count = llm_units.len(),
        path = scan_root.display().to_string()
    );

    // 2) Glossary (built-in baseline + user overrides).
    let glossary = match &glossary_file {
        Some(p) => {
            let mut g = Glossary::builtin();
            g.terms.extend(Glossary::load_json(p)?.terms);
            g
        }
        None => Glossary::builtin(),
    };

    // 3) Provider. `mock` needs no key; real providers resolve keys from
    //    keychain/env at call time and never log them.
    let key_source = match &key_env {
        Some(name) => KeySource::Env(name.clone()),
        None => KeySource::Auto,
    };
    let presets = rimloc_llm::provider::builtin_presets();
    let engine_provider: Box<dyn Provider> = match provider.as_str() {
        "mock" => Box::new(MockProvider::new()),
        "anthropic" => Box::new(rimloc_llm::anthropic::AnthropicProvider::new(
            model.unwrap_or_else(|| presets["anthropic"].model.clone()),
            key_source,
        )),
        id @ ("openai" | "zai" | "ollama") => {
            let mut preset: ProviderPreset = presets[id].clone();
            if let Some(m) = model {
                preset.model = m;
            }
            if let Some(u) = base_url {
                preset.base_url = Some(u);
            }
            Box::new(
                rimloc_llm::openai_compat::OpenAiCompatProvider::from_preset(&preset, key_source),
            )
        }
        other => {
            return Err(color_eyre::eyre::eyre!(
                "unknown provider `{other}` (expected: mock, anthropic, openai, zai, ollama)"
            ))
        }
    };

    // 4) Dry run: report scope/cost without any provider call.
    if dry_run {
        let est = rimloc_llm::cost::estimate(&llm_units, None);
        ui_info!(
            "translate-dryrun",
            units = est.units,
            batches = (est.units / 40).max(1),
            path = out_po.display().to_string()
        );
        return Ok(());
    }

    // 5) Translate with checkpoint/resume and strict placeholder validation.
    let opts = EngineOptions {
        source_lang: source_lang.clone(),
        target_lang: target_lang.clone(),
        batch_char_budget: batch_budget,
        strict_placeholders: !no_strict_placeholders,
        checkpoint_path: checkpoint,
        ..Default::default()
    };
    let engine = TranslationEngine::new(engine_provider.as_ref(), opts, glossary);
    let mut translations: BTreeMap<String, String> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    let summary = engine.translate(&llm_units, |outcome| match &outcome.translation {
        Some(t) => {
            translations.insert(outcome.id.clone(), t.clone());
        }
        None => failures.push(format!(
            "{}: {}",
            outcome.id,
            outcome.error.clone().unwrap_or_default()
        )),
    })?;

    // 6) Write translated PO for the existing import-po/build-mod pipeline.
    let mut po = String::new();
    po.push_str("msgid \"\"\nmsgstr \"\"\n\"X-RimLoc-Generator: rimloc translate\\n\"\n\n");
    for u in by_key.values() {
        let Some(tr) = translations.get(&u.key) else {
            continue;
        };
        let esc = |s: &str| {
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        };
        po.push_str(&format!("msgctxt \"{}\"\n", esc(&u.key)));
        po.push_str(&format!(
            "msgid \"{}\"\n",
            esc(u.source.as_deref().unwrap_or(""))
        ));
        po.push_str(&format!("msgstr \"{}\"\n\n", esc(tr)));
    }
    if let Some(parent) = out_po.parent() {
        std::fs::create_dir_all(parent)?;
    }
    rimloc_services::write_atomic(&out_po, po.as_bytes())?;
    ui_info!("translate-po-saved", path = out_po.display().to_string());
    ui_info!(
        "translate-summary",
        translated = summary.translated,
        failed = summary.failed,
        skipped = summary.skipped_done
    );
    if !failures.is_empty() {
        let sep = "; ";
        let sample = failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(sep);
        tracing::warn!(event = "translate_failures", count = failures.len(), sample = %sample);
    }
    if summary.failed > 0 {
        std::process::exit(2);
    }
    Ok(())
}
