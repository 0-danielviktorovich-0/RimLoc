//! Offline contribution bundle builder — the Rust half of the contribution
//! contract (docs/development/SELFLOC_BRIDGE.md, "Contribution Bundle";
//! owner mandate §5/§6/§11: translation-only payload, validation gate BEFORE
//! assembly, preview before sending).
//!
//! The contract authority is the TS pair
//! `gui/tauri-app/frontend-v2/scripts/contribution-schema.ts` (schema v1) and
//! `build-contribution.ts` (readiness statuses); this module is a 1:1 port of
//! the constants, the validators and the wire fields — a bundle written here
//! must parse STRICTLY against `parseContributionBundle` on the TS side
//! (schema fields exactly, no extras anywhere).
//!
//! Direction difference from the TS builder (recorded decision): the TS
//! builder reads an UNTRUSTED translator change file, so it runs a shape
//! sanitize pass first (foreign fields, malformed entries, conflicting
//! duplicates). This builder's input is the TRUSTED canonical session state
//! ([`crate::session::ProjectSessionManager`]) — those shapes hold by
//! construction — so the sanitize pass has no counterpart here; the §6 gate
//! and the readiness statuses are the contract that survives.
//!
//! Recorded decisions (task mandate "зафиксируй решение"):
//!
//! - `base_catalog_revision` is the REAL catalog revision from the project
//!   source root's `catalog.meta.json` (`catalog_revision`, written by the
//!   bridge exporter as a git sha with optional `-dirty`) — read, never
//!   invented. Missing/unparseable/REVISION_RE-violating meta is a
//!   NEEDS-FIXES root refusal.
//! - `base_value` (SF-1) carries the CURRENT en-catalog value of the entry
//!   (the source text the translation is against — the only snapshot an
//!   open GUI session can record honestly). Note for maintainers: the TS
//!   applier's SF-1 value-conflict gate compares `base_value` against its
//!   LIVE LOCALE dictionary (`src/i18n/<locale>.ts`), so a GUI-built bundle
//!   surfaces `value conflict (rebase_required)` for keys whose live locale
//!   value differs from en — fail-closed, a manual edit is never silently
//!   overwritten; rebase via `npm run build:contribution` resolves it.
//! - The session's translation locale uses the folder contract ("Russian")
//!   while the bundle tag is the TS tag ("ru"); the matcher mirrors the
//!   frontend's own slice rule (case-insensitive equality or the folder
//!   name's lowercase form starting with the tag — the exact rule
//!   `project.svelte.ts` applies to decide what the translator saw).

use std::collections::BTreeMap;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::contract::{ContractError, ContractErrorCode, SessionEpoch};
use crate::session::ProjectSessionManager;
use rimloc_validate::placeholder_set_mismatch;

// ---------------------------------------------------------------------------
// Contract constants (1:1 with contribution-schema.ts)
// ---------------------------------------------------------------------------

/// Bundle schema version; additive changes are minor, breaking ones bump this.
pub const BUNDLE_SCHEMA_VERSION: &str = "1";
/// Marker that the payload is a UI translation contribution, nothing else.
pub const BUNDLE_KIND: &str = "rimloc-ui-translation";
/// The source locale is authoritative and never accepts contributions.
pub const SOURCE_LOCALE: &str = "en";
/// Hard cap on a single translated value (current catalog max is 230 chars).
pub const VALUE_MAX_LEN: usize = 1000;
/// Hard cap on contributor display name / note length (SF-3).
pub const CONTRIBUTOR_NAME_MAX_LEN: usize = 80;
pub const CONTRIBUTOR_NOTE_MAX_LEN: usize = 500;
/// Hard cap on changes per bundle (catalog is ~1.2k messages; headroom).
pub const CHANGES_MAX_COUNT: usize = 5000;
/// The issue ref for root-level (non-entry) refusals — the TS builder's
/// `<root>` marker; `ref` itself is a Rust keyword, `id` carries it.
pub const ROOT_REF: &str = "<root>";

/// Locale tag shape: lowercase language, optional subtags. Deliberately
/// rejects `/`, `\`, `..`, whitespace — the tag names the bundle file and is
/// the applier's dictionary path component, so traversal must be impossible.
pub fn is_locale_form(locale: &str) -> bool {
    static RE_LOCALE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[a-z]{2,3}(?:-[A-Za-z0-9]+)*$").unwrap());
    RE_LOCALE.is_match(locale)
}

/// catalog_revision / base_catalog_revision shape: git sha (7-40 hex) with
/// optional `-dirty`.
pub fn is_revision_form(revision: &str) -> bool {
    static RE_REVISION: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[0-9a-f]{7,40}(?:-dirty)?$").unwrap());
    RE_REVISION.is_match(revision)
}

