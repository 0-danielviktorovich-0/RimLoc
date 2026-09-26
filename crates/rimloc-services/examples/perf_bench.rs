//! Performance measurement harness (mandate §29, 2026-09-27): backend session
//! timings over a large synthetic mod (12k Keyed + 3k DefInjected).
//!
//! Uses ONLY the public session API — the same seam the Tauri contract
//! adapter wraps — so numbers here describe the GUI backend directly.
//!
//! Phases:
//!   create  — `create()` from a mod root (scan + fingerprint + persist).
//!             Mints a fresh project each run; prints its id.
//!   open    — fresh-process cold `open(project_id)` (disk load + drift
//!             verdict), then 5 in-memory `snapshot()` calls.
//!   full    — create → open → snapshot x5 → apply x3 (batch of 100 Keyed
//!             intents) → validate x3 → export. One process, all PERF lines.
//!
//! Timing is wall-clock via std::time::Instant; each op prints per-run and
//! median ms. External wrapper measures process RSS separately.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use rimloc_domain::canonical::EntryKind;
use rimloc_services::contract::{ApplyIntentsRequest, IntentAction, TranslationIntent};
use rimloc_services::session::ProjectSessionManager;

const APPLY_BATCH: usize = 100;
const SNAPSHOT_RUNS: usize = 5;
const VALIDATE_RUNS: usize = 3;
const APPLY_RUNS: usize = 3;

fn median_ms(runs: &[f64]) -> f64 {
    let mut v = runs.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
}

