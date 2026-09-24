//! Observability helpers (gate L): structured operation log, secret-aware
//! sanitizer and a sanitized support bundle.
//!
//! Design constraints (docs/development/AUTONOMOUS_STATUS.md §W4.5 п.5-6):
//! a small *relevant* causal trace beats raw logs, and redaction must be
//! automatic and previewable (Included / Redacted / Excluded) while keeping
//! the bundle diagnostically useful. When a bundle is collected because an
//! operation failed, the failing [`OperationLog`] is preserved verbatim —
//! its id, stages and error chains are the causal context and are never
//! replaced by a synthetic success.

use crate::util::write_atomic;
use crate::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Operation ID
// ---------------------------------------------------------------------------

static OP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a ULID-like unique operation id without external dependencies:
/// hex-encoded unix millis (sortable) + per-process counter + pid.
pub fn generate_operation_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let n = OP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("op-{millis:012x}-{n:03}-{pid:x}")
}

fn rfc3339_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// `Instant::now` as a serde `default` path (serde's `skip` needs a way to
/// construct non-`Default` runtime-only fields on deserialization).
fn instant_now() -> Instant {
    Instant::now()
}

// ---------------------------------------------------------------------------
// SHA-256 — standard `sha2` crate (already in the dependency tree)
// ---------------------------------------------------------------------------

/// SHA-256 of `data`, hex-encoded lowercase. Uses the standard `sha2` crate
/// for the support-bundle manifest.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

// ---------------------------------------------------------------------------
// OperationLog
// ---------------------------------------------------------------------------

/// One timed stage of an operation with named counters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageRecord {
    pub stage: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, u64>,
}

/// One error with its full causal chain (`source()` walk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub stage: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chain: Vec<String>,
}

/// Structured journal of a single operation: id, stages, counters, timings
/// and the error chain. Serialized to JSON; persisted with `write_atomic`.
///
/// Lifecycle is explicit and honest: `end_stage` only closes the stage;
/// `finished_at` / `total_duration_ms` are set exclusively by [`Self::finish`].
/// A log serialized before `finish` honestly reports no `finished_at` — a
/// still-running or crashed operation stays looking like one. Calling
/// `begin_stage` after `finish` re-opens the operation (clears the finish
/// markers) rather than silently extending a "finished" run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub operation_id: String,
    pub name: String,
    pub started_at: String,
    #[serde(default)]
    pub stages: Vec<StageRecord>,
    #[serde(default)]
    pub errors: Vec<ErrorRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
    #[serde(skip)]
    stage_starts: BTreeMap<String, Instant>,
    #[serde(skip, default = "instant_now")]
    started: Instant,
}

impl OperationLog {
    /// Start a new operation log with a freshly generated operation id.
    pub fn new(name: &str) -> Self {
        Self {
            operation_id: generate_operation_id(),
            name: name.to_string(),
            started_at: rfc3339_now(),
            stages: Vec::new(),
            errors: Vec::new(),
            finished_at: None,
            total_duration_ms: None,
            stage_starts: BTreeMap::new(),
            started: Instant::now(),
        }
    }

    /// True once `finish` has been called (and no later `begin_stage`
    /// re-opened the operation).
    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    /// Begin (or resume) a named stage. Re-opens a finished operation.
    pub fn begin_stage(&mut self, stage: &str) {
        self.stage_starts.insert(stage.to_string(), Instant::now());
        if let Some(rec) = self.stages.iter_mut().find(|s| s.stage == stage) {
            rec.started_at = rfc3339_now();
            rec.finished_at = None;
            rec.duration_ms = None;
        } else {
            self.stages.push(StageRecord {
                stage: stage.to_string(),
                started_at: rfc3339_now(),
                finished_at: None,
                duration_ms: None,
                counters: BTreeMap::new(),
            });
        }
        // Honest lifecycle: new work means the operation is no longer finished.
        self.finished_at = None;
        self.total_duration_ms = None;
    }

    /// End the named stage, recording its duration. Does NOT mark the
    /// operation finished — only [`Self::finish`] does.
    pub fn end_stage(&mut self, stage: &str) {
        let elapsed = self
            .stage_starts
            .get(stage)
            .map(|t| t.elapsed().as_millis() as u64);
        if let Some(rec) = self.stages.iter_mut().find(|s| s.stage == stage) {
            rec.finished_at = Some(rfc3339_now());
            rec.duration_ms = elapsed;
        }
    }

    /// Record a counter value for a stage (accumulates, last write wins).
    pub fn counter(&mut self, stage: &str, key: &str, value: u64) {
        if let Some(rec) = self.stages.iter_mut().find(|s| s.stage == stage) {
            rec.counters.insert(key.to_string(), value);
        }
    }

    /// Record an error (message + full `source()` chain) for a stage.
    pub fn error(&mut self, stage: &str, err: &dyn std::error::Error) {
        let mut chain = Vec::new();
        let mut src = err.source();
        while let Some(e) = src {
            chain.push(e.to_string());
            src = e.source();
        }
        self.errors.push(ErrorRecord {
            stage: stage.to_string(),
            message: err.to_string(),
            chain,
        });
    }

    /// Record a plain-text error note for a stage (no `Error` object).
    pub fn error_message(&mut self, stage: &str, message: &str) {
        self.errors.push(ErrorRecord {
            stage: stage.to_string(),
            message: message.to_string(),
            chain: Vec::new(),
        });
    }

    /// Mark the operation finished (single source of truth for
    /// `finished_at` / `total_duration_ms`).
    pub fn finish(&mut self) {
        self.finished_at = Some(rfc3339_now());
        self.total_duration_ms = Some(self.started.elapsed().as_millis() as u64);
    }

    /// Derived machine-readable status from lifecycle + recorded errors:
    /// unfinished with errors = `failed`; unfinished without errors =
    /// `running`; finished without errors = `succeeded`; finished with
    /// errors = `failed` (a finished run with errors never claims success).
    /// Derived on demand — never stored — so older serialized logs without
    /// the field stay readable and every export agrees.
    pub fn status(&self) -> OperationStatus {
        match (self.finished_at.is_some(), self.errors.is_empty()) {
            (true, true) => OperationStatus::Succeeded,
            (_, false) => OperationStatus::Failed,
            (false, true) => OperationStatus::Running,
        }
    }

    /// Serialize to a pretty JSON value, injecting the derived `status`.
    pub fn to_value(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(ref mut m) = v {
            m.insert(
                "status".to_string(),
                serde_json::Value::String(self.status().as_str().to_string()),
            );
        }
        v
    }

    /// Serialize to pretty JSON text (derived `status` included).
    pub fn to_json_pretty(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self.to_value())?)
    }

    /// Persist the log as JSON via `write_atomic` (derived `status` included).
    pub fn write_json(&self, path: &Path) -> std::io::Result<()> {
        let mut json =
            serde_json::to_string_pretty(&self.to_value()).unwrap_or_else(|_| "{}".to_string());
        json.push('\n');
        write_atomic(path, json.as_bytes())
    }
}

/// Machine-readable derived status of an [`OperationLog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    Running,
    Succeeded,
    Failed,
}

impl OperationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationStatus::Running => "running",
            OperationStatus::Succeeded => "succeeded",
            OperationStatus::Failed => "failed",
        }
    }
}

// ---------------------------------------------------------------------------
// Sanitizer
// ---------------------------------------------------------------------------

/// Sensitive key-name words: a field whose name contains any of these is
/// redacted automatically. `auth`/`credential` are natural extensions of the
/// mandated [key, token, secret, password, authorization] to cover auth
/// headers and credentials blocks.
const SENSITIVE_KEY_WORDS: [&str; 7] = [
    "key",
    "token",
    "secret",
    "password",
    "authorization",
    "auth",
    "credential",
];

/// Environment variables whose (sanitized) values may enter a bundle: a
/// small explicit list of diagnostically necessary fields only. OS/arch/
/// versions/locales are captured structurally elsewhere, account names
/// (USER/USERNAME) have no diagnostic value in an anonymized bundle, and
/// prefixed families (RIMLOC_*/CARGO_*/RUST_*) may carry unrecognized
/// credentials — everything outside this list is excluded (with the
/// exclusion kept visible in the preview).
const ENV_ALLOWLIST_EXACT: [&str; 4] = ["LANG", "LC_ALL", "TERM", "TMPDIR"];

// Whole-value secret shapes (the entire string IS the secret).
static RE_SK_SECRET: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^sk[-_][A-Za-z0-9_-]{8,}").expect("valid regex"));
static RE_BEARER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^bearer\s+\S+").expect("valid regex"));
static RE_LONG_OPAQUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9+/=_.-]{40,}$").expect("valid regex"));
static RE_HEX_RUN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^[0-9a-f]{32,}$").expect("valid regex"));
static RE_PEM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-----BEGIN [A-Z ]*PRIVATE KEY-----").expect("valid regex"));
static RE_SECRET_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(ghp_|gho_|github_pat_|xox[baprs]-|AKIA)").expect("valid regex"));