/// Change/message id shape (SF-2): ASCII letters, digits, dot, dash, slash,
/// underscore; 1-200 chars; no leading/trailing dots — an id must be safe to
/// address as a dictionary key and to rewrite.
pub fn is_valid_change_id(id: &str) -> bool {
    static RE_CHANGE_ID: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[A-Za-z0-9_.\-/]{1,200}$").unwrap());
    RE_CHANGE_ID.is_match(id) && !id.starts_with('.') && !id.ends_with('.')
}

/// True when the locale may receive translations (well-formed, not source).
pub fn is_contributable_locale(locale: &str) -> bool {
    is_locale_form(locale) && locale != SOURCE_LOCALE
}

/// Control characters are never legitimate in values or contributor
/// metadata. The TS class `[\u0000-\u0008\u000B\u000C\u000E-\u001F]`
/// deliberately keeps tab/CR/LF legal — mirrored here exactly.
pub fn has_control_chars(text: &str) -> bool {
    text.chars().any(|c| {
        let u = c as u32;
        (0x00..=0x08).contains(&u) || u == 0x0B || u == 0x0C || (0x0E..=0x1F).contains(&u)
    })
}

// ---------------------------------------------------------------------------
// Secret scan — PATTERN NAMES only, never the matched text
// ---------------------------------------------------------------------------

/// Secret patterns of the contract. A hit is reported by NAME only, so a
/// refusal can be surfaced without leaking the secret into logs, previews or
/// test output.
static SECRET_PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        ("aws-access-key", Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap()),
        (
            "github-token",
            Regex::new(r"\bgh[oprsu]_[A-Za-z0-9]{30,}\b").unwrap(),
        ),
        (
            "github-fine-grained-token",
            Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{60,}\b").unwrap(),
        ),
        (
            "slack-token",
            Regex::new(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b").unwrap(),
        ),
        (
            "api-key-prefix",
            Regex::new(r"\bsk-[A-Za-z0-9_-]{20,}\b").unwrap(),
        ),
        (
            "credential-assignment",
            Regex::new(
                r"(?i)\b(?:password|passwd|secret|token|api[_-]?key|access[_-]?key)\b\s*[:=]\s*\S{8,}",
            )
            .unwrap(),
        ),
        (
            "private-key-block",
            Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----").unwrap(),
        ),
    ]
});

/// Pattern names whose signature the text matches (empty = clean).
pub fn scan_secrets(text: &str) -> Vec<&'static str> {
    SECRET_PATTERNS
        .iter()
        .filter(|(_, re)| re.is_match(text))
        .map(|(name, _)| *name)
        .collect()
}

// ---------------------------------------------------------------------------
// Wire types — serde field names ARE the schema fields
// ---------------------------------------------------------------------------

/// One translation change. Exactly these three fields, nothing else.
/// `base_value` (SF-1) records the source value the translation is against:
/// the applier's value-conflict gate refuses to run when the live value has
/// moved on (a stale bundle must never overwrite a manual edit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleChange {
    pub id: String,
    pub base_value: String,
    pub value: String,
}

/// Optional attribution. Exactly these fields when present. The GUI beta
/// slice collects no contributor metadata — the builder emits `None` and the
/// field is skipped, which `parseContributionBundle` accepts (`contributor`
/// is optional).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleContributor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The contribution bundle: translation data only (schema v1). No executable
/// code, no file paths, no tool metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionBundle {
    pub schema_version: String,
    pub kind: String,
    pub locale: String,
    pub base_catalog_revision: String,
    pub changes: Vec<BundleChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributor: Option<BundleContributor>,
}

/// Readiness statuses of build-contribution.ts: READY (everything valid) /
/// PARTIAL-BUT-VALID (structurally valid subset bundled, refusals
/// enumerated) / NEEDS-FIXES (structural breakage or zero valid — NO bundle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum BundleStatus {
    Ready,
    PartialButValid,
    NeedsFixes,
}

impl BundleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BundleStatus::Ready => "READY",
            BundleStatus::PartialButValid => "PARTIAL-BUT-VALID",
            BundleStatus::NeedsFixes => "NEEDS-FIXES",
        }
    }
}

/// A precise, translator-actionable rejection reason. Never echoes secrets.
/// (`id` carries the TS builder's `ref`: a change id, or [`ROOT_REF`] when
/// no usable id exists.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub id: String,
    pub reason: String,
}

impl ValidationIssue {
    fn root(reason: impl Into<String>) -> Self {
        ValidationIssue {
            id: ROOT_REF.to_string(),
            reason: reason.into(),
        }
    }
}