fn report(op: &str, runs_ms: &[f64]) {
    println!(
        "PERF op={} runs={} median_ms={:.2} min_ms={:.2} max_ms={:.2}",
        op,
        runs_ms.len(),
        median_ms(runs_ms),
        runs_ms.iter().cloned().fold(f64::INFINITY, f64::min),
        runs_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
}

fn timed<T>(op: &str, runs: usize, mut f: impl FnMut() -> T) -> (T, Vec<f64>) {
    let mut times = Vec::with_capacity(runs);
    let mut last = None;
    for _ in 0..runs {
        let t0 = Instant::now();
        last = Some(f());
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let out = last.expect("at least one run");
    report(op, &times);
    (out, times)
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("PERF_FAIL: {msg}");
    ExitCode::FAILURE
}

fn open_manager(managed: &Path) -> Result<ProjectSessionManager, ExitCode> {
    ProjectSessionManager::new(managed).map_err(|e| {
        let _ = fail(&format!("managed root: {e}"));
        ExitCode::FAILURE
    })
}

fn phase_create(mod_root: &Path, managed: &Path) -> Result<String, ExitCode> {
    let mgr = open_manager(managed)?;
    let t0 = Instant::now();
    let snap = mgr
        .create(mod_root, None)
        .map_err(|e| fail(&format!("create: {e}")))?;
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!(
        "PERF op=create_ms={ms:.2} project_id={} entries={} translations={}",
        snap.project_id,
        snap.project.entries.len(),
        snap.project.translations.len()
    );
    Ok(snap.project_id)
}

fn phase_open(managed: &Path, project_id: &str) -> Result<(), ExitCode> {
    let mgr = open_manager(managed)?;
    let (snap, open_times) = timed("open_cold_disk", 1, || mgr.open(project_id));
    let snap = snap.map_err(|e| fail(&format!("open: {e}")))?;
    let _ = open_times;
    println!(
        "PERF entries={} translations={}",
        snap.project.entries.len(),
        snap.project.translations.len()
    );
    let (_, snap_times) = timed("snapshot_inproc", SNAPSHOT_RUNS, || {
        mgr.snapshot(&snap.project_id).map(|s| s.project.entries.len())
    });
    let _ = snap_times;
    Ok(())
}

fn intents_for(snap: &rimloc_services::contract::ProjectSnapshot, batch: usize) -> Vec<TranslationIntent> {
    snap.project
        .entries
        .iter()
        .filter(|e| e.id.kind == EntryKind::Keyed)
        .take(batch)
        .map(|e| TranslationIntent {
            entry: e.id.clone(),
            locale: "Russian".into(),
            action: IntentAction::SetTranslation,
            text: Some(e.text.clone()),
        })
        .collect()
}

fn phase_full(mod_root: &Path, managed: &Path, batch: usize) -> Result<(), ExitCode> {
    // create (scan + persist) — fresh project per run of this binary
    let t0 = Instant::now();
    let mgr = open_manager(managed)?;
    let snap = mgr
        .create(mod_root, None)
        .map_err(|e| fail(&format!("create: {e}")))?;
    report("create_scan_persist", &[t0.elapsed().as_secs_f64() * 1000.0]);

    // cold open from disk (fresh registry entry for the id is present, so
    // emulate the restart path by opening a NEW manager instance)
    let cold = open_manager(managed)?;
    let (opened, _) = timed("open_cold_disk", 1, || cold.open(&snap.project_id));
    let snap = opened.map_err(|e| fail(&format!("open: {e}")))?;

    // in-memory snapshots — the GUI table refresh backend path
    timed("snapshot_inproc", SNAPSHOT_RUNS, || {
        mgr.snapshot(&snap.project_id).map(|s| s.project.entries.len())
    });

    // apply batch (save) — realistic GUI batch edit of 100 Keyed entries
    let mut rev = snap.revision;
    let mut epoch = snap.session_epoch;
    let mut apply_runs = Vec::new();
    let mut applied_total = 0usize;
    for _ in 0..APPLY_RUNS {
        let intents = intents_for(&snap, batch);
        let req = ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: rev,
            session_epoch: epoch,
            intents,
        };
        let t = Instant::now();
        let r = mgr.apply(&req).map_err(|e| fail(&format!("apply: {e}")))?;
        apply_runs.push(t.elapsed().as_secs_f64() * 1000.0);
        rev = r.revision;
        epoch = snap.session_epoch; // epoch stable within process
        applied_total += r.applied;
    }
    report(&format!("apply_batch{batch}"), &apply_runs);
    println!("PERF applied_total={applied_total}");

    // validate — read-only, the GUI validate button backend
    let mut val_runs = Vec::new();
    let mut val_summary = String::new();
    for _ in 0..VALIDATE_RUNS {
        let t = Instant::now();
        let rep = mgr
            .validate_project(&snap.project_id, epoch, Some("Russian"))
            .map_err(|e| fail(&format!("validate: {e}")))?;
        val_runs.push(t.elapsed().as_secs_f64() * 1000.0);
        val_summary = format!(
            "status={} errors={} warnings={} info={} findings={}",
            rep.status, rep.error_count, rep.warning_count, rep.info_count, rep.findings.len()
        );
    }
    report("validate", &val_runs);
    println!("PERF validate_summary {val_summary}");

    // export — write project back to a mod tree
    let out = PathBuf::from("/tmp/rimloc-perf/export-out");
    let t = Instant::now();
    let exp = mgr
        .export_project(&snap.project_id, epoch, &out, "Russian")
        .map_err(|e| fail(&format!("export: {e}")))?;
    report("export_project", &[t.elapsed().as_secs_f64() * 1000.0]);
    println!(
        "PERF export files_written={} reparsed_keys={} skipped_unknown={}",
        exp.files_written, exp.reparsed_keys, exp.skipped_unknown_type.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let phase = match args.next() {
        Some(p) => p,
        None => return fail("usage: perf_bench <create|open|full> --mod <root> --managed <dir> [--id <project_id>]"),
    };
    let mut mod_root: Option<PathBuf> = None;
    let mut managed: Option<PathBuf> = None;
    let mut id: Option<String> = None;
    let mut batch: usize = APPLY_BATCH;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--mod" => mod_root = args.next().map(PathBuf::from),
            "--managed" => managed = args.next().map(PathBuf::from),
            "--id" => id = args.next(),
            "--batch" => batch = args.next().and_then(|v| v.parse().ok()).unwrap_or(APPLY_BATCH),
            other => return fail(&format!("unknown arg {other}")),
        }
    }
    let managed = match managed {
        Some(m) => m,
        None => return fail("--managed required"),
    };
    let res = match phase.as_str() {
        "create" => {
            let Some(m) = mod_root else { return fail("--mod required") };
            phase_create(&m, &managed).map(|id| println!("PERF_LAST_ID={id}"))
        }
        "open" => {
            let Some(i) = id else { return fail("--id required") };
            phase_open(&managed, &i)
        }
        "full" => {
            let Some(m) = mod_root else { return fail("--mod required") };
            phase_full(&m, &managed, batch)
        }
        other => return fail(&format!("unknown phase {other}")),
    };
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}
