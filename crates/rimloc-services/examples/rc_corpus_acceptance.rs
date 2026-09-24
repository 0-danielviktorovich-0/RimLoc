//! Real-corpus acceptance harness (READ-ONLY sources; W-corpus task).
//!
//! Builds canonical projects over explicitly passed source roots and checks
//! the stable public pipeline end-to-end:
//!   build_project → inventory/context counts → effective-location reality →
//!   view honesty → serialize/save/reload equality (BEFORE import) →
//!   existing-pack import (origin/locale/bookkeeping) → save/reload equality
//!   (AFTER import) → native write_rimworld_translation into an isolated
//!   artifact tree → re-parse via the EXISTING scanner and validator with a
//!   BOTH-WAYS identity comparison (lost/invented/corrupted/duplicated
//!   output all fail).
//!
//! Safety contract:
//! - source roots are opened READ-ONLY (the harness never writes there);
//! - outputs go ONLY into a unique directory under `/tmp` (verified), created
//!   AFTER arg validation; the path is refused when it resolves under any
//!   source root, the game bundle, or the repo's `test/` fixtures;
//! - the harness never installs a translation and never launches the game;
//! - run `testlab/scripts/verify-source-hashes.sh` BEFORE and AFTER any
//!   real-source run (never with `--update`).
//!
//! Gate semantics: parser crashes, lost canonical entries, effective/other
//! contexts missing on disk, roundtrip inequality, import/write/reparse
//! errors and duplicate output identities are FAILURES (non-zero exit).
//! Human source-pack defects (validator findings, empty entries, missing
//! packs, a target-only mod with no English source) are REPORTED as
//! evidence, never invented into expectations.
//!
//! Usage:
//!   rc_corpus_acceptance --mod <PATH> [--mod <PATH> …] [--synthetic]
//!
//! Locale/version are the fixed acceptance values (Russian / 1.6) — no
//! free-form CLI values reach the native writer.
//!
//! Exit codes: 0 all gates passed · 1 gate failure · 2 harness misuse.

use rimloc_domain::canonical::{EntryKind, Origin, PatchStage, Project, ViewLabel};
use rimloc_services::project::{
    apply_existing_translation, build_project, provenance_summary, write_rimworld_translation,
};
use rimloc_services::project_store::{load_project, save_project, serialize_project};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const LANG: &str = "Russian";
const VERSION: &str = "1.6";

fn parse_args() -> Result<Vec<PathBuf>, String> {
    let mut mods = Vec::new();
    let mut synthetic = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--mod" => {
                let v = it.next().ok_or("--mod requires a path")?;
                mods.push(PathBuf::from(v));
            }
            "--synthetic" => synthetic = true,
            "--help" | "-h" => {
                eprintln!(
                    "usage: rc_corpus_acceptance --mod <PATH> [--mod <PATH> …] [--synthetic]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if synthetic {
        mods.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../test/ProvenanceMod")
                .canonicalize()
                .map_err(|e| format!("synthetic fixture not found: {e}"))?,
        );
    }
    if mods.is_empty() {
        return Err("no --mod roots passed (explicit roots only)".into());
    }
    for m in &mods {
        if !m.is_dir() {
            return Err(format!("mod root is not a directory: {}", m.display()));
        }
    }
    Ok(mods)
}

