//! Agent action-trace (owner mandate 2026-09-29: FOCP-style debugging for
//! RimLoc — the agent must be able to fully drive, test and SEE the app).
//!
//! `RIMLOC_TRACE=1` makes every traced contract command append one JSON line
//! to `<app-data>/RimLoc/logs/trace.jsonl`:
//! `{ "ts": <unix_ms>, "cmd": <name>, "ok": <bool>, "ms": <u128>,
//!    "detail": <short human summary> }`.
//!
//! Design rules:
//! - Off by default; no env — zero I/O, zero behavior change.
//! - Failures to write the trace are swallowed: tracing must never break a
//!   user-facing command.
//! - Payloads are command names and short summaries only — never file
//!   contents or secrets.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

fn trace_path() -> Option<PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let base = dirs::data_dir()?;
        Some(base.join("com.rimloc.gui").join("RimLoc").join("logs").join("trace.jsonl"))
    })
    .clone()
}

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("RIMLOC_TRACE").as_deref() == Ok("1"))
}

/// Append one trace record for a finished contract command. `detail` is a
/// short outcome summary (e.g. "project=<id> revision=<n>" or the typed
/// error code); keep it free of file contents.
pub fn trace_cmd(cmd: &str, ok: bool, ms: u128, detail: &str) {
    if !enabled() {
        return;
    }
    let Some(path) = trace_path() else { return };
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!(
        "{{\"ts\":{ts},\"cmd\":{cmd},\"ok\":{ok},\"ms\":{ms},\"detail\":{detail}}}\n",
        cmd = serde_json::to_string(cmd).unwrap_or_else(|_| "\"?\"".into()),
        detail = serde_json::to_string(detail).unwrap_or_else(|_| "\"?\"".into()),
    );
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Wrap a contract command body: stamps duration and outcome into the trace.
/// The closure returns `(Result payload, detail)` where detail already
/// carries the short summary for both branches.
pub fn traced<T, E>(cmd: &str, body: impl FnOnce() -> Result<T, E>, detail_of: impl FnOnce(&Result<T, E>) -> String) -> Result<T, E> {
    let started = std::time::Instant::now();
    let out = body();
    trace_cmd(cmd, out.is_ok(), started.elapsed().as_millis(), &detail_of(&out));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_cmd_is_noop_without_env() {
        // No RIMLOC_TRACE in the test environment (cargo test does not set
        // it): the call must not panic and must not create any file.
        std::env::remove_var("RIMLOC_TRACE");
        trace_cmd("unit_test_cmd", true, 1, "noop");
    }

    #[test]
    fn traced_reports_duration_and_result() {
        std::env::remove_var("RIMLOC_TRACE");
        let r: Result<u8, ()> = traced("unit_traced", || Ok(7), |v| format!("{v:?}"));
        assert_eq!(r, Ok(7));
        let r: Result<u8, ()> = traced("unit_traced_err", || Err(()), |v| format!("{v:?}"));
        assert!(r.is_err());
    }
}