// Secret fragments embedded inside free text (masked in place so the
// surrounding diagnostic context survives).
//
// Replacement policy (see `redact_secret_fragments`): a regex with capture
// group 1 must carry ONLY a static label ("Authorization: ", "token: "),
// which survives next to the marker; the credential itself must NEVER be
// inside group 1. Whole-token patterns are non-capturing so the entire
// match is replaced.
static RE_PEM_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----")
        .expect("valid regex")
});
static RE_AUTH_HEADER_MID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(authorization\s*[:=]\s*)(bearer\s+)?\S+").expect("valid regex"));
static RE_KV_SECRET_MID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)\b((?:api[_-]?key|secret|password|passwd|access[_-]?token|auth[_-]?token|token)\s*[=:]\s*)[^\s;&,"']+"#,
    )
    .expect("valid regex")
});
static RE_BEARER_MID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(\bbearer\s)[A-Za-z0-9._\-]{8,}").expect("valid regex"));
static RE_SK_TOKEN_MID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bsk[-_][A-Za-z0-9_-]{8,}").expect("valid regex"));
static RE_PROVIDER_TOKEN_MID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b(?:ghp_[A-Za-z0-9]{20,}|gho_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[A-Z0-9]{16})",
    )
    .expect("valid regex")
});
static RE_HEX_TOKEN_MID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b[0-9a-f]{32,}\b").expect("valid regex"));
// Opaque token fragment: deliberately excludes `/` so filesystem paths and
// URLs are not shredded — path rewriting below handles those instead.
static RE_OPAQUE_TOKEN_MID: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9+_=.-]{40,}\b").expect("valid regex"));

pub const REDACTION_MARKER: &str = "[REDACTED]";

/// Verdict for a single field under the redaction rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldDecision {
    /// Kept (possibly rewritten: secrets inside free text masked, absolute
    /// home paths rewritten to `~`). Carries the final value when it was
    /// rewritten, `None` when the value passes untouched.
    Included(Option<String>),
    /// Key kept, value replaced with a redaction marker.
    Redacted,
    /// Dropped entirely (env vars outside the allowlist).
    Excluded,
}

/// Outcome of `Sanitizer::classify` — the redaction preview. `included`
/// carries the exact values that would be written, so the preview is the
/// truth about the final payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifyReport {
    pub included: Vec<(String, String)>,
    pub redacted: Vec<String>,
    pub excluded: Vec<String>,
}

impl ClassifyReport {
    fn merge(&mut self, other: &ClassifyReport) {
        self.included.extend(other.included.iter().cloned());
        self.redacted.extend(other.redacted.iter().cloned());
        self.excluded.extend(other.excluded.iter().cloned());
    }

    fn dedupe(&mut self) {
        self.included.sort();
        self.included.dedup();
        self.redacted.sort();
        self.redacted.dedup();
        self.excluded.sort();
        self.excluded.dedup();
    }
}

fn home_dir() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("HOME") {
        let p = PathBuf::from(h);
        if p.is_absolute() {
            return Some(p);
        }
    }
    if let Some(h) = std::env::var_os("USERPROFILE") {
        let p = PathBuf::from(h);
        if p.is_absolute() {
            return Some(p);
        }
    }
    None
}

/// Rewrite absolute home paths inside a value to `~` so bundles don't leak
/// filesystem layout. Handles values that embed the home path mid-string
/// (e.g. PATH-like lists). Returns `None` when nothing needed rewriting.
fn sanitize_home_path(value: &str, home: Option<&Path>) -> Option<String> {
    let home_s = home?.to_string_lossy().into_owned();
    if home_s.len() <= 1 {
        return None;
    }
    let mut out = String::with_capacity(value.len());
    let mut changed = false;
    let mut rest = value;
    while let Some(pos) = rest.find(home_s.as_str()) {
        let after = &rest[pos + home_s.len()..];
        // Only rewrite at a path boundary ("/Users/x" must not corrupt
        // "/Users/xyz").
        let boundary_ok = after.is_empty() || after.starts_with('/') || after.starts_with('\\');
        out.push_str(&rest[..pos]);
        if boundary_ok {
            out.push('~');
            changed = true;
        } else {
            out.push_str(&home_s);
        }
        rest = after;
    }
    out.push_str(rest);
    if changed {
        Some(out)
    } else {
        None
    }
}

fn key_name_is_sensitive(key: &str) -> bool {
    // Normalize separators/case, then check substring containment against the
    // sensitive words ("anything похожее на ключ/токен по ключу-слову").
    // Over-redaction is the safe direction; diagnostic keys are unlikely to
    // carry these substrings incidentally.
    let normalized: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    SENSITIVE_KEY_WORDS.iter().any(|s| normalized.contains(s))
}

fn value_looks_secret(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() {
        return false;
    }
    if RE_SK_SECRET.is_match(v)
        || RE_BEARER.is_match(v)
        || RE_PEM.is_match(v)
        || RE_SECRET_PREFIX.is_match(v)
        || RE_HEX_RUN.is_match(v)
    {
        return true;
    }
    // Opaque blobs (JWTs, base64 keys): skip values with path separators —
    // those are handled by path rewriting and must keep diagnostic value.
    if !v.contains('/') && !v.contains('\\') {
        return RE_LONG_OPAQUE.is_match(v);
    }
    false
}

/// Mask secret fragments inside free text, preserving the surrounding
/// diagnostic context ("Authorization: Bearer abc" → "Authorization:
/// [REDACTED]"). Returns `None` when nothing needed masking.
fn redact_secret_fragments(value: &str) -> Option<String> {
    if value_looks_secret(value.trim()) {
        // Whole-value secret: handled by the caller with a full redaction.
        return None;
    }
    let mut masked = value.to_string();
    let mut changed = false;
    for re in [
        &RE_PEM_BLOCK,
        &RE_AUTH_HEADER_MID,
        &RE_KV_SECRET_MID,
        &RE_BEARER_MID,
        &RE_SK_TOKEN_MID,
        &RE_PROVIDER_TOKEN_MID,
        &RE_HEX_TOKEN_MID,
        &RE_OPAQUE_TOKEN_MID,
    ] {
        if re.is_match(&masked) {
            masked = re
                .replace_all(&masked, |caps: &regex::Captures| {
                    // Keep the labelled prefix ("Authorization: "), mask the value.
                    match caps.get(1) {
                        Some(prefix) => format!("{}{}", prefix.as_str(), REDACTION_MARKER),
                        None => REDACTION_MARKER.to_string(),
                    }
                })
                .into_owned();
            changed = true;
        }
    }
    if changed {
        Some(masked)
    } else {
        None
    }
}

fn env_name_allowlisted(name: &str) -> bool {
    let name = name.trim();
    ENV_ALLOWLIST_EXACT
        .iter()
        .any(|a| a.eq_ignore_ascii_case(name))
}

/// Secret-aware sanitizer with an explicit Included/Redacted/Excluded preview.
#[derive(Debug, Clone)]
pub struct Sanitizer {
    /// Marker written in place of redacted values.
    pub redaction_marker: String,
    /// Home directory used for `~` path rewriting; `None` = resolve from env.
    home: Option<PathBuf>,
}

impl Default for Sanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Sanitizer {
    pub fn new() -> Self {
        Self {
            redaction_marker: REDACTION_MARKER.to_string(),
            home: None,
        }
    }

    /// Sanitizer with an explicit home directory for path rewriting
    /// (used by tests and by callers that already know the user home).
    pub fn with_home(home: &Path) -> Self {
        Self {
            redaction_marker: REDACTION_MARKER.to_string(),
            home: Some(home.to_path_buf()),
        }
    }

    fn resolve_home(&self) -> Option<PathBuf> {
        self.home.clone().or_else(home_dir)
    }

    /// One composable text pipeline used everywhere a free-text value is
    /// kept: masks secret fragments inside the text AND normalizes absolute
    /// home paths, both transformations in sequence (a compound value with
    /// a token and a path gets both). Whole-value secrets are NOT handled
    /// here — the caller decides `Redacted` for those. Returns `Some(final)`
    /// when anything changed.
    pub fn sanitize_text(&self, s: &str) -> Option<String> {
        if value_looks_secret(s) {
            return None;
        }
        let masked = redact_secret_fragments(s);
        let base: &str = masked.as_deref().unwrap_or(s);
        let home = self.resolve_home();
        match sanitize_home_path(base, home.as_deref()) {
            Some(clean) => Some(clean),
            None => masked,
        }
    }

    /// Decide what happens to a generic (non-env) field.
    pub fn decide(&self, key: &str, value: &str) -> FieldDecision {
        if key_name_is_sensitive(key) || value_looks_secret(value) {
            return FieldDecision::Redacted;
        }
        match self.sanitize_text(value) {
            Some(clean) => FieldDecision::Included(Some(clean)),
            None => FieldDecision::Included(None),
        }
    }

    /// Decide what happens to an environment variable entry.
    pub fn decide_env(&self, name: &str, value: &str) -> FieldDecision {
        if !env_name_allowlisted(name) {
            return FieldDecision::Excluded;
        }
        self.decide(name, value)
    }

    /// Redaction preview for a field list. Field names prefixed `env:` are
    /// treated as environment variables (allowlist applies).
    pub fn classify<S: AsRef<str>>(&self, fields: &[(S, S)]) -> ClassifyReport {
        let mut report = ClassifyReport::default();
        for (k, v) in fields {
            let key = k.as_ref();
            let value = v.as_ref();
            let decision = if let Some(env_name) = key
                .strip_prefix("env:")
                .or_else(|| key.strip_prefix("ENV:"))
            {
                self.decide_env(env_name, value)
            } else {
                self.decide(key, value)
            };
            match decision {
                FieldDecision::Included(rewritten) => {
                    let shown = rewritten.unwrap_or_else(|| value.to_string());
                    report.included.push((key.to_string(), shown));
                }
                FieldDecision::Redacted => report.redacted.push(key.to_string()),
                FieldDecision::Excluded => report.excluded.push(key.to_string()),
            }
        }
        report
    }