/// The pure §6 gate for one change against the en catalog — 1:1 with
/// `validateChange` in build-contribution.ts, with the placeholder contract
/// delegated to the wave-6 checker [`placeholder_set_mismatch`] (the same
/// strict `{name}`-set comparison the session validator runs on UI-catalog
/// entries). Returns the rejection reason, `None` when valid.
pub fn validate_change(
    id: &str,
    value: &str,
    en_catalog: &BTreeMap<String, String>,
) -> Option<String> {
    let Some(source) = en_catalog.get(id) else {
        return Some("id does not exist in the en catalog (new keys are not contributable)".into());
    };
    if value.is_empty() {
        return Some("value is empty (omit the change instead)".into());
    }
    if value.chars().count() > VALUE_MAX_LEN {
        return Some(format!("value exceeds the {VALUE_MAX_LEN}-character limit"));
    }
    if has_control_chars(value) {
        return Some("value contains control characters".into());
    }
    if let Some(message) = placeholder_set_mismatch(source, value) {
        return Some(message);
    }
    let secrets = scan_secrets(value);
    if !secrets.is_empty() {
        return Some(format!(
            "value matches secret pattern(s) {secrets:?} — credentials never enter a bundle"
        ));
    }
    None
}

/// Pure bundle assembly from already-validated changes: schema fields only,
/// changes sorted by id (the canonical order — deterministic output).
pub fn assemble_bundle(
    locale: &str,
    valid_changes: Vec<BundleChange>,
    base_catalog_revision: &str,
) -> ContributionBundle {
    let mut changes = valid_changes;
    changes.sort_by(|a, b| a.id.cmp(&b.id));
    ContributionBundle {
        schema_version: BUNDLE_SCHEMA_VERSION.to_string(),
        kind: BUNDLE_KIND.to_string(),
        locale: locale.to_string(),
        base_catalog_revision: base_catalog_revision.to_string(),
        changes,
        contributor: None,
    }
}

// ---------------------------------------------------------------------------
// Session-facing build
// ---------------------------------------------------------------------------

/// The gathered view of an OPEN session the builder consumes: the UI-catalog
/// EN canon (id → source text, only entries whose provenance is
/// `selected_by = ui_catalog`), the requested locale's candidate changes
/// (catalog key → non-empty translation text), and the read-only source root
/// (the revision source AND the write-guard's denied tree). Assembled by
/// [`ProjectSessionManager::contribution_source`].
#[derive(Debug, Clone, Default)]
pub struct ContributionSource {
    pub catalog_en: BTreeMap<String, String>,
    pub changes: Vec<(String, String)>,
    pub mod_root: std::path::PathBuf,
}

/// The catalog meta field the revision is read from
/// (SELFLOC_BRIDGE.md: `catalog.meta.json.catalog_revision`).
#[derive(serde::Deserialize)]
struct CatalogMetaRevision {
    #[serde(default)]
    catalog_revision: Option<String>,
}

/// The REAL catalog revision of a catalog source directory, read from
/// `catalog.meta.json` — `None` when the file is missing or unparseable
/// (the caller refuses honestly; a revision is never invented).
pub fn catalog_revision_of(root: &Path) -> Option<String> {
    let bytes = std::fs::read(root.join(crate::ui_catalog::CATALOG_META_FILE)).ok()?;
    let meta: CatalogMetaRevision = serde_json::from_slice(&bytes).ok()?;
    meta.catalog_revision
}

/// The pure gate + assembly over gathered session data (everything except
/// the write): root gates first (contributable locale, catalog presence,
/// revision form, per-bundle cap), then the §6 gate per change. Mirrors
/// build-contribution.ts's accumulate-then-decide: NEEDS-FIXES carries the
/// full issue list and no bundle.
pub fn build_outcome(locale: &str, source: &ContributionSource) -> ContributionOutcome {
    let mut issues: Vec<ValidationIssue> = Vec::new();

    // Root gate 1: the bundle locale must be contributable (well-formed tag,
    // never the source locale).
    let locale_ok = is_contributable_locale(locale);
    if !locale_ok {
        issues.push(ValidationIssue::root(format!(
            "locale `{locale}` is not contributable (must match a locale tag and must not be the source locale \"{SOURCE_LOCALE}\")"
        )));
    }

    // Root gate 2: the open project must BE the UI-catalog project. A mod
    // project has no ui_catalog entries — the contribution flow does not
    // apply to it (the Rust-side selfloc guard).
    let catalog_ok = !source.catalog_en.is_empty();
    if !catalog_ok {
        issues.push(ValidationIssue::root(
            "the open project has no UI-catalog entries (selected_by = ui_catalog) — the contribution flow is only available on the RimLoc UI catalog project",
        ));
    }

    // Root gate 3: the base revision is read from the catalog meta, never
    // invented; it must satisfy the applier's strict revision shape.
    let revision = catalog_revision_of(&source.mod_root);
    let revision_ok = revision.as_deref().is_some_and(is_revision_form);
    if !revision_ok {
        issues.push(ValidationIssue::root(
            "base catalog revision unavailable: the project source root has no parseable catalog.meta.json catalog_revision (git sha, optional -dirty) — rebuild the project from a RimLoc UI catalog source",
        ));
    }

    // Root gate 4: per-bundle cap (generous headroom over the ~1.2k catalog).
    let over_limit = source.changes.len() > CHANGES_MAX_COUNT;
    if over_limit {
        issues.push(ValidationIssue::root(format!(
            "changes exceeds {CHANGES_MAX_COUNT} entries"
        )));
    }

    // SF-4: conflicting duplicates cannot occur (one translation per
    // (identity, locale) in the canonical state) — the gather would have to
    // invent them; nothing to check here by construction.

    let mut valid: Vec<BundleChange> = Vec::new();
    if !over_limit {
        for (id, value) in &source.changes {
            if let Some(reason) = validate_change(id, value, &source.catalog_en) {
                issues.push(ValidationIssue {
                    id: id.clone(),
                    reason,
                });
                continue;
            }
            // SF-1 base_value: the CURRENT en-catalog value (recorded
            // decision — see the module docs for the applier semantics).
            let base_value = source.catalog_en.get(id).cloned().unwrap_or_default();
            valid.push(BundleChange {
                id: id.clone(),
                base_value,
                value: value.clone(),
            });
        }
    }

    if !locale_ok || !catalog_ok || !revision_ok || over_limit || valid.is_empty() {
        return ContributionOutcome {
            status: BundleStatus::NeedsFixes,
            bundle: None,
            issues,
            accepted_count: 0,
        };
    }

    let status = if issues.is_empty() {
        BundleStatus::Ready
    } else {
        BundleStatus::PartialButValid
    };
    let accepted_count = valid.len();
    let bundle = assemble_bundle(locale, valid, revision.as_deref().unwrap_or_default());
    ContributionOutcome {
        status,
        bundle: Some(bundle),
        issues,
        accepted_count,
    }
}

