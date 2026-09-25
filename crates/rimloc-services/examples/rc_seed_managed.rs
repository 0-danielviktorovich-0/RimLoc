//! Game-acceptance prep (night worker, 2026-09-26): backend proxy for the
//! built-GUI owner journey. Uses ONLY the public session API (the same seam
//! the Tauri contract adapter wraps):
//!
//! 1. Seed the GUI's managed root with a REAL contract project created from
//!    the production mod 1814383360 (read-only scan) — lets the owner open
//!    it in the built GUI with a single click and lets the night automation
//!    drive the click-only part of the journey (open → workspace → validate).
//! 2. Prove the backend create→export pipeline on a /tmp COPY of the same
//!    mod (the real mod is never written; export out_dir is outside it).
//!
//! NOT committed — temporary acceptance-prep artifact. Delete after use.

use std::path::PathBuf;
use std::process::ExitCode;

use rimloc_services::session::ProjectSessionManager;

const REAL_MOD: &str = "/Applications/RimWorld.app/Mods/1814383360";
const GUI_MANAGED: &str =
    "/Users/danielviktorovich/Library/Application Support/com.rimloc.gui/managed";
const COPY_MANAGED: &str = "/tmp/rimloc-accept-managed";
const EXPORT_OUT: &str = "/tmp/rimloc-accept-export-out";

fn fail(msg: &str) -> ExitCode {
    eprintln!("SEED_FAIL: {msg}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    // ---- 0. Probe-only mode: fresh-process list() over the EXISTING seed.
    // Verifies the dbae486 disk-scan fix on the real managed file WITHOUT
    // creating another project (run: rc_seed_managed --probe).
    if std::env::args().any(|a| a == "--probe") {
        let Ok(mgr) = ProjectSessionManager::new(GUI_MANAGED) else {
            return fail("probe: cannot open GUI managed root");
        };
        let list = mgr.list();
        println!("PROBE list_count={}", list.len());
        for s in &list {
            println!(
                "PROBE item id={} name={} rev={} tv={:?}",
                s.project_id, s.name, s.revision, s.target_version
            );
        }
        let report = mgr.list_report();
        println!("PROBE unloadable={}", report.unloadable.len());
        for u in &report.unloadable {
            println!("PROBE unloadable id={} reason={}", u.project_id, u.reason);
        }
        return if list.is_empty() && report.unloadable.is_empty() {
            fail("probe: managed root lists nothing")
        } else {
            ExitCode::SUCCESS
        };
    }

    // ---- 1. Seed the GUI managed root from the REAL production mod (read-only) ----
    let Ok(gui_mgr) = ProjectSessionManager::new(GUI_MANAGED) else {
        return fail("cannot open GUI managed root");
    };
    let snap = match gui_mgr.create(std::path::Path::new(REAL_MOD), None) {
        Ok(s) => s,
        Err(e) => return fail(&format!("create: {e}")),
    };
    println!(
        "SEEDED project_id={} revision={} epoch={} dirty={} acked_revision={} entries={} translations={}",
        snap.project_id,
        snap.revision,
        snap.session_epoch,
        snap.dirty,
        snap.acked_revision,
        snap.project.entries.len(),
        snap.project.translations.len(),
    );
    let summaries = gui_mgr.list();
    println!("GUI_MANAGED_LIST={summaries:?}");

    // Read-only validate over the same session (never writes).
    match gui_mgr.validate_project(&snap.project_id, snap.session_epoch, None) {
        Ok(rep) => {
            println!(
                "VALIDATE status={} errors={} warnings={} info={} findings={} job={}",
                rep.status,
                rep.error_count,
                rep.warning_count,
                rep.info_count,
                rep.findings.len(),
                rep.job_id
            );
            for f in rep.findings.iter().take(5) {
                println!(
                    "  finding [{}] {} {}: {}",
                    f.severity, f.kind, f.key, f.message
                );
            }
        }
        Err(e) => return fail(&format!("validate: {e}")),
    }

    // ---- 2. create→export proof on a /tmp COPY (real mod untouched) ----
    let copy_root = PathBuf::from("/tmp/rimloc-accept-mod-copy/1814383360");
    let Ok(copy_mgr) = ProjectSessionManager::new(COPY_MANAGED) else {
        return fail("cannot open copy managed root");
    };
    let csnap = match copy_mgr.create(&copy_root, None) {
        Ok(s) => s,
        Err(e) => return fail(&format!("copy create: {e}")),
    };
    println!(
        "COPY_CREATED project_id={} entries={} translations={}",
        csnap.project_id,
        csnap.project.entries.len(),
        csnap.project.translations.len(),
    );

    // Edit like the GUI does: one set_translation intent (Keyed entry),
    // then export with REAL content and reparse verification.
    let entry = csnap
        .project
        .entries
        .iter()
        .find(|e| e.id.kind == rimloc_domain::canonical::EntryKind::Keyed)
        .map(|e| e.id.clone());
    let Some(entry) = entry else {
        return fail("copy project has no Keyed entry to edit");
    };
    let intents = vec![rimloc_services::contract::TranslationIntent {
        entry: entry.clone(),
        locale: "Russian".into(),
        action: rimloc_services::contract::IntentAction::SetTranslation,
        text: Some("Приёмочное тестирование RimLoc".into()),
    }];
    let req = rimloc_services::contract::ApplyIntentsRequest {
        project_id: csnap.project_id.clone(),
        expected_revision: csnap.revision,
        session_epoch: csnap.session_epoch,
        intents,
    };
    match copy_mgr.apply(&req) {
        Ok(arsp) => println!(
            "APPLY applied={} skipped={} revision={}",
            arsp.applied,
            arsp.skipped.len(),
            arsp.revision,
        ),
        Err(e) => return fail(&format!("apply: {e}")),
    }

    match copy_mgr.export_project(
        &csnap.project_id,
        csnap.session_epoch,
        std::path::Path::new(EXPORT_OUT),
        "Russian",
    ) {
        Ok(exp) => println!(
            "EXPORT job={} out_dir={} files_written={} reparsed_keys={} skipped_unknown={}",
            exp.job_id,
            exp.out_dir.path,
            exp.files_written,
            exp.reparsed_keys,
            exp.skipped_unknown_type.len(),
        ),
        Err(e) => return fail(&format!("export: {e}")),
    }
    ExitCode::SUCCESS
}