    /// Sanitized field list (what actually goes into a bundle).
    pub fn sanitize_fields<S: AsRef<str>>(&self, fields: &[(S, S)]) -> Vec<(String, String)> {
        self.classify(fields).included
    }

    /// Sanitize an arbitrary JSON value (new copy): sensitive keys get
    /// `[REDACTED]`, whole-value secrets are redacted, secret fragments
    /// inside free text are masked in place, absolute home paths rewritten
    /// to `~`. Returns the sanitized copy together with the preview report
    /// describing exactly what happened to it.
    pub fn sanitize_json(&self, value: &serde_json::Value) -> (serde_json::Value, ClassifyReport) {
        let mut report = ClassifyReport::default();
        let clean = self.sanitize_json_inner(value, &[], &mut report);
        report.dedupe();
        (clean, report)
    }

    fn sanitize_json_inner(
        &self,
        value: &serde_json::Value,
        path: &[String],
        report: &mut ClassifyReport,
    ) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, v) in map {
                    if key_name_is_sensitive(k) {
                        report.redacted.push(format_path(path, k));
                        out.insert(
                            k.clone(),
                            serde_json::Value::String(self.redaction_marker.clone()),
                        );
                    } else {
                        let mut child_path: Vec<String> = path.to_vec();
                        child_path.push(k.clone());
                        out.insert(k.clone(), self.sanitize_json_inner(v, &child_path, report));
                    }
                }
                serde_json::Value::Object(out)
            }
            serde_json::Value::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let mut child_path: Vec<String> = path.to_vec();
                        child_path.push(format!("[{i}]"));
                        self.sanitize_json_inner(v, &child_path, report)
                    })
                    .collect(),
            ),
            other => {
                // Leaf: apply the same value-based rules as `decide`.
                let key_path = format_path(path, "");
                if let serde_json::Value::String(s) = other {
                    if value_looks_secret(s) {
                        if !path.is_empty() {
                            report.redacted.push(key_path);
                        }
                        return serde_json::Value::String(self.redaction_marker.clone());
                    }
                    // Composable pipeline: masking + home normalization both
                    // apply (no early return between them).
                    if let Some(final_value) = self.sanitize_text(s) {
                        if !path.is_empty() {
                            report.included.push((key_path, final_value.clone()));
                        }
                        return serde_json::Value::String(final_value);
                    }
                    if !path.is_empty() {
                        report.included.push((key_path, s.clone()));
                    }
                    return other.clone();
                }
                if !path.is_empty() {
                    report.included.push((key_path, other.to_string()));
                }
                other.clone()
            }
        }
    }
}

/// Render a JSON path as `logs[0].message` style (no dot before `[`).
fn format_path(path: &[String], leaf: &str) -> String {
    let mut out = String::new();
    for part in path {
        if part.starts_with('[') {
            out.push_str(part);
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(part);
        }
    }
    if !out.is_empty() && !leaf.is_empty() {
        out.push('.');
    }
    out.push_str(leaf);
    out
}

// ---------------------------------------------------------------------------
// Support bundle
// ---------------------------------------------------------------------------

/// Non-secret project context for a support bundle. `extra` carries freeform
/// metadata and is sanitized before it reaches any bundle file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rw_version: Option<String>,
    /// RimLoc version (CLI passes its own package version here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rimloc_version: Option<String>,
    /// Freeform metadata; sanitized (adversarial fields must not leak).
    #[serde(default)]
    pub extra: serde_json::Value,
}

/// Inputs for [`collect_support_bundle_for`].
#[derive(Debug, Clone, Default)]
pub struct SupportBundleInputs {
    /// Mod / source root the diagnostics describe.
    pub scan_root: PathBuf,
    pub project_meta: ProjectMeta,
    /// The operation being diagnosed (typically the FAILED one). Preserved
    /// verbatim in the bundle — id, stages, timings, errors and causal
    /// chains — so an independent reader can name the likely cause. Never
    /// replaced by a synthetic success.
    pub operation: Option<OperationLog>,
    /// Identifiers of entries affected by the failure (keys/paths/ids).
    /// Sanitized before writing.
    pub affected: Vec<String>,
}

/// One file of a written support bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// A written, sanitized support bundle and its manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundle {
    /// Id of the diagnosed operation when one was supplied (the id a bug
    /// report should quote), otherwise the collector's own operation id.
    pub operation_id: String,
    pub dir: PathBuf,
    pub files: Vec<BundleFile>,
    pub redacted: Vec<String>,
    pub excluded: Vec<String>,
}

/// Collect the sanitized environment (allowlist + path/secret rules) keeping
/// the full preview so exclusions/redactions are reportable.
fn sanitized_env_fields(san: &Sanitizer) -> (Vec<(String, String)>, ClassifyReport) {
    let mut fields: Vec<(String, String)> = Vec::new();
    for (k, v) in std::env::vars_os() {
        let (Some(k), Some(v)) = (k.into_string().ok(), v.into_string().ok()) else {
            continue;
        };
        fields.push((format!("env:{k}"), v));
    }
    fields.sort();
    let report = san.classify(&fields);
    let included = report
        .included
        .iter()
        .map(|(k, v)| (k.replacen("env:", "", 1), v.clone()))
        .collect();
    (included, report)
}

fn collect_scan_counts(scan_root: &Path, op: &mut OperationLog) -> BTreeMap<String, u64> {
    let stage = "diagnostics";
    let mut counts = BTreeMap::new();
    let mut xml_files: u64 = 0;
    for entry in walkdir::WalkDir::new(scan_root).max_depth(12) {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .map(|e| e.eq_ignore_ascii_case("xml"))
                .unwrap_or(false)
        {
            xml_files += 1;
        }
    }
    counts.insert("xml_files".to_string(), xml_files);

    let mut languages: Vec<String> = Vec::new();
    let languages_dir = scan_root.join("Languages");
    if let Ok(read) = std::fs::read_dir(&languages_dir) {
        for e in read.flatten() {
            if e.path().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    languages.push(name.to_string());
                }
            }
        }
    }
    languages.sort();
    op.counter(stage, "language_dirs", languages.len() as u64);
    counts.insert("languages".to_string(), languages.len() as u64);

    match crate::scan_units(scan_root) {
        Ok(units) => {
            let n = units.len() as u64;
            op.counter(stage, "units_scanned", n);
            counts.insert("units_scanned".to_string(), n);
        }
        Err(e) => {
            op.error(stage, e.as_ref());
            counts.insert("units_scanned".to_string(), 0);
            op.counter(stage, "units_scanned", 0);
        }
    }
    counts
}

/// Bundled inputs for [`build_report_md`] (keeps the argument count sane).
struct ReportContext<'a> {
    san: &'a Sanitizer,
    primary: Option<&'a OperationLog>,
    collection: &'a OperationLog,
    project: &'a ProjectMeta,
    scan_root: &'a Path,
    counts: &'a BTreeMap<String, u64>,
    env_pairs: &'a [(String, String)],
    env_report: &'a ClassifyReport,
    affected: &'a [String],
}