/// The outcome of the pure build: status, the bundle (READY /
/// PARTIAL-BUT-VALID only), the enumerated refusals and the accepted count.
#[derive(Debug, Clone)]
pub struct ContributionOutcome {
    pub status: BundleStatus,
    pub bundle: Option<ContributionBundle>,
    pub issues: Vec<ValidationIssue>,
    pub accepted_count: usize,
}

/// The command DTO: what the GUI shows after a build attempt. `bundle_path`
/// is `None` exactly when the status is NEEDS-FIXES (no file is written).
#[derive(Debug, Clone, Serialize)]
pub struct BuildContributionResponse {
    pub status: BundleStatus,
    pub bundle_path: Option<String>,
    pub accepted_count: usize,
    pub rejected: Vec<ValidationIssue>,
}

/// The bundle file name inside the caller-chosen directory.
pub fn bundle_file_name(locale: &str) -> String {
    format!("rimloc-ui-contribution-{locale}.json")
}

/// Build the contribution bundle from the OPEN session and write it into the
/// caller-chosen directory. Guards mirror `export_project` exactly: the out
/// dir must be ABSOLUTE (form, before any filesystem access) and must not
/// sit inside the read-only source tree nor inside the managed root
/// (fail-closed containment). NEEDS-FIXES writes nothing.
pub fn build_contribution(
    manager: &ProjectSessionManager,
    project_id: &str,
    session_epoch: SessionEpoch,
    out_dir: &Path,
    locale: &str,
) -> Result<BuildContributionResponse, ContractError> {
    let source = manager.contribution_source(project_id, session_epoch, locale)?;
    let outcome = build_outcome(locale, &source);

    let Some(bundle) = outcome.bundle else {
        return Ok(BuildContributionResponse {
            status: outcome.status,
            bundle_path: None,
            accepted_count: 0,
            rejected: outcome.issues,
        });
    };

    // --- write path: the export_project guard partition, verbatim --------
    // Path-FORM guard BEFORE any guard, canonicalization or write (P1-2):
    // a relative path would land wherever the GUI process was launched from.
    if !out_dir.is_absolute() {
        return Err(ContractError::invalid_output_path(out_dir));
    }
    let mod_root = &source.mod_root;
    if mod_root.as_os_str().is_empty() {
        return Err(ContractError::new(
            ContractErrorCode::GuardOutputDenied,
            "the session has no source root recorded; the read-only source-tree guard cannot be established — re-create the project from its source mod".to_string(),
        ));
    }
    if crate::is_within(out_dir, mod_root) {
        return Err(ContractError::new(
            ContractErrorCode::GuardOutputDenied,
            format!(
                "output directory `{}` is inside the read-only source tree `{}`",
                out_dir.display(),
                mod_root.display()
            ),
        ));
    }
    if crate::is_within(out_dir, manager.managed_root()) {
        return Err(ContractError::new(
            ContractErrorCode::GuardOutputDenied,
            format!(
                "output directory `{}` is inside the managed projects root `{}`",
                out_dir.display(),
                manager.managed_root().display()
            ),
        ));
    }

    let path = out_dir.join(bundle_file_name(locale));
    std::fs::create_dir_all(out_dir).map_err(|e| {
        ContractError::new(
            ContractErrorCode::Internal,
            format!(
                "cannot create output directory `{}`: {e}",
                out_dir.display()
            ),
        )
    })?;
    // 2-space pretty + trailing newline — the TS writer's byte shape
    // (JSON.stringify(bundle, null, 2) + "\n").
    let mut body = serde_json::to_vec_pretty(&bundle).map_err(|e| {
        ContractError::new(
            ContractErrorCode::Internal,
            format!("cannot serialize the contribution bundle: {e}"),
        )
    })?;
    body.push(b'\n');
    if let Err(e) = std::fs::write(&path, &body) {
        return Err(ContractError::new(
            ContractErrorCode::Internal,
            format!("cannot write `{}`: {e}", path.display()),
        ));
    }

    Ok(BuildContributionResponse {
        status: outcome.status,
        bundle_path: Some(path.display().to_string()),
        accepted_count: outcome.accepted_count,
        rejected: outcome.issues,
    })
}

