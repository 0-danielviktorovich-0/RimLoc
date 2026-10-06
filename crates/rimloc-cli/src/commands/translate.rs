//! `rimloc translate` — LLM translation into CANONICAL project state.
//!
//! Canonical path (PO is no longer the internal seam): the source mod is
//! opened as a session project (`ProjectSessionManager::create`), provider
//! results land as typed [`TranslationIntent`]s through `apply`, and the
//! command output is a versioned canonical JSON report. `--emit-po` keeps
//! the legacy adapter artifact for old pipelines (import-po/build-mod).

use rimloc_llm::{
    glossary::Glossary, mock::MockProvider, provider::ProviderPreset, EngineOptions, KeySource,
    TranslationEngine,
};
use rimloc_services::contract::{ApplyIntentsRequest, IntentAction, TranslationIntent};
use rimloc_services::ProjectSessionManager;
use std::collections::BTreeMap;

/// Bump on any breaking change to the JSON report shape.
const RESULTS_SCHEMA_VERSION: u32 = 1;

#[derive(serde::Serialize)]
struct TranslateResultsJson<'a> {
    schema_version: u32,
    generator: &'static str,
    project_id: &'a str,
    /// Target locale in the RimWorld folder contract (e.g. "Russian").
    locale: &'a str,
    provider: &'a str,
    results: Vec<TranslateResultRow<'a>>,
    summary: TranslateSummaryJson,
}

#[derive(serde::Serialize)]
struct TranslateResultRow<'a> {
    /// Logical source key (the Keyed key or `<defName>.<field>`).
    key: &'a str,
    /// Full structural identity rendering (`kind·def_type·key`).
    identity: String,
    locale: &'a str,
    source: &'a str,
    target: Option<&'a str>,
    /// `applied` (intent acked into canonical state), `refused` (the session
    /// rejected the intent — code/message carry the reason) or `failed`
    /// (the provider did not return a usable translation).
    status: &'a str,
    provenance: TranslateProvenanceJson<'a>,
}

#[derive(serde::Serialize)]
struct TranslateProvenanceJson<'a> {
    provider: &'a str,
    /// Canonical origin class of the produced text.
    origin: &'static str,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    def_type: Option<&'a str>,
    /// Effective source file of the entry (project-root relative).
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a str>,
    /// Winner reason from the scan pipeline (pre-freeze vocabulary).
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_by: Option<&'a str>,
    /// Raw engine outcome status for this unit ("ok" / "skipped-done").
    engine_status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refused_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

#[derive(serde::Serialize)]
struct TranslateSummaryJson {
    translated: usize,
    applied: usize,
    refused: usize,
    failed: usize,
    skipped_done: usize,
}

/// One report row being assembled: the canonical entry identity plus the
/// provider outcome for its key.
struct PendingRow {
    key: String,
    identity: String,
    kind: String,
    def_type: Option<String>,
    file: Option<String>,
    selected_by: Option<String>,
    source: String,
    target: Option<String>,
    engine_status: String,
    error: Option<String>,
    /// Status of the canonical apply for this row: `applied`, `refused`
    /// (with the session's reason) or `failed` (no provider text).
    apply_status: ApplyStatus,
}