/// Human-readable report. Every dynamic value passes through the sanitizer
/// (via the sanitized JSON of each source) before it lands in the file.
fn build_report_md(ctx: &ReportContext<'_>) -> Result<String> {
    let ReportContext {
        san,
        primary,
        collection,
        project,
        scan_root,
        counts,
        env_pairs,
        env_report,
        affected,
    } = *ctx;
    // ONE sanitized ProjectMeta JSON drives the payload, the header fields
    // and the redaction summary: benign versions/languages stay verbatim,
    // whole-value secrets degrade to the marker. Rendering the header from
    // anything else either over-redacts benign values or leaks secrets.
    let mut meta_value = serde_json::to_value(project)?;
    if meta_value
        .get("rimloc_version")
        .map(|v| v.is_null())
        .unwrap_or(true)
    {
        meta_value["rimloc_version"] =
            serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string());
    }
    let (meta_clean, meta_report) = san.sanitize_json(&meta_value);
    let field = |key: &str| -> Option<&str> { meta_clean[key].as_str() };

    // Sanitize the primary operation (if any) once and render from the
    // sanitized copy — what the reader sees is exactly what was sanitized.
    let primary_clean = primary.map(|op| san.sanitize_json(&op.to_value()).0);
    let scan_root_display = san
        .sanitize_text(&scan_root.to_string_lossy())
        .unwrap_or_else(|| scan_root.to_string_lossy().into_owned());

    let mut md = String::new();
    md.push_str("# RimLoc support bundle\n\n");
    match (primary, &primary_clean) {
        (Some(op), Some(clean)) => {
            md.push_str(&format!(
                "- Diagnosed operation: `{}` ({})\n",
                clean["operation_id"].as_str().unwrap_or("?"),
                clean["name"].as_str().unwrap_or("?")
            ));
            // Derived status: identical semantics to the JSON payload.
            match clean["status"].as_str() {
                Some("failed") if op.is_finished() => {
                    md.push_str("- Operation state: **failed** (finished with errors)\n");
                }
                Some("failed") => {
                    md.push_str(
                        "- Operation state: **failed** (NOT finished — failed/crashed) — preserved as-is\n",
                    );
                }
                Some("running") => {
                    md.push_str("- Operation state: **running** — preserved as-is\n");
                }
                _ => md.push_str("- Operation state: **succeeded**\n"),
            }
            // The collection operation is finished before the report is
            // rendered, so this is a real timestamp, never a guess.
            md.push_str(&format!(
                "- Collection: `{}` finished at {}\n",
                collection.operation_id,
                collection.finished_at.as_deref().unwrap_or("?")
            ));
        }
        _ => {
            md.push_str(&format!(
                "- Operation: `{}` ({})\n",
                collection.operation_id, collection.name
            ));
        }
    }
    md.push_str(&format!("- Generated: {}\n", rfc3339_now()));
    md.push_str(&format!("- Scan root: {scan_root_display}\n"));
    // Header fields come from the sanitized meta: benign values render
    // verbatim, a whole-value secret renders as the marker, absent fields
    // render as nothing.
    if let Some(v) = field("rimloc_version") {
        md.push_str(&format!("- RimLoc version: {v}\n"));
    }
    if let Some(v) = field("rw_version") {
        md.push_str(&format!("- RimWorld version: {v}\n"));
    }
    if let Some(v) = field("target_lang") {
        md.push_str(&format!("- Target language: {v}\n"));
    }
    md.push('\n');

    // Causal context of the diagnosed operation comes first: this is the
    // part an independent reviewer needs. The section title follows the
    // derived status — a succeeded operation is not labelled failed.
    if let Some(clean) = &primary_clean {
        match clean["status"].as_str() {
            Some("running") => md.push_str("## Diagnosed operation context (running)\n\n"),
            Some("succeeded") => md.push_str("## Operation context (succeeded)\n\n"),
            _ => md.push_str("## Failed operation context\n\n"),
        }
        if let Some(stages) = clean["stages"].as_array() {
            md.push_str("| stage | started | finished | duration_ms |\n|---|---|---|---|\n");
            for s in stages {
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    s["stage"].as_str().unwrap_or("?"),
                    s["started_at"].as_str().unwrap_or("?"),
                    s["finished_at"].as_str().unwrap_or("—"),
                    s["duration_ms"]
                        .as_u64()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "—".to_string())
                ));
            }
            md.push('\n');
        }
        if let Some(errors) = clean["errors"].as_array() {
            if !errors.is_empty() {
                md.push_str("### Errors (causal chain)\n\n");
                for e in errors {
                    md.push_str(&format!(
                        "- **{}**: {}\n",
                        e["stage"].as_str().unwrap_or("?"),
                        e["message"].as_str().unwrap_or("?")
                    ));
                    if let Some(chain) = e["chain"].as_array() {
                        for link in chain {
                            md.push_str(&format!(
                                "  - caused by: {}\n",
                                link.as_str().unwrap_or("?")
                            ));
                        }
                    }
                }
                md.push('\n');
            }
        }
        if !affected.is_empty() {
            md.push_str("### Affected entries\n\n");
            for a in affected {
                md.push_str(&format!("- `{a}`\n"));
            }
            md.push('\n');
        }
    }

    // Collection errors, also sanitized before rendering.
    let collection_clean = san.sanitize_json(&collection.to_value()).0;
    match collection_clean["errors"].as_array() {
        Some(errors) if !errors.is_empty() => {
            md.push_str("## Collection errors\n\n");
            for e in errors {
                md.push_str(&format!(
                    "- **{}**: {}\n",
                    e["stage"].as_str().unwrap_or("?"),
                    e["message"].as_str().unwrap_or("?")
                ));
                if let Some(chain) = e["chain"].as_array() {
                    for link in chain {
                        md.push_str(&format!(
                            "  - caused by: {}\n",
                            link.as_str().unwrap_or("?")
                        ));
                    }
                }
            }
            md.push('\n');
        }
        _ => md.push_str("## Collection errors\n\nNone recorded.\n\n"),
    }

    md.push_str("## Scan counts\n\n| metric | value |\n|---|---|\n");
    for (k, v) in counts {
        md.push_str(&format!("| {k} | {v} |\n"));
    }
    md.push('\n');

    // Redaction summary describes the actual final payload: metadata,
    // environment (incl. excluded env vars) and serialized operation logs.
    md.push_str("## Redaction summary\n\n");
    md.push_str(&format!(
        "- Included fields: {}\n- Redacted fields: {} (metadata: {}, environment: {}, operation logs: masked in place)\n- Excluded fields: {} (environment outside the allowlist)\n",
        meta_report.included.len(),
        meta_report.redacted.len() + env_report.redacted.len(),
        meta_report.redacted.len(),
        env_report.redacted.len(),
        env_report.excluded.len(),
    ));
    if !env_report.excluded.is_empty() {
        md.push_str("- Excluded env vars: ");
        md.push_str(&env_report.excluded.join(", "));
        md.push('\n');
    }
    if !meta_report.redacted.is_empty() {
        md.push_str("- Redacted keys (metadata): ");
        md.push_str(&meta_report.redacted.join(", "));
        md.push('\n');
    }
    md.push('\n');

    md.push_str("## Environment (sanitized)\n\n");
    md.push_str(&format!(
        "- OS: {} ({})\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    for (k, v) in env_pairs {
        md.push_str(&format!("- `{k}`: {v}\n"));
    }
    if let Some(extra_clean) = meta_clean.get("extra") {
        let empty_obj = serde_json::Map::new();
        let map = extra_clean.as_object().unwrap_or(&empty_obj);
        if !map.is_empty() {
            md.push_str("\n## Project metadata (sanitized)\n\n```json\n");
            md.push_str(&serde_json::to_string_pretty(extra_clean)?);
            md.push_str("\n```\n");
        }
    }
    Ok(md)
}

/// Collect a sanitized support bundle under `out_dir` (created if missing).
///
/// Writes `report.md` (human summary incl. preserved failure context),
/// `diagnostics.json` (diagnosed operation + collection log + counts +
/// sanitized meta), `environment.json` (sanitized env) and `manifest.json`
/// (sha256 of the payload files), all via `write_atomic`.
pub fn collect_support_bundle_for(
    inputs: &SupportBundleInputs,
    out_dir: &Path,
) -> Result<SupportBundle> {
    // Read-only invariant, enforced BEFORE any mkdir/write: the bundle must
    // never be written into the scanned source tree (equal or nested,
    // including symlink aliases and parent-traversing not-yet-existing
    // paths — both are caught by the canonical containment view).
    if crate::is_within(out_dir, &inputs.scan_root) {
        color_eyre::eyre::bail!(
            "support bundle output directory `{}` is inside the read-only source tree `{}`; choose a directory outside the scanned source",
            out_dir.display(),
            inputs.scan_root.display()
        );
    }

    let san = Sanitizer::new();
    let mut op = OperationLog::new("support_bundle");

    // Stage: environment — sanitized env + runtime facts. The payload is
    // built with raw values and sanitized ONCE as a whole before writing,
    // so every field (versions included) gets the full compound pipeline.
    op.begin_stage("environment");
    let (env_pairs, env_report) = sanitized_env_fields(&san);
    let env_value = serde_json::json!({
        "operation_id": op.operation_id,
        "related_operation_id": inputs.operation.as_ref().map(|o| o.operation_id.clone()),
        "generated_at": rfc3339_now(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "rimloc_version": inputs
            .project_meta
            .rimloc_version
            .clone()
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        "rw_version": inputs.project_meta.rw_version,
        "locales": env_pairs
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("LANG") || k.eq_ignore_ascii_case("LC_ALL"))
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>(),
        "env": env_pairs,
        "scan_root": inputs.scan_root.to_string_lossy(),
    });
    let (env_clean, env_payload_report) = san.sanitize_json(&env_value);
    let env_path = out_dir.join("environment.json");
    write_atomic(
        &env_path,
        format!("{}\n", serde_json::to_string_pretty(&env_clean)?).as_bytes(),
    )?;
    op.counter("environment", "env_vars_included", env_pairs.len() as u64);
    op.end_stage("environment");

    // Stage: diagnostics — scan counts over the source root.
    op.begin_stage("diagnostics");
    let counts = collect_scan_counts(&inputs.scan_root, &mut op);
    op.end_stage("diagnostics");

    // All data gathering is done: finish the collection operation BEFORE
    // the report is rendered, so the report reflects a genuinely completed
    // collection (a real finished_at, never a guessed one). The report
    // rendering itself is presentation, not a recorded stage.
    op.finish();
    let affected_sanitized: Vec<String> = inputs
        .affected
        .iter()
        .filter_map(|a| {
            // Whole-value secrets are dropped entirely; everything else goes
            // through the same composable pipeline as every kept text.
            if value_looks_secret(a) {
                return None;
            }
            Some(san.sanitize_text(a).unwrap_or_else(|| a.clone()))
        })
        .collect();
    let report_md = build_report_md(&ReportContext {
        san: &san,
        primary: inputs.operation.as_ref(),
        collection: &op,
        project: &inputs.project_meta,
        scan_root: &inputs.scan_root,
        counts: &counts,
        env_pairs: &env_pairs,
        env_report: &env_report,
        affected: &affected_sanitized,
    })?;
    let report_path = out_dir.join("report.md");
    write_atomic(&report_path, report_md.as_bytes())?;

    // Diagnostics payload: diagnosed operation verbatim in structure but
    // sanitized in content; collection log; counts; sanitized meta
    // (every field); sanitized affected ids.
    let (primary_clean, primary_report) = match inputs.operation.as_ref().map(|o| o.to_value()) {
        Some(v) => {
            let (c, r) = san.sanitize_json(&v);
            (Some(c), r)
        }
        None => (None, ClassifyReport::default()),
    };
    let (collection_clean, collection_report) = san.sanitize_json(&op.to_value());
    let meta_value = serde_json::to_value(&inputs.project_meta)?;
    let (meta_clean, meta_report) = san.sanitize_json(&meta_value);

    let diagnostics = serde_json::json!({
        "operation": primary_clean,
        "collection": collection_clean,
        "counts": counts,
        "affected": affected_sanitized,
        "project_meta": meta_clean,
    });
    let diag_path = out_dir.join("diagnostics.json");
    write_atomic(
        &diag_path,
        format!("{}\n", serde_json::to_string_pretty(&diagnostics)?).as_bytes(),
    )?;

    let mut files = Vec::new();
    for path in [report_path, diag_path, env_path] {
        let bytes = std::fs::read(&path)?;
        files.push(BundleFile {
            path: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            sha256: sha256_hex(&bytes),
            bytes: bytes.len() as u64,
        });
    }
    let bundle_operation_id = inputs
        .operation
        .as_ref()
        .map(|o| o.operation_id.clone())
        .unwrap_or_else(|| op.operation_id.clone());
    let manifest = serde_json::json!({
        "operation_id": bundle_operation_id,
        "collection_operation_id": op.operation_id,
        "files": files,
    });
    write_atomic(
        &out_dir.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?).as_bytes(),
    )?;

    let mut report = env_report.clone();
    report.merge(&env_payload_report);
    report.merge(&primary_report);
    report.merge(&collection_report);
    report.merge(&meta_report);
    report.dedupe();

    Ok(SupportBundle {
        operation_id: bundle_operation_id,
        dir: out_dir.to_path_buf(),
        files,
        redacted: report.redacted,
        excluded: report.excluded,
    })
}