// ---------------------------------------------------------------------------
// Tests — the task's mandated coverage: READY happy-path (deserialized by
// the same types), PARTIAL (valid subset + enumerated refusals), NEEDS-FIXES
// (zero valid), secret hit (pattern name only), base_value recorded, limits,
// control chars, locale validation.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ApplyIntentsRequest, IntentAction, TranslationIntent};
    use crate::session::ProjectSessionManager;
    use rimloc_domain::canonical::{EntryKind, SourceEntryId};
    use std::path::PathBuf;

    const REVISION: &str = "5837dfeb77ba5f3844f6208a3c78c0cac8951857-dirty";

    fn write_catalog(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(crate::ui_catalog::CATALOG_SOURCE_FILE),
            r#"{
  "schema_version": "1",
  "messages": [
    { "id": "common.appName", "source_text": "RimLoc", "placeholders": [] },
    { "id": "common.close", "source_text": "Close", "placeholders": [] },
    { "id": "home.greeting", "source_text": "Welcome, {name}!", "placeholders": ["name"] },
    { "id": "home.count", "source_text": "{count} items", "placeholders": ["count"] }
  ]
}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join(crate::ui_catalog::CATALOG_META_FILE),
            format!(r#"{{"schema_version":"1","catalog_revision":"{REVISION}"}}"#),
        )
        .unwrap();
    }

    fn set_intent(key: &str, text: &str) -> TranslationIntent {
        TranslationIntent {
            entry: SourceEntryId {
                kind: EntryKind::Keyed,
                key: key.to_string(),
                def_type: None,
            },
            locale: "Russian".to_string(),
            action: IntentAction::SetTranslation,
            text: Some(text.to_string()),
        }
    }

    /// A manager with an OPEN selfloc session: the small catalog + the given
    /// translations applied under the folder-contract locale "Russian".
    fn selfloc_session(
        root: &Path,
        translations: &[(&str, &str)],
    ) -> (ProjectSessionManager, String, u64, PathBuf) {
        let catalog_dir = root.join("catalog");
        write_catalog(&catalog_dir);
        let mgr = ProjectSessionManager::new(root.join("managed")).expect("managed root");
        let snap = mgr.create(&catalog_dir, None).expect("create");
        if !translations.is_empty() {
            let intents: Vec<TranslationIntent> =
                translations.iter().map(|(k, v)| set_intent(k, v)).collect();
            let res = mgr
                .apply(&ApplyIntentsRequest {
                    project_id: snap.project_id.clone(),
                    expected_revision: snap.revision,
                    session_epoch: snap.session_epoch,
                    intents,
                })
                .expect("apply");
            assert_eq!(
                res.applied,
                translations.len(),
                "every fixture intent applies"
            );
        }
        let epoch = snap.session_epoch;
        (mgr, snap.project_id, epoch, catalog_dir)
    }

    #[test]
    fn ready_happy_path_round_trips_through_the_same_types() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let (mgr, pid, epoch, _src) = selfloc_session(
            dir.path(),
            &[
                ("common.close", "Закрыть"),
                ("home.greeting", "Привет, {name}!"),
            ],
        );
        let resp = build_contribution(&mgr, &pid, epoch, &out, "ru").expect("build");
        assert_eq!(resp.status, BundleStatus::Ready);
        assert_eq!(resp.accepted_count, 2);
        assert!(resp.rejected.is_empty());
        let path = resp.bundle_path.expect("READY writes the bundle");
        assert!(
            path.ends_with("rimloc-ui-contribution-ru.json"),
            "bundle file name carries the locale tag: {path}"
        );
        // The bundle deserializes through THE SAME wire types (field names
        // are the schema) and carries the exact schema fields — no extras.
        let bytes = std::fs::read(&path).unwrap();
        let bundle: ContributionBundle = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(bundle.schema_version, BUNDLE_SCHEMA_VERSION);
        assert_eq!(bundle.kind, BUNDLE_KIND);
        assert_eq!(bundle.locale, "ru");
        assert_eq!(bundle.base_catalog_revision, REVISION);
        assert!(bundle.contributor.is_none());
        // Canonical order: by id.
        let ids: Vec<&str> = bundle.changes.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["common.close", "home.greeting"]);
        // SF-1: base_value records the CURRENT en-catalog value.
        let close = &bundle.changes[0];
        assert_eq!(close.base_value, "Close");
        assert_eq!(close.value, "Закрыть");
        // The JSON field set is exactly the schema's (applier compatibility).
        let raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut root_keys: Vec<&str> = raw
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        root_keys.sort_unstable();
        assert_eq!(
            root_keys,
            vec![
                "base_catalog_revision",
                "changes",
                "kind",
                "locale",
                "schema_version"
            ]
        );
        let mut change_keys: Vec<&str> = raw["changes"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        change_keys.sort_unstable();
        assert_eq!(change_keys, vec!["base_value", "id", "value"]);
    }

    #[test]
    fn partial_bundles_the_valid_subset_and_enumerates_refusals() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let (mgr, pid, epoch, _) = selfloc_session(
            dir.path(),
            &[
                ("common.close", "Закрыть"),
                // Placeholder lost: the wave-6 strict set comparison fires.
                ("home.greeting", "Привет!"),
            ],
        );
        let resp = build_contribution(&mgr, &pid, epoch, &out, "ru").expect("build");
        assert_eq!(resp.status, BundleStatus::PartialButValid);
        assert_eq!(resp.accepted_count, 1);
        let by_id: BTreeMap<&str, &str> = resp
            .rejected
            .iter()
            .map(|i| (i.id.as_str(), i.reason.as_str()))
            .collect();
        assert!(
            by_id["home.greeting"].contains("Placeholder mismatch vs source"),
            "wave-6 placeholder reason expected: {}",
            by_id["home.greeting"]
        );
        // The file holds exactly the valid subset.
        let bundle: ContributionBundle =
            serde_json::from_slice(&std::fs::read(resp.bundle_path.unwrap()).unwrap()).unwrap();
        assert_eq!(bundle.changes.len(), 1);
        assert_eq!(bundle.changes[0].id, "common.close");
    }

    /// The empty-value and control-char gate branches are UNREACHABLE through
    /// an open session by design — `apply` refuses empty texts and XML-1.0-
    /// invalid control characters at emission (H3), and the TS control class
    /// keeps only \t\n\r legal, which are all XML-legal. The gate keeps them
    /// as defense in depth (the applier re-runs it against hand-made
    /// bundles), asserted here on the pure outcome.
    #[test]
    fn empty_and_control_char_and_limit_branches_of_the_pure_gate() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path());
        let mut catalog_en = BTreeMap::new();
        catalog_en.insert("a".to_string(), "A".to_string());
        let long = "ж".repeat(VALUE_MAX_LEN + 1);
        let source = ContributionSource {
            catalog_en,
            changes: vec![
                ("a".to_string(), String::new()),
                ("a".to_string(), "За\u{0007}крыть".to_string()),
                ("a".to_string(), long),
                ("a".to_string(), "Закрыть".to_string()),
            ],
            mod_root: dir.path().to_path_buf(),
        };
        let outcome = build_outcome("ru", &source);
        let reasons: Vec<&str> = outcome.issues.iter().map(|i| i.reason.as_str()).collect();
        assert!(reasons.contains(&"value is empty (omit the change instead)"));
        assert!(reasons.contains(&"value contains control characters"));
        assert!(reasons
            .contains(&format!("value exceeds the {VALUE_MAX_LEN}-character limit").as_str()));
        assert_eq!(outcome.status, BundleStatus::PartialButValid);
        assert_eq!(outcome.accepted_count, 1);
        assert_eq!(outcome.bundle.unwrap().changes[0].value, "Закрыть");
    }

    #[test]
    fn needs_fixes_when_zero_valid_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out");
        let (mgr, pid, epoch, _) =
            selfloc_session(dir.path(), &[("home.greeting", "Привет без плейсхолдера")]);
        let resp = build_contribution(&mgr, &pid, epoch, &out, "ru").expect("build");
        assert_eq!(resp.status, BundleStatus::NeedsFixes);
        assert_eq!(resp.accepted_count, 0);
        assert!(resp.bundle_path.is_none(), "NEEDS-FIXES writes no bundle");
        assert!(!out.exists(), "no directory is created for a refusal");
        assert!(resp
            .rejected
            .iter()
            .any(|i| i.id == "home.greeting" && i.reason.contains("Placeholder mismatch")));
    }

    #[test]
    fn secret_hit_rejects_with_pattern_name_only() {
        let dir = tempfile::tempdir().unwrap();
        let secret = "my key is sk-abcdef1234567890abcdef end";
        let (mgr, pid, epoch, _) = selfloc_session(dir.path(), &[("common.close", secret)]);
        let resp =
            build_contribution(&mgr, &pid, epoch, &dir.path().join("out"), "ru").expect("build");
        assert_eq!(resp.status, BundleStatus::NeedsFixes);
        let issue = resp
            .rejected
            .iter()
            .find(|i| i.id == "common.close")
            .expect("the secret entry is refused");
        assert!(
            issue.reason.contains("api-key-prefix"),
            "the reason names the pattern: {}",
            issue.reason
        );
        assert!(
            !issue.reason.contains("sk-abcdef"),
            "the matched text must never be echoed: {}",
            issue.reason
        );
    }

    #[test]
    fn locale_gates_source_and_malformed_tags() {
        let dir = tempfile::tempdir().unwrap();
        let (mgr, pid, epoch, _) = selfloc_session(dir.path(), &[("common.close", "Закрыть")]);
        for locale in ["en", "Ru", "../escape", "ru RU", "ru/ru"] {
            let resp = build_contribution(&mgr, &pid, epoch, &dir.path().join("out"), locale)
                .expect("locale refusals are data, not transport errors");
            assert_eq!(
                resp.status,
                BundleStatus::NeedsFixes,
                "locale `{locale}` must refuse"
            );
            assert!(resp.bundle_path.is_none());
            assert!(
                resp.rejected.iter().any(|i| i.id == ROOT_REF),
                "the refusal is a root issue"
            );
        }
    }

    #[test]
    fn non_catalog_project_is_a_root_refusal() {
        let dir = tempfile::tempdir().unwrap();
        // A plain RimWorld mod (no catalog.*) — the ordinary mod scanner path.
        let mod_root = dir.path().join("mod");
        std::fs::create_dir_all(mod_root.join("Languages/English/Keyed")).unwrap();
        std::fs::write(
            mod_root.join("Languages/English/Keyed/K.xml"),
            "<LanguageData><K.label>hi</K.label></LanguageData>",
        )
        .unwrap();
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, None).unwrap();
        let resp = build_contribution(
            &mgr,
            &snap.project_id,
            snap.session_epoch,
            &dir.path().join("out"),
            "ru",
        )
        .expect("build");
        assert_eq!(resp.status, BundleStatus::NeedsFixes);
        assert!(
            resp.rejected
                .iter()
                .any(|i| i.reason.contains("no UI-catalog entries")),
            "the Rust-side selfloc gate fires: {:?}",
            resp.rejected
        );
    }

    #[test]
    fn broken_catalog_revision_is_a_root_refusal_never_invented() {
        let dir = tempfile::tempdir().unwrap();
        let catalog_dir = dir.path().join("catalog");
        write_catalog(&catalog_dir);
        // A revision outside REVISION_RE: the applier would refuse the bundle,
        // so the builder must refuse first.
        std::fs::write(
            catalog_dir.join(crate::ui_catalog::CATALOG_META_FILE),
            r#"{"schema_version":"1","catalog_revision":"NOT-A-SHA"}"#,
        )
        .unwrap();
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&catalog_dir, None).unwrap();
        let res = mgr
            .apply(&ApplyIntentsRequest {
                project_id: snap.project_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                intents: vec![set_intent("common.close", "Закрыть")],
            })
            .unwrap();
        assert_eq!(res.applied, 1);
        let resp = build_contribution(
            &mgr,
            &snap.project_id,
            snap.session_epoch,
            &dir.path().join("out"),
            "ru",
        )
        .expect("build");
        assert_eq!(resp.status, BundleStatus::NeedsFixes);
        assert!(resp
            .rejected
            .iter()
            .any(|i| i.id == ROOT_REF && i.reason.contains("base catalog revision unavailable")));
    }

    #[test]
    fn write_guards_refuse_relative_source_tree_and_managed_root() {
        let dir = tempfile::tempdir().unwrap();
        let (mgr, pid, epoch, catalog_dir) =
            selfloc_session(dir.path(), &[("common.close", "Закрыть")]);

        // Relative out dir: the FORM refusal fires before any write.
        let err = build_contribution(&mgr, &pid, epoch, Path::new("rel/out"), "ru")
            .expect_err("relative path refused");
        assert_eq!(err.code, ContractErrorCode::InvalidOutputPath);

        // Inside the read-only source tree.
        let err = build_contribution(&mgr, &pid, epoch, &catalog_dir.join("sub"), "ru")
            .expect_err("source-tree target refused");
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied);

        // Inside the managed projects root.
        let err = build_contribution(&mgr, &pid, epoch, &mgr.managed_root().join("sub"), "ru")
            .expect_err("managed-root target refused");
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied);
    }

    #[test]
    fn stale_epoch_and_unknown_project_are_typed_transport_errors() {
        let dir = tempfile::tempdir().unwrap();
        let (mgr, pid, epoch, _) = selfloc_session(dir.path(), &[("common.close", "Закрыть")]);
        let err = build_contribution(&mgr, &pid, epoch + 1, &dir.path().join("out"), "ru")
            .expect_err("stale epoch refused");
        assert_eq!(err.code, ContractErrorCode::StaleEpoch);
        let err = build_contribution(&mgr, "proj-absent", epoch, &dir.path().join("out"), "ru")
            .expect_err("unknown project refused");
        assert_eq!(err.code, ContractErrorCode::ProjectNotFound);
    }

    #[test]
    fn folder_contract_locale_is_matched_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        // Session translations live under the folder-contract locale
        // ("Russian"); the bundle tag is the TS tag ("ru"). The matcher must
        // bridge the two — this is the same slice rule the frontend applies.
        let (mgr, pid, epoch, _) = selfloc_session(dir.path(), &[("common.close", "Закрыть")]);
        let source = mgr.contribution_source(&pid, epoch, "ru").expect("gather");
        assert_eq!(
            source.changes,
            vec![("common.close".to_string(), "Закрыть".to_string())]
        );
        // An unrelated tag collects nothing (no silent cross-locale bleed).
        let other = mgr.contribution_source(&pid, epoch, "uk").expect("gather");
        assert!(other.changes.is_empty());
    }

    #[test]
    fn todo_placeholders_never_enter_the_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let catalog_dir = dir.path().join("catalog");
        write_catalog(&catalog_dir);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&catalog_dir, None).unwrap();
        mgr.apply(&ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: snap.revision,
            session_epoch: snap.session_epoch,
            intents: vec![TranslationIntent {
                entry: SourceEntryId {
                    kind: EntryKind::Keyed,
                    key: "common.close".to_string(),
                    def_type: None,
                },
                locale: "Russian".to_string(),
                action: IntentAction::MarkTodo,
                text: None,
            }],
        })
        .unwrap();
        // The TODO placeholder is "missing" by the domain's own counting —
        // it must not ride into a contribution bundle as a translation.
        let source = mgr
            .contribution_source(&snap.project_id, snap.session_epoch, "ru")
            .expect("gather");
        assert!(source.changes.is_empty(), "{:?}", source.changes);
    }

    // --- pure-contract parity with the TS schema -------------------------

    #[test]
    fn contract_validators_match_the_ts_regexes() {
        // LOCALE_RE ^[a-z]{2,3}(?:-[A-Za-z0-9]+)*$
        for ok in ["ru", "en", "uk", "rus", "pt-BR", "zh-Hans"] {
            assert!(is_locale_form(ok), "{ok}");
        }
        for bad in ["Ru", "", "ru-RU-", "ru_RU", "ru/ru", "ru ..", "r"] {
            assert!(!is_locale_form(bad), "{bad}");
        }
        // REVISION_RE ^[0-9a-f]{7,40}(?:-dirty)?$
        assert!(is_revision_form("abcdef1"));
        assert!(is_revision_form("abcdef1-dirty"));
        assert!(is_revision_form(&"a".repeat(40)));
        assert!(!is_revision_form("abcdef"));
        assert!(!is_revision_form(&"a".repeat(41)));
        assert!(!is_revision_form("ABCDEF1"));
        assert!(!is_revision_form("abcdef1-dirtyx"));
        // CHANGE_ID_RE + no leading/trailing dots.
        assert!(is_valid_change_id("common.close"));
        assert!(is_valid_change_id("a/b_c-d.e"));
        assert!(!is_valid_change_id(".lead"));
        assert!(!is_valid_change_id("trail."));
        assert!(!is_valid_change_id("has space"));
        assert!(!is_valid_change_id(""));
        assert!(!is_valid_change_id(&"a".repeat(201)));
    }

    #[test]
    fn secret_scan_names_only_and_control_char_class_matches_ts() {
        assert!(scan_secrets("AKIAIOSFODNN7EXAMPLE").contains(&"aws-access-key"));
        assert!(
            scan_secrets("token: ghp_abcabcabcabcabcabcabcabcabcabcabcabc")
                .contains(&"github-token")
        );
        assert!(scan_secrets("-----BEGIN RSA PRIVATE KEY-----").contains(&"private-key-block"));
        assert!(scan_secrets("password = superlongsecret").contains(&"credential-assignment"));
        assert!(scan_secrets("just a normal sentence").is_empty());
        // The TS class keeps \t \n \r legal and refuses everything else below
        // 0x20 outside them.
        assert!(!has_control_chars("a\tb\nc\rd"));
        assert!(has_control_chars("a\u{0000}b"));
        assert!(has_control_chars("a\u{0007}b"));
        assert!(has_control_chars("a\u{001F}b"));
        assert!(!has_control_chars("a\u{007F}b"), "DEL is outside the class");
    }
}