enum ApplyStatus {
    Applied,
    Refused { code: String, message: String },
    Failed,
}

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
    out_json: std::path::PathBuf,
    managed_root: Option<std::path::PathBuf>,
    batch_budget: usize,
    dry_run: bool,
    no_strict_placeholders: bool,
    key_env: Option<String>,
    emit_po: bool,
) -> color_eyre::Result<()> {
    use rimloc_llm::provider::Provider;

    let cfg = rimloc_config::load_config().unwrap_or_default();
    let effective_version = cfg.game_version.clone();

    // 1) Canonical source inventory. The session create path and the
    //    dry-run both run the SAME pipeline (`build_project`): the effective
    //    view resolves flat / version-dir / LoadFolders layouts, and the
    //    source side is the canonical English inventory. Dry-run stays
    //    write-free (in-memory project, no managed record, no provider
    //    call) while producing the exact scope the real run applies.
    let inventory = rimloc_services::project::build_project(&root, effective_version.as_deref())?;

    // Eligibility pre-filter: the session refuses non-translatable intents
    // per-entry as data; excluding them here keeps the provider call volume
    // honest (never billed for entries the canonical gate would refuse).
    let engine_gate = rimloc_services::EligibilityEngine::new();
    let entries: Vec<_> = inventory
        .entries
        .iter()
        .filter(|e| {
            !e.text.trim().is_empty()
                && engine_gate.evaluate(e, None).decision
                    != rimloc_domain::eligibility::Decision::NonTranslatable
        })
        .cloned()
        .collect();

    // Unit ids stay the LOGICAL KEY: checkpoint files written by earlier
    // releases resume unchanged, and same-key entries across kinds receive
    // one identical translation (the legacy dedup semantics).
    let llm_units: Vec<rimloc_llm::provider::TranslateUnit> = entries
        .iter()
        .map(|e| rimloc_llm::provider::TranslateUnit {
            id: e.id.key.clone(),
            source: e.text.clone(),
            context: Some(
                e.contexts
                    .first()
                    .map(|c| c.file.clone())
                    .unwrap_or_default(),
            ),
        })
        .collect();

    ui_info!(
        "translate-extracted",
        count = llm_units.len(),
        path = root.display().to_string()
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
            } else if id == "ollama" {
                // Local server: auto-detect the first installed model.
                if let Some(u) = preset.base_url.as_deref() {
                    if let Some(detected) = rimloc_llm::openai_compat::first_installed_model(u) {
                        preset.model = detected;
                    }
                }
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

    // The target locale in the RimWorld folder contract — the form the
    // canonical session (and every export path) requires.
    let locale_dir = rimloc_import_po::rimworld_lang_dir(&target_lang);

    // 4) Dry run: report scope/cost without any provider call and without
    //    any write (parity: this count is exactly the intent count a real
    //    run produces — both come from the same inventory pipeline).
    if dry_run {
        let est = rimloc_llm::cost::estimate(&llm_units, None);
        ui_info!(
            "translate-dryrun",
            units = est.units,
            batches = (est.units / 40).max(1),
            path = out_json.display().to_string()
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
    // key -> (translated text, raw engine status)
    let mut translations: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut failures: BTreeMap<String, String> = BTreeMap::new();
    let summary = engine.translate(&llm_units, |outcome| match &outcome.translation {
        Some(t) => {
            translations.insert(outcome.id.clone(), (t.clone(), outcome.status.clone()));
        }
        None => {
            failures.insert(
                outcome.id.clone(),
                outcome.error.clone().unwrap_or_default(),
            );
        }
    })?;

    // 6) Canonical apply: one writer — the session. Translations land as
    //    typed intents on a persisted managed project; no PO file is
    //    written on this path.
    let managed_dir = match managed_root {
        Some(p) => p,
        None => default_managed_root(),
    };
    let manager = ProjectSessionManager::new(&managed_dir)
        .map_err(|e| color_eyre::eyre::eyre!("managed root {}: {e}", managed_dir.display()))?;
    let snapshot = manager
        .create(&root, effective_version.as_deref())
        .map_err(|e| color_eyre::eyre::eyre!("canonical project create: {e}"))?;
    let project_id = snapshot.project_id.clone();
    ui_info!(
        "translate-project-created",
        id = snapshot.project_id.as_str()
    );

    // Report rows are built per entry FIRST (stable order), then intents
    // reference rows by index so per-intent refusals map back exactly.
    let mut rows: Vec<PendingRow> = Vec::with_capacity(entries.len());
    let mut intent_rows: Vec<usize> = Vec::new();
    let mut intents: Vec<TranslationIntent> = Vec::new();
    for e in &entries {
        let (target, engine_status, error) = match translations.get(&e.id.key) {
            Some((t, s)) => (Some(t.clone()), s.clone(), None),
            None => (
                None,
                "error".to_string(),
                Some(failures.get(&e.id.key).cloned().unwrap_or_default()),
            ),
        };
        let (apply_status, intent_text) = match &target {
            Some(t) => (ApplyStatus::Applied, Some(t.clone())),
            None => (ApplyStatus::Failed, None),
        };
        let row_idx = rows.len();
        rows.push(PendingRow {
            key: e.id.key.clone(),
            identity: e.id.display_identity(),
            kind: format!("{:?}", e.id.kind),
            def_type: e.id.def_type.clone(),
            file: e.contexts.first().map(|c| c.file.clone()),
            selected_by: e.provenance.selected_by.clone(),
            source: e.text.clone(),
            engine_status,
            error,
            apply_status,
            target,
        });
        if let Some(text) = intent_text {
            intent_rows.push(row_idx);
            intents.push(TranslationIntent {
                entry: e.id.clone(),
                locale: locale_dir.clone(),
                action: IntentAction::SetTranslation,
                text: Some(text),
            });
        }
    }

    let apply_res = manager
        .apply(&ApplyIntentsRequest {
            project_id,
            expected_revision: snapshot.revision,
            session_epoch: snapshot.session_epoch,
            intents,
        })
        .map_err(|e| color_eyre::eyre::eyre!("canonical apply: {e}"))?;
    ui_info!(
        "translate-applied",
        applied = apply_res.applied,
        refused = apply_res.skipped.len()
    );

    // Fold the per-intent refusals back onto their rows (skipped.index
    // addresses the intents array; intent_rows maps positions to rows).
    for s in &apply_res.skipped {
        if let Some(&row_idx) = intent_rows.get(s.index) {
            rows[row_idx].apply_status = ApplyStatus::Refused {
                code: format!("{:?}", s.code),
                message: s.message.clone(),
            };
        }
    }

    // 7) Canonical JSON results report.
    let mut results: Vec<TranslateResultRow> = Vec::with_capacity(rows.len());
    for row in &rows {
        let (status, refused) = match &row.apply_status {
            ApplyStatus::Applied => ("applied", None),
            ApplyStatus::Refused { code, message } => {
                ("refused", Some((code.clone(), message.clone())))
            }
            ApplyStatus::Failed => ("failed", None),
        };
        let (refused_code, refused_message) = match refused {
            Some((c, m)) => (Some(c), Some(m)),
            None => (None, None),
        };
        results.push(TranslateResultRow {
            key: &row.key,
            identity: row.identity.clone(),
            locale: &locale_dir,
            source: &row.source,
            target: row.target.as_deref(),
            status,
            provenance: TranslateProvenanceJson {
                provider: &provider,
                origin: "llm",
                kind: row.kind.clone(),
                def_type: row.def_type.as_deref(),
                file: row.file.as_deref(),
                selected_by: row.selected_by.as_deref(),
                engine_status: &row.engine_status,
                refused_code,
                refused_message,
                error: row.error.as_deref(),
            },
        });
    }

    let report = TranslateResultsJson {
        schema_version: RESULTS_SCHEMA_VERSION,
        generator: "rimloc translate",
        project_id: &snapshot.project_id,
        locale: &locale_dir,
        provider: &provider,
        results,
        summary: TranslateSummaryJson {
            translated: summary.translated,
            applied: apply_res.applied,
            refused: apply_res.skipped.len(),
            failed: summary.failed,
            skipped_done: summary.skipped_done,
        },
    };
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(&report)?;
    rimloc_services::write_atomic(&out_json, &json)?;
    ui_info!(
        "translate-json-saved",
        path = out_json.display().to_string()
    );

    // 8) Legacy adapter artifact (explicit opt-in only): the PO file the
    //    old pipeline fed to import-po/build-mod. Canonical state stays
    //    the source of truth; this is a serialization of it.
    if emit_po {
        let mut po = String::new();
        po.push_str("msgid \"\"\nmsgstr \"\"\n\"X-RimLoc-Generator: rimloc translate\\n\"\n\n");
        for row in &rows {
            let Some(tr) = &row.target else {
                continue;
            };
            let esc = |s: &str| {
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            };
            po.push_str(&format!("msgctxt \"{}\"\n", esc(&row.key)));
            po.push_str(&format!("msgid \"{}\"\n", esc(&row.source)));
            po.push_str(&format!("msgstr \"{}\"\n\n", esc(tr)));
        }
        if let Some(parent) = out_po.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rimloc_services::write_atomic(&out_po, po.as_bytes())?;
        ui_info!("translate-po-saved", path = out_po.display().to_string());
    }

    ui_info!(
        "translate-summary",
        translated = summary.translated,
        failed = summary.failed,
        skipped = summary.skipped_done
    );
    if summary.failed > 0 {
        let sample = failures
            .values()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        tracing::warn!(event = "translate_failures", count = failures.len(), sample = %sample);
        std::process::exit(2);
    }
    Ok(())
}

/// The shared canonical managed-projects store: the SAME default the GUI
/// uses, so a CLI-translated project opens in the GUI project list and
/// vice versa. `RIMLOC_DATA_DIR/managed` overrides (deployment sandboxes);
/// without OS data-dir support the system temp dir is the honest fallback —
/// never silently write next to the CWD.
fn default_managed_root() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("RIMLOC_DATA_DIR") {
        if !dir.trim().is_empty() {
            return std::path::PathBuf::from(dir).join("managed");
        }
    }
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("com.rimloc.gui")
        .join("managed")
}