/// Collect a bundle with default (collector-owned) operation context.
pub fn collect_support_bundle_in(
    out_dir: &Path,
    scan_root: &Path,
    project_meta: &ProjectMeta,
) -> Result<SupportBundle> {
    collect_support_bundle_for(
        &SupportBundleInputs {
            scan_root: scan_root.to_path_buf(),
            project_meta: project_meta.clone(),
            operation: None,
            affected: Vec::new(),
        },
        out_dir,
    )
}

/// Collect a support bundle in a default location:
/// `./rimloc-support/<operation_id>/` relative to the current directory.
///
/// If the current directory turns out to be inside the scanned source tree,
/// the call refuses instead of writing into the source — pass an explicit
/// out dir outside the source via [`collect_support_bundle_in`] / [`collect_support_bundle_for`].
pub fn collect_support_bundle(
    scan_root: &Path,
    project_meta: &ProjectMeta,
) -> Result<SupportBundle> {
    let op_id = generate_operation_id();
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("rimloc-support")
        .join(op_id);
    if crate::is_within(&dir, scan_root) {
        color_eyre::eyre::bail!(
            "current directory is inside the read-only source tree `{}`; pass an explicit support-bundle output directory outside the source",
            scan_root.display()
        );
    }
    collect_support_bundle_in(&dir, scan_root, project_meta)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn read_to_string(path: &Path) -> String {
        let mut f = std::fs::File::open(path).expect("open");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("read");
        s
    }

    // --- sha256 ------------------------------------------------------------

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn sha256_multi_block_stable() {
        // 200 'a' chars — multiple 64-byte blocks and padding.
        let data = vec![b'a'; 200];
        let hex = sha256_hex(&data);
        assert_eq!(hex.len(), 64);
        assert_eq!(hex, hex.to_lowercase());
    }

    // --- operation id -------------------------------------------------------

    #[test]
    fn operation_status_is_derived_lifecycle_and_error_aware() {
        // unfinished, no errors → running
        let mut running = OperationLog::new("r");
        running.begin_stage("work");
        assert_eq!(running.status(), OperationStatus::Running);
        assert_eq!(
            running.to_value()["status"],
            serde_json::Value::String("running".to_string())
        );

        // unfinished, with errors → failed
        let mut failed = OperationLog::new("f");
        failed.begin_stage("v");
        failed.error_message("v", "boom");
        failed.end_stage("v");
        assert_eq!(failed.status(), OperationStatus::Failed);

        // finished, no errors → succeeded
        let mut ok = OperationLog::new("ok");
        ok.begin_stage("w");
        ok.end_stage("w");
        ok.finish();
        assert_eq!(ok.status(), OperationStatus::Succeeded);
        assert!(ok.to_json_pretty().expect("json").contains("\"succeeded\""));

        // finished, with errors → failed (a finished run with errors never
        // claims success)
        let mut fin_err = OperationLog::new("fe");
        fin_err.begin_stage("w");
        fin_err.error_message("w", "boom");
        fin_err.end_stage("w");
        fin_err.finish();
        assert!(fin_err.is_finished());
        assert_eq!(fin_err.status(), OperationStatus::Failed);
        assert_eq!(
            fin_err.to_value()["status"],
            serde_json::Value::String("failed".to_string())
        );

        // Legacy serialized logs WITHOUT the derived field stay readable
        // and re-derive the status on export.
        let legacy = serde_json::json!({
            "operation_id": "op-legacy",
            "name": "legacy",
            "started_at": "2026-01-01T00:00:00.000Z",
            "stages": [],
        });
        let parsed: OperationLog = serde_json::from_value(legacy).expect("legacy parses");
        assert_eq!(parsed.status(), OperationStatus::Running);
        assert_eq!(
            parsed.to_value()["status"],
            serde_json::Value::String("running".to_string())
        );
    }

    #[test]
    fn operation_ids_are_unique_and_prefixed() {
        let a = generate_operation_id();
        let b = generate_operation_id();
        assert_ne!(a, b);
        assert!(a.starts_with("op-"));
        assert!(b.starts_with("op-"));
    }

    // --- operation log ------------------------------------------------------

    #[derive(Debug)]
    struct RootError(String);
    impl std::fmt::Display for RootError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::error::Error for RootError {}

    #[derive(Debug)]
    struct WrappedError {
        root: RootError,
    }
    impl std::fmt::Display for WrappedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "wrapped failure")
        }
    }
    impl std::error::Error for WrappedError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.root)
        }
    }

    #[test]
    fn operation_log_stages_counters_errors_and_json() {
        let mut op = OperationLog::new("test-op");
        assert!(!op.operation_id.is_empty());

        op.begin_stage("scan");
        op.counter("scan", "units", 42);
        op.end_stage("scan");

        op.begin_stage("diagnose");
        op.error(
            "diagnose",
            &WrappedError {
                root: RootError("root cause".to_string()),
            },
        );
        op.end_stage("diagnose");
        op.finish();

        assert!(op.is_finished());
        assert!(op.total_duration_ms.is_some());
        assert_eq!(op.stages.len(), 2);
        assert_eq!(op.stages[0].counters.get("units"), Some(&42));
        assert_eq!(op.errors.len(), 1);
        // error chain walked through `source()`
        assert_eq!(op.errors[0].chain, vec!["root cause".to_string()]);

        let json = op.to_json_pretty().expect("json");
        assert!(json.contains("operation_id"));
        assert!(json.contains("root cause"));
        assert!(json.contains("\"duration_ms\""));

        // write_json lands via write_atomic
        let tmp = tempfile::tempdir().expect("tmp");
        let path = tmp.path().join("nested").join("op.json");
        op.write_json(&path).expect("write");
        assert!(path.exists());
        assert!(!path.with_extension("tmp.write").exists());
    }

    #[test]
    fn operation_log_lifecycle_is_honest() {
        let mut op = OperationLog::new("lifecycle");
        assert!(!op.is_finished());
        assert!(op.finished_at.is_none());

        op.begin_stage("work");
        op.end_stage("work");
        // Ending a stage must NOT mark the operation finished.
        assert!(!op.is_finished());
        assert!(op.finished_at.is_none());
        assert!(op.total_duration_ms.is_none());

        let snapshot = op.to_value();
        assert!(snapshot["finished_at"].is_null());

        op.finish();
        assert!(op.is_finished());
        assert!(op.finished_at.is_some());
        assert!(op.total_duration_ms.is_some());

        // New work after finish re-opens the operation honestly.
        op.begin_stage("more");
        assert!(!op.is_finished());
        assert!(op.finished_at.is_none());
        op.end_stage("more");
        op.finish();
        assert!(op.is_finished());
    }

    // --- sanitizer: field-level ---------------------------------------------

    #[test]
    fn sanitizer_classifies_keys_tokens_paths_and_plain_fields() {
        // Deterministic home so the `~` rewrite is asserted portably.
        let san = Sanitizer::with_home(Path::new("/Users/someone"));
        let fields = [
            ("apiKey", "sk-ant-1234567890abcdef"),
            ("api_token", "abcdef0123456789abcdef0123456789"),
            ("password", "hunter2"),
            ("Authorization", "Bearer abc.def.ghi"),
            ("home_path", "/Users/someone/.rimloc/cache"),
            ("units_scanned", "1234"),
            ("target_lang", "Russian"),
        ];
        let report = san.classify(&fields);

        // credential-looking keys → redacted
        assert!(report.redacted.contains(&"apiKey".to_string()));
        assert!(report.redacted.contains(&"api_token".to_string()));
        assert!(report.redacted.contains(&"password".to_string()));
        assert!(report.redacted.contains(&"Authorization".to_string()));
        // home path → included with ~ rewrite
        let home = report
            .included
            .iter()
            .find(|(k, _)| k == "home_path")
            .map(|(_, v)| v.clone())
            .expect("home_path included");
        assert_eq!(home, "~/.rimloc/cache");
        // plain fields pass through
        assert!(report
            .included
            .contains(&("units_scanned".to_string(), "1234".to_string())));
        assert!(report
            .included
            .contains(&("target_lang".to_string(), "Russian".to_string())));
        assert!(report.excluded.is_empty());
    }

    #[test]
    fn sanitizer_redacts_secret_looking_values_regardless_of_key() {
        let san = Sanitizer::new();
        let report = san.classify(&[
            ("note", "sk-proj-0123456789abcdef"),
            ("header", "Bearer eyJhbGciOiJIUzI1NiJ9"),
            ("blob", "-----BEGIN RSA PRIVATE KEY-----"),
        ]);
        for k in &report.redacted {
            assert!(["note", "header", "blob"].contains(&k.as_str()));
        }
        assert_eq!(report.redacted.len(), 3);
    }

    #[test]
    fn sanitizer_masks_secrets_inside_free_text_keeping_context() {
        let san = Sanitizer::with_home(Path::new("/Users/someone"));
        let fields = [
            (
                "error_log",
                "request failed with Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.abcdef; retry 3",
            ),
            ("hint", "set apiKey=sk-live-0123456789abcdef before calls"),
            (
                "path_note",
                "config at /Users/someone/.rimloc/config.json is valid",
            ),
        ];
        let report = san.classify(&fields);

        let log = report
            .included
            .iter()
            .find(|(k, _)| k == "error_log")
            .map(|(_, v)| v.clone())
            .expect("error_log kept with masked secret");
        assert!(
            log.contains("request failed with Authorization:"),
            "context kept: {log}"
        );
        assert!(log.contains(REDACTION_MARKER), "token masked: {log}");
        assert!(!log.contains("eyJhbGciOiJIUzI1NiJ9"), "token leaked: {log}");

        let hint = report
            .included
            .iter()
            .find(|(k, _)| k == "hint")
            .map(|(_, v)| v.clone())
            .expect("hint kept");
        assert!(hint.contains("set "), "context kept: {hint}");
        assert!(hint.contains(REDACTION_MARKER));
        assert!(!hint.contains("sk-live-0123456789abcdef"));

        let note = report
            .included
            .iter()
            .find(|(k, _)| k == "path_note")
            .map(|(_, v)| v.clone())
            .expect("path_note kept");
        assert_eq!(note, "config at ~/.rimloc/config.json is valid");
    }

    #[test]
    fn sanitizer_masks_provider_tokens_table_with_context_kept() {
        // Table-driven: every provider token family must be fully masked by
        // the mid-text pass — the credential itself must never survive as a
        // "label" prefix next to the marker.
        let san = Sanitizer::new();
        let cases = [
            "AKIA1234567890ABCDEF",
            "ghp_0123456789abcdefghij",
            "gho_0123456789abcdefghij",
            "github_pat_0123456789abcdefghij",
            "xoxb-0123456789-abcdef",
            "xoxp-1-2-3-abcdef0123",
        ];
        for token in cases {
            let value = format!("request failed while using {token} upstream");
            match san.decide("error_log", &value) {
                FieldDecision::Included(Some(masked)) => {
                    assert!(
                        masked.contains("request failed while using"),
                        "context kept for {token}: {masked}"
                    );
                    assert!(masked.contains(REDACTION_MARKER), "masked: {masked}");
                    assert!(
                        !masked.contains(token),
                        "token {token} survived in: {masked}"
                    );
                    assert!(
                        !masked.contains("AKIA") && !masked.contains("ghp_"),
                        "credential prefix survived in: {masked}"
                    );
                }
                other => {
                    let msg = format!("{token}: expected kept-with-mask, got {other:?}");
                    panic!("{}", msg);
                }
            }
        }
    }

    #[test]
    fn sanitizer_env_allowlist_is_minimal_and_excludes_strangers() {
        let san = Sanitizer::new();
        let fields = [
            ("env:LANG", "en_US.UTF-8"),
            ("env:LC_ALL", "en_US.UTF-8"),
            ("env:TERM", "xterm-256color"),
            ("env:TMPDIR", "/var/folders/ab/"),
            // Wide families and account names are no longer allowlisted.
            ("env:PATH", "/usr/bin:/bin"),
            ("env:HOME", "/Users/someone"),
            ("env:USER", "someone"),
            ("env:RIMLOC_LOG_FORMAT", "json"),
            ("env:CARGO_HOME", "/usr/local/cargo"),
            // unknown prefixed var carrying a non-sk secret → still excluded
            ("env:RIMLOC_PROVIDER_SECRET", "opaque-secret-value-00"),
            ("env:SOME_RANDOM_VAR", "value"),
        ];
        let report = san.classify(&fields);
        for kept in ["env:LANG", "env:LC_ALL", "env:TERM", "env:TMPDIR"] {
            assert!(
                report.included.iter().any(|(k, _)| k == kept),
                "{kept} kept"
            );
        }
        for dropped in [
            "env:PATH",
            "env:HOME",
            "env:USER",
            "env:RIMLOC_LOG_FORMAT",
            "env:CARGO_HOME",
            "env:RIMLOC_PROVIDER_SECRET",
            "env:SOME_RANDOM_VAR",
        ] {
            assert!(
                report.excluded.contains(&dropped.to_string()),
                "{dropped} excluded"
            );
        }
        // The exclusion preview is the truth: excluded values appear nowhere.
        assert!(report
            .included
            .iter()
            .all(|(_, v)| !v.contains("opaque-secret-value-00")));
    }

    #[test]
    fn sanitizer_compound_path_and_token_get_both_transforms() {
        // Round-two probe case: one free-text value with a home path AND a
        // secret — masking and home normalization must BOTH apply.
        let san = Sanitizer::with_home(Path::new("/Users/demo"));
        let value = "parsed /Users/demo/mod/Defs/a.xml then hit Bearer tokSYNTH123456";
        let out = san.sanitize_text(value).expect("compound value rewritten");
        assert!(out.contains("~/mod/Defs/a.xml"), "path normalized: {out}");
        assert!(out.contains(REDACTION_MARKER), "token masked: {out}");
        assert!(!out.contains("tokSYNTH123456"), "token leaked: {out}");
        assert!(!out.contains("/Users/demo"), "home prefix leaked: {out}");

        // Whole-value secret in a compound-looking string stays redacted.
        let whole = san.decide("note", "sk-abc123456789");
        assert_eq!(whole, FieldDecision::Redacted);
    }

    // --- sanitizer: JSON (nested structures, arrays, string leaves) ---------

    #[test]
    fn sanitizer_json_redacts_nested_metadata() {
        let san = Sanitizer::with_home(Path::new("/Users/someone"));
        let meta = serde_json::json!({
            "apiKey": "sk-proj-abcdef123456",
            "nested": { "token": "ghp_0123456789abcdefghijklmnopqrstuvwxyz" },
            "note": "clean value",
            "home": "/Users/someone/project",
        });
        let (clean, report) = san.sanitize_json(&meta);
        assert_eq!(
            clean["apiKey"],
            serde_json::Value::String(REDACTION_MARKER.to_string())
        );
        assert_eq!(
            clean["nested"]["token"],
            serde_json::Value::String(REDACTION_MARKER.to_string())
        );
        assert_eq!(
            clean["note"],
            serde_json::Value::String("clean value".to_string())
        );
        assert_eq!(
            clean["home"],
            serde_json::Value::String("~/project".to_string())
        );
        assert!(report.redacted.contains(&"apiKey".to_string()));
        assert!(report.redacted.contains(&"nested.token".to_string()));
    }

    #[test]
    fn sanitizer_json_applies_value_rules_to_string_leaves_and_arrays() {
        let san = Sanitizer::new();
        let payload = serde_json::json!({
            "message": "sk-1234567890abcdef",
            "log": [
                "request ok",
                "failed: Authorization: Bearer tok1234567890 and then retries",
            ],
            "detail": {
                "trace": "used token=0123456789abcdef0123456789abcdef for provider",
            },
        });
        let (clean, report) = san.sanitize_json(&payload);

        // whole-value secret on an unnamed string leaf → full redaction
        assert_eq!(
            clean["message"],
            serde_json::Value::String(REDACTION_MARKER.to_string())
        );
        assert!(report.redacted.iter().any(|p| p == "message"));

        // array element: secret masked in place, clean context preserved
        let second = clean["log"][1].as_str().expect("array leaf string");
        assert!(
            second.starts_with("failed: Authorization:"),
            "got: {second}"
        );
        assert!(second.contains(REDACTION_MARKER));
        assert!(!second.contains("tok1234567890"));
        // untouched clean element survives
        assert_eq!(
            clean["log"][0],
            serde_json::Value::String("request ok".to_string())
        );

        // nested object leaf with key=value secret inside text
        let trace = clean["detail"]["trace"].as_str().expect("trace");
        assert!(trace.starts_with("used "), "context kept: {trace}");
        assert!(trace.contains(REDACTION_MARKER));
        assert!(!trace.contains("0123456789abcdef"));

        // preview reflects the actual outcomes: masked-in-place context is
        // included (with the masked value), whole-value secrets are redacted
        assert!(report
            .included
            .iter()
            .any(|(p, v)| p == "log[1]" && v.contains(REDACTION_MARKER)));
        assert!(report
            .included
            .iter()
            .any(|(p, v)| p == "detail.trace" && v.contains(REDACTION_MARKER)));
    }

    // --- support bundle -----------------------------------------------------

    fn make_mod_fixture(tmp: &Path) -> PathBuf {
        let mod_root = tmp.join("TestMod");
        let keyed = mod_root.join("Languages/English/Keyed");
        std::fs::create_dir_all(&keyed).expect("dirs");
        std::fs::write(
            keyed.join("Actions.xml"),
            "<LanguageData>\n  <Test.label>Label</Test.label>\n</LanguageData>\n",
        )
        .expect("write xml");
        mod_root
    }

    #[test]
    fn support_bundle_is_sanitized_and_manifest_hashes_match() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());

        let bundle_dir = tmp.path().join("bundle");
        // Build the adversarial home path from the REAL home so the rewrite
        // is exercised on the machine running the test.
        let real_home = std::env::var("HOME").unwrap_or_else(|_| "/Users/someone".to_string());
        let adversarial_home = format!("{real_home}/secretplace");
        let meta = ProjectMeta {
            name: Some("Test".into()),
            target_lang: Some("Russian".into()),
            rw_version: Some("1.6".into()),
            rimloc_version: Some("0.1.0-alpha.1".into()),
            // adversarial: secrets in freeform metadata must not leak
            extra: serde_json::json!({
                "apiKey": "sk-test-0123456789abcdef",
                "secret_value": "super-secret-thing",
                "home_dir": adversarial_home,
                "note": "Authorization: Bearer leakedtok123456789 mid-text",
                "ok_field": "fine",
            }),
        };
        let bundle = collect_support_bundle_in(&bundle_dir, &mod_root, &meta).expect("bundle");

        // files exist + manifest sha256 matches on-disk content
        let manifest_raw = read_to_string(&bundle_dir.join("manifest.json"));
        let manifest: serde_json::Value = serde_json::from_str(&manifest_raw).expect("json");
        let files = manifest["files"].as_array().expect("files").clone();
        assert_eq!(files.len(), 3);
        for f in &files {
            let name = f["path"].as_str().expect("path");
            let expected = f["sha256"].as_str().expect("sha");
            let actual = sha256_hex(&std::fs::read(bundle_dir.join(name)).expect("read"));
            assert_eq!(expected, actual, "sha mismatch for {name}");
            assert_eq!(expected.len(), 64);
        }
        assert_eq!(bundle.files.len(), 3);
        assert!(!bundle.operation_id.is_empty());

        // adversarial: secrets never reach any bundle file
        for name in ["report.md", "diagnostics.json", "environment.json"] {
            let content = read_to_string(&bundle_dir.join(name));
            assert!(
                !content.contains("sk-test-0123456789abcdef"),
                "{name} leaked apiKey"
            );
            assert!(
                !content.contains("super-secret-thing"),
                "{name} leaked secret_value"
            );
            assert!(
                !content.contains("leakedtok123456789"),
                "{name} leaked mid-text bearer token"
            );
            assert!(
                !content.contains(&adversarial_home),
                "{name} leaked absolute home path"
            );
            assert!(
                !content.contains(&real_home),
                "{name} leaked the real home directory"
            );
        }
        for name in ["report.md", "diagnostics.json"] {
            let content = read_to_string(&bundle_dir.join(name));
            assert!(
                content.contains("~/secretplace"),
                "{name} must rewrite home to ~"
            );
            assert!(
                content.contains(REDACTION_MARKER),
                "{name} must show redaction markers"
            );
        }
        // excluded env vars are disclosed, not silently dropped
        let report_md = read_to_string(&bundle_dir.join("report.md"));
        assert!(
            report_md.contains("Excluded env vars"),
            "report must disclose env exclusions"
        );
        assert!(
            !bundle.excluded.is_empty(),
            "SupportBundle.excluded must reflect actual exclusions"
        );
        assert!(bundle.redacted.iter().any(|k| k.contains("apiKey")));

        let report = read_to_string(&bundle_dir.join("report.md"));
        assert!(report.contains("RimLoc support bundle"));
        assert!(report.contains("units_scanned"));
        // diagnostics carries the collection log
        let diag = read_to_string(&bundle_dir.join("diagnostics.json"));
        assert!(diag.contains("operation_id"));
        assert!(diag.contains("stages"));
        // Report truth (023): benign header fields render VERBATIM and
        // identically across artifacts; the collection completion is a real
        // timestamp, never a placeholder.
        assert!(
            report.contains("- RimWorld version: 1.6"),
            "benign version kept verbatim"
        );
        assert!(report.contains("- Target language: Russian"));
        assert!(report.contains("- RimLoc version: 0.1.0-alpha.1"));
        assert!(
            diag.contains("\"rw_version\": \"1.6\""),
            "same value in diagnostics"
        );
        assert!(
            !report.contains("finished at ?"),
            "no placeholder completion"
        );
        // Derived collection status agrees with the finished, error-free run.
        let diag_json: serde_json::Value = serde_json::from_str(&diag).expect("diag json");
        assert_eq!(
            diag_json["collection"]["status"].as_str(),
            Some("succeeded"),
            "collection derived status"
        );
        assert!(
            diag_json["collection"]["finished_at"].is_string(),
            "real completion timestamp in diagnostics"
        );
    }

    #[test]
    fn bundle_preserves_failing_operation_as_causal_context() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());
        let bundle_dir = tmp.path().join("bundle");

        // Controlled validator failure: stages, counters, error + causal chain.
        let mut failed = OperationLog::new("validate_po");
        failed.begin_stage("validate");
        failed.counter("validate", "checked_units", 12);
        failed.error(
            "validate",
            &WrappedError {
                root: RootError("placeholder mismatch in Actions.Test.label".to_string()),
            },
        );
        failed.end_stage("validate");
        // Crashed/failed runs never reach finish() — keep it that way.
        let failed_id = failed.operation_id.clone();
        assert!(!failed.is_finished());

        let bundle = collect_support_bundle_for(
            &SupportBundleInputs {
                scan_root: mod_root,
                project_meta: ProjectMeta {
                    name: Some("Test".into()),
                    ..Default::default()
                },
                operation: Some(failed),
                affected: vec!["Actions.Test.label".to_string()],
            },
            &bundle_dir,
        )
        .expect("bundle");

        // The bundle quotes the ORIGINAL operation id, not a synthetic one.
        assert_eq!(bundle.operation_id, failed_id);
        let manifest: serde_json::Value =
            serde_json::from_str(&read_to_string(&bundle_dir.join("manifest.json")))
                .expect("manifest");
        assert_eq!(
            manifest["operation_id"],
            serde_json::Value::String(failed_id.clone())
        );

        let diag_raw = read_to_string(&bundle_dir.join("diagnostics.json"));
        let diag: serde_json::Value = serde_json::from_str(&diag_raw).expect("diagnostics");

        // Original operation preserved verbatim in structure: same id, its
        // stage, its counter, its causal chain — NOT a synthetic success.
        let operation = &diag["operation"];
        assert_eq!(
            operation["operation_id"],
            serde_json::Value::String(failed_id.clone())
        );
        assert_eq!(
            operation["name"],
            serde_json::Value::String("validate_po".to_string())
        );
        assert_eq!(
            operation["stages"][0]["stage"],
            serde_json::Value::String("validate".to_string())
        );
        assert_eq!(
            operation["stages"][0]["counters"]["checked_units"],
            serde_json::Value::from(12)
        );
        assert_eq!(
            operation["errors"][0]["message"],
            serde_json::Value::String("wrapped failure".to_string())
        );
        assert_eq!(
            operation["errors"][0]["chain"][0],
            serde_json::Value::String("placeholder mismatch in Actions.Test.label".to_string())
        );
        // Controlled-failure state kept: the preserved log stays unfinished.
        assert!(operation["finished_at"].is_null());
        // Affected ids survive sanitization.
        assert_eq!(
            diag["affected"][0],
            serde_json::Value::String("Actions.Test.label".to_string())
        );

        // An independent reader can name the probable cause from report.md.
        let report = read_to_string(&bundle_dir.join("report.md"));
        assert!(
            report.contains(&failed_id),
            "report quotes the failing operation id"
        );
        assert!(report.contains("Failed operation context"));
        assert!(report.contains("NOT finished"));
        assert!(report.contains("placeholder mismatch in Actions.Test.label"));
        assert!(report.contains("Actions.Test.label"));
        assert!(report.contains("caused by"));
    }

    #[test]
    fn bundle_survives_unreadable_scan_root() {
        // A missing scan root must produce a bundle with a recorded error,
        // not fail the whole operation — diagnostics value over perfection.
        let tmp = tempfile::tempdir().expect("tmp");
        let bundle_dir = tmp.path().join("bundle");
        let missing = tmp.path().join("does-not-exist");
        let bundle = collect_support_bundle_in(&bundle_dir, &missing, &ProjectMeta::default())
            .expect("bundle");
        assert_eq!(bundle.files.len(), 3);
        let diag = read_to_string(&bundle_dir.join("diagnostics.json"));
        assert!(diag.contains("errors"));
    }

    #[test]
    fn bundle_compound_secrets_never_reach_any_output_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());
        let bundle_dir = tmp.path().join("bundle");

        // Adversarial values built from the REAL home so the normalization
        // is exercised on the machine running the test.
        let real_home = std::env::var("HOME").unwrap_or_else(|_| "/Users/demo".to_string());
        let compound_path = format!("{real_home}/mod/Defs/a.xml");

        // rw_version is a WHOLE-value secret — must be redacted by the
        // whole-payload sanitization, never fall through raw.
        let meta = ProjectMeta {
            name: Some("Test".into()),
            target_lang: None,
            rw_version: Some("sk-wholevalue-secret-9999".into()),
            rimloc_version: Some("0.1.0-alpha.1".into()),
            extra: serde_json::json!({
                "trace": format!("parsed {compound_path} then hit Bearer SYNTHBEARER123456"),
                "apiKey": "sk-test-0123456789abcdef",
            }),
        };

        // Failing operation whose causal chain carries the compound value.
        let mut failed = OperationLog::new("validate_po");
        failed.begin_stage("validate");
        failed.error(
            "validate",
            &WrappedError {
                root: RootError(format!(
                    "scan failed at {compound_path} with Bearer SYNTHCHAIN987654321"
                )),
            },
        );
        failed.end_stage("validate");

        let affected = vec![format!("{compound_path} with token=SYNTHAFF000111222333")];

        let bundle = collect_support_bundle_for(
            &SupportBundleInputs {
                scan_root: mod_root,
                project_meta: meta,
                operation: Some(failed),
                affected,
            },
            &bundle_dir,
        )
        .expect("bundle");

        // Zero synthetic keys and zero home prefixes in ALL four files.
        for name in [
            "report.md",
            "diagnostics.json",
            "environment.json",
            "manifest.json",
        ] {
            let content = read_to_string(&bundle_dir.join(name));
            for leaked in [
                "SYNTHBEARER123456",
                "SYNTHCHAIN987654321",
                "SYNTHAFF000111222333",
                "sk-wholevalue-secret-9999",
                "sk-test-0123456789abcdef",
                &real_home,
            ] {
                assert!(!content.contains(leaked), "{name} leaked {leaked}");
            }
        }

        // Diagnostic context survives: the normalized path and the masked
        // markers are visible where the values were kept.
        let report = read_to_string(&bundle_dir.join("report.md"));
        assert!(
            report.contains("~/mod/Defs/a.xml"),
            "context kept in report"
        );
        assert!(report.contains(REDACTION_MARKER));
        // 023 bug 1: a whole-value secret in a header field is masked in the
        // report instead of leaking raw, while the label survives.
        assert!(
            report.contains("RimWorld version: [REDACTED]"),
            "secret header masked, label kept"
        );
        assert!(report.contains(&bundle.operation_id));
        let env = read_to_string(&bundle_dir.join("environment.json"));
        assert!(
            !env.contains("sk-"),
            "environment rw_version must be redacted: {env}"
        );
        assert!(bundle.redacted.iter().any(|k| k.contains("apiKey")));
    }

    // --- output boundary: bundle must never write into the source ----------

    /// Recursive fingerprint of a tree (path + sha256 per file), so any
    /// unwanted write into the source changes the value.
    fn tree_fingerprint(root: &Path) -> String {
        let mut acc = String::new();
        for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            acc.push_str(&rel);
            acc.push('\n');
            if entry.file_type().is_file() {
                acc.push_str(&sha256_hex(&std::fs::read(path).unwrap_or_default()));
                acc.push('\n');
            }
        }
        sha256_hex(acc.as_bytes())
    }

    #[test]
    fn bundle_provider_tokens_never_reach_any_output_file() {
        // Table-driven over the PUBLIC bundle: synthetic provider tokens are
        // embedded in real validator-shaped contexts; every one of the four
        // output files must be clean while the surrounding causal message
        // survives.
        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());
        let bundle_dir = tmp.path().join("bundle");

        let mut failed = OperationLog::new("validate_po");
        failed.begin_stage("validate");
        failed.error_message(
            "validate",
            "provider upstream said ghp_0123456789abcdefghij and AKIA1234567890ABCDEF both rejected",
        );
        failed.end_stage("validate");

        let bundle = collect_support_bundle_for(
            &SupportBundleInputs {
                scan_root: mod_root,
                project_meta: ProjectMeta {
                    name: Some("Test".into()),
                    extra: serde_json::json!({
                        "note": "retry after gho_0123456789abcdefghij and xoxb-0123456789-abcdef failed",
                        "pat": "github_pat_0123456789abcdefghij",
                    }),
                    ..Default::default()
                },
                operation: Some(failed),
                affected: vec![],
            },
            &bundle_dir,
        )
        .expect("bundle");
        assert_eq!(bundle.files.len(), 3);

        let tokens = [
            "ghp_0123456789abcdefghij",
            "gho_0123456789abcdefghij",
            "github_pat_0123456789abcdefghij",
            "xoxb-0123456789-abcdef",
            "AKIA1234567890ABCDEF",
        ];
        for name in [
            "report.md",
            "diagnostics.json",
            "environment.json",
            "manifest.json",
        ] {
            let content = read_to_string(&bundle_dir.join(name));
            for t in tokens {
                assert!(!content.contains(t), "{name} leaked {t}");
            }
        }
        // Context survives in the human report.
        let report = read_to_string(&bundle_dir.join("report.md"));
        assert!(
            report.contains("both rejected"),
            "causal message context kept: {}",
            report
        );
        assert!(report.contains(REDACTION_MARKER));
    }

    #[test]
    fn bundle_output_inside_source_is_rejected_and_source_untouched() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());
        // Sentinel for the recursive-hash check.
        let about = mod_root.join("About");
        std::fs::create_dir_all(&about).expect("dirs");
        std::fs::write(about.join("About.xml"), "SENTINEL").expect("sentinel");
        let before = tree_fingerprint(&mod_root);

        let inside = mod_root.join("_bundle");
        let equal = mod_root.clone();
        // Not-yet-existing path with parent traversal — still inside.
        let traversal = mod_root.join("sub/../rimloc-support");

        for attempt in [&inside, &equal, &traversal] {
            let result = collect_support_bundle_in(attempt, &mod_root, &ProjectMeta::default());
            assert!(result.is_err(), "{attempt:?} must be rejected");
            assert_eq!(
                tree_fingerprint(&mod_root),
                before,
                "source unchanged after rejected attempt {attempt:?}"
            );
        }
        assert!(!inside.exists(), "no mkdir inside the source");
        assert!(!traversal.exists(), "no mkdir through traversal");

        // Symlink alias into the source tree → caught via canonical view.
        #[cfg(unix)]
        {
            let alias = tmp.path().join("alias");
            std::os::unix::fs::symlink(&mod_root, &alias).expect("symlink");
            let through_alias = alias.join("bundle");
            let result =
                collect_support_bundle_in(&through_alias, &mod_root, &ProjectMeta::default());
            assert!(
                result.is_err(),
                "symlink alias destination must be rejected"
            );
            assert!(!through_alias.exists());
            assert_eq!(tree_fingerprint(&mod_root), before);
        }

        // A normal separate output still succeeds and writes its files.
        let outside = tmp.path().join("bundle");
        let bundle = collect_support_bundle_in(&outside, &mod_root, &ProjectMeta::default())
            .expect("outside destination must succeed");
        assert_eq!(bundle.files.len(), 3);
        assert!(outside.join("manifest.json").exists());
        assert_eq!(tree_fingerprint(&mod_root), before);
    }

    #[test]
    fn bundle_rejects_sibling_traversal_and_planted_temp_symlinks() {
        // Lead probe shape through the PUBLIC API: base/source + base/out
        // are siblings; the traversal tail crosses back into the source.
        let tmp = tempfile::tempdir().expect("tmp");
        let base = tmp.path();
        let source = base.join("source");
        let keyed = source.join("Languages/English/Keyed");
        std::fs::create_dir_all(&keyed).expect("dirs");
        std::fs::write(
            keyed.join("Actions.xml"),
            "<LanguageData><K.label>v</K.label></LanguageData>",
        )
        .expect("xml");
        let sentinel = source.join("sentinel.txt");
        std::fs::write(&sentinel, b"KEEP").expect("sentinel");
        let out = base.join("out");
        std::fs::create_dir_all(&out).expect("dirs");

        let before = tree_fingerprint(&source);
        let traversal = out.join("missing/../../source/bundle");
        let result = collect_support_bundle_in(&traversal, &source, &ProjectMeta::default());
        assert!(result.is_err(), "sibling traversal must be rejected");
        assert!(
            !source.join("bundle").exists(),
            "nothing written into source"
        );
        assert_eq!(std::fs::read(&sentinel).expect("sentinel"), b"KEEP");
        assert_eq!(tree_fingerprint(&source), before, "source untouched");

        // Pre-planted temp symlinks (old predictable names) must not turn a
        // legitimate bundle write into a source overwrite.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&sentinel, out.join("environment.json.tmp.write"))
                .expect("plant old-name temp symlink");
            std::os::unix::fs::symlink(&sentinel, out.join(".environment.json.tmp.write"))
                .expect("plant dot-name temp symlink");
        }
        let safe_out = out;
        let bundle = collect_support_bundle_in(&safe_out, &source, &ProjectMeta::default())
            .expect("separate out dir succeeds");
        assert_eq!(bundle.files.len(), 3);
        assert_eq!(
            std::fs::read(&sentinel).expect("sentinel"),
            b"KEEP",
            "sentinel survives planted temp symlinks"
        );
        assert_eq!(tree_fingerprint(&source), before, "source still untouched");
    }

    #[test]
    fn default_collector_refuses_when_cwd_is_inside_source() {
        // current_dir is process-global: serialize cwd-mutating tests.
        static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = CWD_LOCK.lock().expect("cwd lock");

        let tmp = tempfile::tempdir().expect("tmp");
        let mod_root = make_mod_fixture(tmp.path());
        let prev = std::env::current_dir().expect("cwd");

        struct RestoreCwd(PathBuf);
        impl Drop for RestoreCwd {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _restore = RestoreCwd(prev);

        std::env::set_current_dir(&mod_root).expect("chdir into source");
        let result = collect_support_bundle(&mod_root, &ProjectMeta::default());
        assert!(
            result.is_err(),
            "default collector must refuse to write into the source"
        );
        assert!(mod_root.join("rimloc-support").read_dir().is_err());
    }
}