fn main() {
    let mods = match parse_args() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    match run(mods) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

/// Create the unique artifact root strictly under the verified `/tmp`
/// contract. Nothing else is created before the protected-root checks.
fn create_artifact_root() -> Result<PathBuf, String> {
    let tmp = PathBuf::from("/tmp");
    if !tmp.is_dir() {
        return Err("/tmp is not available".into());
    }
    let tmp_canon = tmp.canonicalize().map_err(|e| e.to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut attempt = 0u32;
    loop {
        let candidate = tmp_canon.join(format!(
            "rimloc-corpus-run-{ts}-{}-{attempt}",
            std::process::id()
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return candidate.canonicalize().map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt > 32 {
                    return Err("cannot allocate a unique artifact directory".into());
                }
            }
            Err(e) => return Err(format!("cannot create artifact dir: {e}")),
        }
    }
}

fn run(mods: Vec<PathBuf>) -> Result<(), String> {
    let artifact = create_artifact_root()?;

    // Protected roots: game bundle, every source root, the repo fixture tree.
    let mut protected: Vec<PathBuf> = vec![PathBuf::from("/Applications/RimWorld.app")];
    for m in &mods {
        protected.push(m.canonicalize().map_err(|e| e.to_string())?);
    }
    protected.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("/nonexistent-test-dir")),
    );
    for prot in &protected {
        if artifact.starts_with(prot) || prot.starts_with(&artifact) {
            return Err(format!(
                "refusing artifact root overlapping a protected source root: {} vs {}",
                artifact.display(),
                prot.display()
            ));
        }
    }

    let mut mods_report = Vec::new();
    let mut any_failed = false;
    for (i, m) in mods.iter().enumerate() {
        let name = m
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("mod-{i}"));
        let mod_artifact = artifact.join(format!("mod-{i:02}-{name}"));
        std::fs::create_dir_all(&mod_artifact)
            .map_err(|e| format!("cannot create mod artifact dir: {e}"))?;
        let report = accept_mod(m, i, &name, &mod_artifact);
        any_failed |= !report.ok;
        mods_report.push(report.summary);
    }

    let status = if any_failed { "fail" } else { "pass" };
    let summary = serde_json::json!({
        "status": status,
        "target_version": VERSION,
        "import_language": LANG,
        "artifact_root": artifact.display().to_string(),
        "mods": mods_report,
    });
    let summary_path = artifact.join("summary.json");
    let bytes =
        serde_json::to_vec_pretty(&summary).map_err(|e| format!("cannot render summary: {e}"))?;
    std::fs::write(&summary_path, bytes).map_err(|e| format!("cannot write summary: {e}"))?;
    println!(
        "{}",
        serde_json::to_string(&summary).map_err(|e| format!("cannot render stdout report: {e}"))?
    );
    let artifact_line = format!("artifact: {}", artifact.display());
    println!("{artifact_line}");
    if any_failed {
        // Gate failures are the CONTRACTED non-zero outcome (exit 1);
        // misuse errors take exit 2 through the caller.
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Debug)]
struct ModReport {
    ok: bool,
    summary: serde_json::Value,
}

fn kind_name(k: EntryKind) -> &'static str {
    match k {
        EntryKind::Keyed => "Keyed",
        EntryKind::DefInjected => "DefInjected",
        EntryKind::TKey => "TKey",
        EntryKind::Strings => "Strings",
        EntryKind::Backstories => "Backstories",
        EntryKind::PatchDerived => "PatchDerived",
    }
}

/// Def type the native writer will use for this entry (mirrors the CURRENT
/// documented output rule in `write_rimworld_translation`, af937fd):
/// id discriminator → TKey metadata → `/DefInjected/` segment of a context
/// file → NONE (the entry is SKIPPED from output and reported — never a
/// "Misc" guess). Canonical IDs only.
fn writer_def_type(e: &rimloc_domain::canonical::SourceEntry) -> Option<String> {
    if let Some(dt) = &e.id.def_type {
        if !dt.trim().is_empty() {
            return Some(dt.clone());
        }
    }
    if let Some(m) = &e.tkey {
        return Some(m.def_type.clone());
    }
    for c in &e.contexts {
        let s = c.file.replace('\\', "/");
        if let Some(i) = s.find("/DefInjected/") {
            let seg = s[i + "/DefInjected/".len()..]
                .split('/')
                .next()
                .unwrap_or_default();
            if !seg.is_empty() {
                return Some(seg.to_string());
            }
        }
    }
    None
}

/// Output identity the writer produces for one translation: (def-type scope,
/// key as it re-parses). Keyed → ("Keyed", key); DefInjected → (def_type,
/// key); TKey → (tkey def_type, key + suffix) — the single serialized key,
/// never alias and suffix simultaneously.
fn output_identity(e: &rimloc_domain::canonical::SourceEntry) -> Option<(String, String)> {
    match e.id.kind {
        EntryKind::Keyed => Some(("Keyed".into(), e.id.key.clone())),
        // Unknown def type: the writer skips the entry (af937fd) — the
        // expected model must skip it too, never render a "Misc" scope.
        EntryKind::DefInjected => writer_def_type(e).map(|dt| (dt, e.id.key.clone())),
        EntryKind::TKey => {
            let m = e.tkey.as_ref()?;
            Some((m.def_type.clone(), format!("{}{}", e.id.key, m.suffix)))
        }
        // Strings/Backstories/PatchDerived writers land with their own gates.
        _ => None,
    }
}

fn scope_of_unit(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    if let Some(i) = s.find("/Keyed/") {
        let _ = i;
        return "Keyed".into();
    }
    if let Some(i) = s.find("/DefInjected/") {
        let seg = s[i + "/DefInjected/".len()..]
            .split('/')
            .next()
            .unwrap_or_default();
        if !seg.is_empty() {
            return seg.to_string();
        }
    }
    "unknown".into()
}

fn accept_mod(mod_root: &Path, idx: usize, name: &str, mod_artifact: &Path) -> ModReport {
    let mut failures: Vec<String> = Vec::new();
    let fail = |failures: &mut Vec<String>, msg: String| {
        failures.push(format!("{name}: {msg}"));
    };

    // G1 — canonical build through the stable pipeline (never scan_auto).
    let mut project: Project = match build_project(mod_root, Some(VERSION)) {
        Ok(p) => p,
        Err(e) => {
            fail(&mut failures, format!("G1 build_project failed: {e}"));
            return ModReport {
                ok: false,
                summary: serde_json::json!({
                    "mod": name, "ok": false,
                    "gate_failures": failures,
                    "gate": "G1", "error": e.to_string(),
                }),
            };
        }
    };

    // G2 — inventory / kind / context counts (evidence; no guessed targets).
    let mut by_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in &project.entries {
        *by_kind.entry(kind_name(e.id.kind)).or_default() += 1;
    }
    let context_count: usize = project.entries.iter().map(|e| e.contexts.len()).sum();
    let provenance = provenance_summary(&project);

    // G3 — EVERY promised context (Effective and Overridden) points at a
    // real file on disk; lines are honest 1-based-or-None.
    let mut context_gaps: Vec<String> = Vec::new();
    let mut checked_contexts = 0usize;
    for e in &project.entries {
        if e.contexts.is_empty() {
            context_gaps.push(format!("{}: no contexts at all", e.id.key));
            continue;
        }
        for c in &e.contexts {
            checked_contexts += 1;
            if !Path::new(&c.file).is_file() {
                context_gaps.push(format!(
                    "{} ({:?}): context file missing on disk: {}",
                    e.id.key, c.role, c.file
                ));
            }
            if let Some(l) = c.line {
                if l == 0 {
                    context_gaps.push(format!("{}: line 0 is not a valid 1-based line", e.id.key));
                }
            }
        }
    }
    if !context_gaps.is_empty() {
        fail(
            &mut failures,
            format!(
                "G3 {} canonical context(s) are not real on-disk locations (of {checked_contexts} checked): {:?}",
                context_gaps.len(),
                context_gaps.iter().take(5).cloned().collect::<Vec<_>>()
            ),
        );
    }

    // G3b — empty inventory is a failure when the mod carries source-side
    // content; a target-only mod legitimately has no English source.
    // Source-side = Defs or an English Languages dir. A LoadFolders.xml
    // alone does NOT imply source content: pure translation mods ship
    // LoadFolders + Languages/<target> only, and their canonical inventory
    // is legitimately empty (013-2 contract) — misclassifying them as
    // source mods made G3b fire on RU-only packs.
    let has_source_side =
        mod_root.join("Defs").is_dir() || mod_root.join("Languages").join("English").is_dir();
    if project.entries.is_empty() && has_source_side {
        fail(
            &mut failures,
            "G3b inventory is EMPTY although the mod ships source-side content (Defs or English Languages) — canonical scan lost everything".into(),
        );
    }

    // G4 — view honesty, checked on the model itself.
    if project.context.target_version.as_deref() != Some(VERSION) {
        fail(
            &mut failures,
            format!(
                "G4 context.target_version={:?} != requested {VERSION}",
                project.context.target_version
            ),
        );
    }
    let view = project.context.view;
    if view == ViewLabel::Exact {
        let any_conditional = project
            .entries
            .iter()
            .any(|e| e.provenance.conditional_branch);
        let any_partial = project
            .entries
            .iter()
            .any(|e| e.provenance.patch_stage == PatchStage::Partial);
        if any_conditional || any_partial {
            fail(
                &mut failures,
                format!(
                    "G4 EXACT view with unresolved provenance: conditional={any_conditional}, partial={any_partial}"
                ),
            );
        }
    }
    let mut version_selected_seen: BTreeMap<String, usize> = BTreeMap::new();
    for e in &project.entries {
        if let Some(v) = &e.provenance.version_selected {
            *version_selected_seen.entry(v.clone()).or_default() += 1;
        }
    }

    // G5 — pre-import serialize/save/reload equality.
    let pre_path = mod_artifact.join("project.pre-import.rimloc.json");
    roundtrip_check(&project, &pre_path, "G5", name, &mut failures);

    // G6 — import the existing target-language pack when the mod ships one.
    let pack_root = mod_root.join("Languages").join(LANG);
    let pack_present = pack_root.is_dir();
    let mut imported = 0usize;
    let mut import_texts: Vec<serde_json::Value> = Vec::new();
    if pack_present {
        let before = project.translations.len();
        match apply_existing_translation(&mut project, &pack_root, LANG) {
            Ok(n) => {
                imported = n;
                let added = &project.translations[before..];
                if added.len() != imported {
                    fail(
                        &mut failures,
                        format!(
                            "G6 import bookkeeping: applied {imported} but {} records appended",
                            added.len()
                        ),
                    );
                }
                for t in added {
                    if t.origin != Origin::Imported {
                        fail(
                            &mut failures,
                            format!(
                                "G6 imported record {} has origin {:?}, expected Imported",
                                t.source_id.key, t.origin
                            ),
                        );
                    }
                    if t.locale != LANG {
                        fail(
                            &mut failures,
                            format!(
                                "G6 imported record {} has locale {:?}, expected {LANG}",
                                t.source_id.key, t.locale
                            ),
                        );
                    }
                    if import_texts.len() < 10 {
                        import_texts.push(serde_json::json!({
                            "key": t.source_id.key,
                            "locale": t.locale,
                            "origin": format!("{:?}", t.origin),
                            "text": t.text.as_deref().map(|s| s.chars().take(120).collect::<String>()),
                        }));
                    }
                }
                if imported > project.entries.len() {
                    fail(
                        &mut failures,
                        format!(
                            "G6 imported {imported} > inventory {}",
                            project.entries.len()
                        ),
                    );
                }
            }
            Err(e) => fail(&mut failures, format!("G6 import failed: {e}")),
        }
    }

    // G7 — post-import roundtrip: imported origin/locale/text must survive
    // save/reload byte-identically before anything is generated from it.
    let post_path = mod_artifact.join("project.post-import.rimloc.json");
    roundtrip_check(&project, &post_path, "G7", name, &mut failures);
    // Generate from the REOPENED project, per the acceptance contract.
    let generation_source: Project = match load_project(&post_path) {
        Ok(p) => p,
        Err(e) => {
            fail(
                &mut failures,
                format!("G7 reload for generation failed: {e}"),
            );
            project.clone()
        }
    };

    // G8 — native write into the ISOLATED artifact tree, then re-parse with
    // the existing scanner and compare BOTH WAYS by output identity.
    let out_gen = mod_artifact.join("generated");
    let write_res = write_rimworld_translation(
        &generation_source,
        &out_gen,
        LANG,
        &format!("RimLoc corpus acceptance — {name}"),
        &format!("RimLoc.corpus.{name}.{idx}"),
        VERSION,
    );
    let (files_written, mut reparsed, mut validate_findings) = match write_res {
        Ok(_) => (count_files(&out_gen), 0usize, Vec::new()),
        Err(e) => {
            fail(&mut failures, format!("G8 native write failed: {e}"));
            (0usize, 0usize, Vec::new())
        }
    };
    let mut expected_outputs: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut skipped_kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    for t in &generation_source.translations {
        if t.locale != LANG {
            continue;
        }
        if t.text
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
        {
            continue;
        }
        if t.lifecycle == rimloc_domain::canonical::Lifecycle::Obsolete {
            continue;
        }
        let Some(e) = generation_source
            .entries
            .iter()
            .find(|e| e.id == t.source_id)
        else {
            continue;
        };
        match output_identity(e) {
            Some(id) => {
                expected_outputs.insert(id, t.text.clone().unwrap_or_default());
            }
            None => *skipped_kinds.entry(kind_name(e.id.kind)).or_default() += 1,
        }
    }
    let mut actual: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut duplicate_identities: Vec<String> = Vec::new();
    if out_gen.is_dir() {
        match rimloc_parsers_xml::scan_keyed_xml(&out_gen) {
            Ok(units) => {
                for u in &units {
                    let id = (scope_of_unit(&u.path), u.key.clone());
                    let text = u.source.clone().unwrap_or_default();
                    if actual.insert(id.clone(), text).is_some() {
                        duplicate_identities.push(format!("{:?}", id));
                    }
                }
                // BOTH-WAYS: lost output, invented output, corrupted text.
                let mut lost: Vec<(String, String)> = Vec::new();
                let mut corrupted: Vec<serde_json::Value> = Vec::new();
                for (id, exp_text) in &expected_outputs {
                    match actual.get(id) {
                        None => lost.push(id.clone()),
                        Some(act) => {
                            if act.trim() != exp_text.trim() {
                                corrupted.push(serde_json::json!({
                                    "scope": id.0, "key": id.1,
                                    "expected": exp_text,
                                    "actual": act,
                                }));
                            }
                        }
                    }
                }
                let invented: Vec<(String, String)> = actual
                    .keys()
                    .filter(|id| !expected_outputs.contains_key(*id))
                    .cloned()
                    .collect();
                if !lost.is_empty() {
                    fail(
                        &mut failures,
                        format!(
                            "G8 LOST generated output ({} identities): {:?}",
                            lost.len(),
                            &lost[..lost.len().min(10)]
                        ),
                    );
                }
                if !invented.is_empty() {
                    fail(
                        &mut failures,
                        format!(
                            "G8 INVENTED generated output not backed by translations: {:?}",
                            &invented[..invented.len().min(10)]
                        ),
                    );
                }
                if !corrupted.is_empty() {
                    fail(
                        &mut failures,
                        format!("G8 generated text differs from canonical translation for {} identity/identities", corrupted.len()),
                    );
                }
                if !duplicate_identities.is_empty() {
                    fail(
                        &mut failures,
                        format!(
                            "G8 duplicate output identities: {:?}",
                            &duplicate_identities[..duplicate_identities.len().min(10)]
                        ),
                    );
                }
                reparsed = units.len();
                // Validator on OUR generated files: errors are reported as
                // evidence; a validator CRASH is a gate failure (never an
                // empty success).
                match rimloc_validate::validate(&units) {
                    Ok(findings) => {
                        validate_findings = findings
                            .iter()
                            .take(50)
                            .map(|m| {
                                serde_json::json!({
                                    "kind": m.kind, "key": m.key, "message": m.message,
                                    "path": m.path, "line": m.line,
                                })
                            })
                            .collect();
                    }
                    Err(e) => fail(
                        &mut failures,
                        format!("G8 validator crashed on generated output: {e}"),
                    ),
                }
            }
            Err(e) => fail(
                &mut failures,
                format!("G8 re-parse of generated output failed: {e}"),
            ),
        }
    }

    // Evidence samples (first effective contexts the inspector would show).
    let samples: Vec<serde_json::Value> = generation_source
        .entries
        .iter()
        .take(10)
        .map(|e| {
            serde_json::json!({
                "key": e.id.key,
                "kind": kind_name(e.id.kind),
                "selected_by": e.provenance.selected_by,
                "version_selected": e.provenance.version_selected,
                "conditional": e.provenance.conditional_branch,
                "patch_stage": format!("{:?}", e.provenance.patch_stage),
                "effective_file": e.contexts.first().map(|c| c.file.clone()),
                "line": e.contexts.first().and_then(|c| c.line),
            })
        })
        .collect();

    let evidence = serde_json::json!({
        "mod": name,
        "root": mod_root.display().to_string(),
        "requested_version": VERSION,
        "view": format!("{view:?}"),
        "context_target_version": generation_source.context.target_version,
        "inventory": {
            "entries": generation_source.entries.len(),
            "contexts": context_count,
            "by_kind": &by_kind,
            "checked_contexts": checked_contexts,
            "provenance_summary": &provenance,
            "version_selected_seen": &version_selected_seen,
        },
        "context_gaps": &context_gaps[..context_gaps.len().min(20)],
        "import": {
            "pack_present": pack_present,
            "pack_root": pack_root.display().to_string(),
            "applied": imported,
            "samples": import_texts,
        },
        "generated": {
            "out_dir": out_gen.display().to_string(),
            "files_written": files_written,
            "reparsed_keys": reparsed,
            "expected_identities": expected_outputs.len(),
            "actual_identities": actual.len(),
            "duplicate_identities": duplicate_identities,
            "skipped_kinds": skipped_kinds,
            "validate_findings": validate_findings,
        },
        "samples": samples,
    });
    let evidence_path = mod_artifact.join("evidence.json");
    if let Err(e) = std::fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&evidence).unwrap_or_else(|_| b"{}".to_vec()),
    ) {
        fail(&mut failures, format!("evidence write failed: {e}"));
    }

    let ok = failures.is_empty();
    ModReport {
        ok,
        summary: serde_json::json!({
            "mod": name,
            "ok": ok,
            "gate_failures": failures,
            "entries": generation_source.entries.len(),
            "by_kind": by_kind,
            "view": format!("{view:?}"),
            "context_gaps": context_gaps.len(),
            "imported": imported,
            "files_written": files_written,
            "reparsed_keys": reparsed,
            "expected_identities": expected_outputs.len(),
            "evidence": evidence_path.display().to_string(),
        }),
    }
}

/// Save → load → re-serialize and compare BOTH the model (PartialEq over
/// every ID/provenance/text) and the deterministic serialized bytes. Every
/// persistence error is recorded as a gate failure.
fn roundtrip_check(
    project: &Project,
    path: &Path,
    gate: &str,
    mod_name: &str,
    failures: &mut Vec<String>,
) {
    let bytes = match serialize_project(project) {
        Ok(b) => b,
        Err(e) => {
            failures.push(format!("{mod_name}: {gate} serialize failed: {e}"));
            return;
        }
    };
    if let Err(e) = save_project(project, path) {
        failures.push(format!("{mod_name}: {gate} save failed: {e}"));
        return;
    }
    let reloaded = match load_project(path) {
        Ok(p) => p,
        Err(e) => {
            failures.push(format!("{mod_name}: {gate} load failed: {e}"));
            return;
        }
    };
    let bytes_after = match serialize_project(&reloaded) {
        Ok(b) => b,
        Err(e) => {
            failures.push(format!("{mod_name}: {gate} re-serialize failed: {e}"));
            return;
        }
    };
    if reloaded != *project {
        failures.push(format!(
            "{mod_name}: {gate} reopen inequality: domain model differs after save/load"
        ));
    }
    if bytes_after != bytes {
        failures.push(format!(
            "{mod_name}: {gate} reopen inequality: serialized bytes differ after save/load"
        ));
    }
}

fn count_files(root: &Path) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                n += count_files(&e.path());
            } else {
                n += 1;
            }
        }
    }
    n
}
