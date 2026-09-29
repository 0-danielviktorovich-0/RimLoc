//! Self-localization entry (mandate D, first wave): the app's own generated
//! UI catalog (`gui/tauri-app/frontend-v2/src/i18n/generated/`,
//! SELFLOC_BRIDGE.md) becomes an ORDINARY project source. The adapter side
//! already exists — `rimloc_services::ui_catalog` routes a directory that
//! contains a parseable `catalog.en.json` + schema-"1" meta through the
//! catalog inventory INSTEAD of the mod scanner, so the session create-path
//! accepts it unchanged. This module supplies the one missing piece: WHERE
//! the packaged app finds that directory.
//!
//! Resource strategy (documented decision, hybrid "bundle → copy to
//! app_data"):
//!
//! 1. SOURCE candidates, first parseable wins:
//!    a. the bundled resource `<resource_dir>/selfloc-catalog` (declared in
//!       `tauri.conf.json` → `bundle.resources` map form; the Tauri bundler
//!       copies the generated JSON there at package time);
//!    b. DEBUG BUILDS ONLY (`#[cfg(debug_assertions)]`): the repository path
//!       next to this crate (`../frontend-v2/src/i18n/generated`). Dev runs
//!       (`tauri dev`, `cargo run`) are the whole night-automation surface,
//!       and a compile-time repo path removes any dependence on whether the
//!       dev runner reproduces the bundle resource layout. It is compiled
//!       OUT of release builds — no build-machine path ever ships.
//! 2. DESTINATION: `<app_data>/RimLoc/selfloc-catalog/RimLoc UI (en)/`.
//!    The copy exists so the project has a STABLE, user-visible source root
//!    that does not move with app updates or resource-dir platform quirks,
//!    and so the project display name (derived by `session::create` from the
//!    directory basename) reads "RimLoc UI (en)" — the mandate's project
//!    name — without special-casing the session layer.
//! 3. The copy is IDEMPOTENT and revision-driven: `catalog.meta.json`'s
//!    `catalog_revision` decides. Same revision → nothing is written;
//!    missing/stale/broken dest → the dest directory (an app-managed CACHE,
//!    never user content — translations live in the managed project file)
//!    is replaced wholesale with all `catalog.*.json` files.
//!
//! Recognition is deliberately strict: a candidate that does not parse as a
//! catalog source is skipped, and total failure is a typed refusal — the
//! entry point never invents a directory that would later fail create with
//! a confusing zero-entry mod project.

use std::path::{Path, PathBuf};

/// The bundled-resource subdirectory name (tauri.conf.json mapping target).
pub const RESOURCE_SUBDIR: &str = "selfloc-catalog";
/// The project directory name under the app-data cache root. The session
/// layer derives the project display name from this basename (session.rs
/// `create`), so the string IS the mandate's project name "RimLoc UI (en)".
pub const PROJECT_DIR_NAME: &str = "RimLoc UI (en)";
/// Live-entry safe extras registered after the ORIGINAL registration
/// (src/tests.rs partition arithmetic): the selfloc entry joins pick_directory
/// in the read-only safe class; the contribution builder (wave 7) writes ONLY
/// through the services guard partition (absolute out dir, source-tree /
/// managed-root denies) — the same class as the contract `project_export`.
pub const POST_ORIGINAL_LIVE_EXTRAS: &[&str] =
    &["selfloc_catalog_dir", "selfloc_build_contribution"];

/// The meta field the idempotency decision reads (SELFLOC_BRIDGE.md:
/// `catalog.meta.json.catalog_revision` — a git sha, optional `-dirty`).
#[derive(serde::Deserialize)]
struct CatalogMetaRevision {
    #[serde(default)]
    catalog_revision: Option<String>,
}

fn catalog_revision_of(dir: &Path) -> Option<String> {
    let bytes = std::fs::read(dir.join(rimloc_services::ui_catalog::CATALOG_META_FILE)).ok()?;
    let meta: CatalogMetaRevision = serde_json::from_slice(&bytes).ok()?;
    meta.catalog_revision
}

/// Source candidates in priority order. `resource_dir` is the app's resolved
/// resource directory (`AppHandle::path().resource_dir()`); `None` when the
/// shell could not resolve it (the caller logs the reason and moves on).
/// The dev fallback is a compile-time constant, present only in debug builds.
pub fn catalog_source_candidates(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(res) = resource_dir {
        out.push(res.join(RESOURCE_SUBDIR));
    }
    #[cfg(debug_assertions)]
    out.push(PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../frontend-v2/src/i18n/generated"
    )));
    out
}

/// First candidate that STRICTLY recognizes as a catalog source
/// (`ui_catalog::is_catalog_source`: parseable full-form messages + schema
/// "1" meta). `None` = honestly no usable source in this build.
pub fn find_catalog_source(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|c| rimloc_services::ui_catalog::is_catalog_source(c))
        .cloned()
}

/// `catalog.*.json` payloads of the source directory (sorted by name for a
/// deterministic copy order). A source recognized by `is_catalog_source`
/// always has at least the en file + meta; the ru sidecar rides along so the
/// copied project ships the existing translation too.
fn catalog_payload_files(source: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(source)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("catalog.") && name.ends_with(".json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// The app-data cache root for the copied catalog (sibling of the `RimLoc`
/// logs dir the setup hook already maintains).
pub fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("RimLoc").join(RESOURCE_SUBDIR)
}

/// Idempotent revision-driven sync: ensure `<cache_root>/<PROJECT_DIR_NAME>`
/// holds a catalog source matching `source`'s revision, and return that
/// project directory. Returns the EXISTING dest untouched when the revision
/// matches (the common steady-state: two `stat` reads per entry click).
pub fn sync_catalog_into(source: &Path, cache_root: &Path) -> std::io::Result<PathBuf> {
    let dest = cache_root.join(PROJECT_DIR_NAME);
    let src_rev = catalog_revision_of(source);
    let dst_rev = catalog_revision_of(&dest);
    let up_to_date = dst_rev.is_some() && dst_rev == src_rev && std::fs::metadata(&dest).is_ok();
    if up_to_date {
        return Ok(dest);
    }
    if dest.exists() {
        // App-managed cache, never user content — replacing a stale or
        // broken copy wholesale is the honest repair (a partial copy must
        // not linger as a half-catalog a create could misread).
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;
    for file in catalog_payload_files(source)? {
        let Some(name) = file.file_name() else {
            continue;
        };
        std::fs::copy(&file, dest.join(name))?;
    }
    // Post-condition, same strict gate the caller used for the source: the
    // copy must itself be a catalog source, else the whole call is a loud
    // refusal instead of a directory that would fail create later.
    if !rimloc_services::ui_catalog::is_catalog_source(&dest) {
        return Err(std::io::Error::other(format!(
            "copied catalog at {} failed source recognition",
            dest.display()
        )));
    }
    Ok(dest)
}

/// Full resolution used by the shell command: find the source, sync the
/// app-data copy, return the project-directory path (the `mod_root` the
/// frontend feeds into the EXISTING contract create flow).
pub fn resolve_catalog_dir(
    resource_dir: Option<PathBuf>,
    app_data_dir: &Path,
) -> Result<String, String> {
    let candidates = catalog_source_candidates(resource_dir);
    let Some(source) = find_catalog_source(&candidates) else {
        return Err(format!(
            "the RimLoc UI catalog is not available in this build (looked in: {})",
            candidates
                .iter()
                .map(|c| c.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    };
    let dest = sync_catalog_into(&source, &cache_root(app_data_dir))
        .map_err(|e| format!("failed to prepare the RimLoc UI catalog copy: {e}"))?;
    Ok(dest.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_catalog(dir: &Path, revision: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(
            dir.join("catalog.en.json"),
            r#"{"schema_version":"1","messages":[{"id":"common.appName","source_text":"RimLoc","placeholders":[]}]}"#,
        )
        .unwrap();
        fs::write(
            dir.join("catalog.meta.json"),
            format!(r#"{{"schema_version":"1","catalog_revision":"{revision}"}}"#),
        )
        .unwrap();
        fs::write(
            dir.join("catalog.ru.json"),
            r#"{"schema_version":"1","messages":[{"id":"common.appName","translated":"RimLoc","placeholders":[]}]}"#,
        )
        .unwrap();
    }

    #[test]
    fn dev_candidate_declares_the_repo_generated_dir() {
        let cands = catalog_source_candidates(Some(PathBuf::from("/res")));
        assert_eq!(cands[0], PathBuf::from("/res").join(RESOURCE_SUBDIR));
        // The repo fallback exists ONLY in debug builds (never ships in a
        // release binary) — the candidate count mirrors the cfg gate.
        #[cfg(debug_assertions)]
        {
            assert_eq!(cands.len(), 2);
            let last = cands[1].display().to_string();
            assert!(
                last.ends_with("frontend-v2/src/i18n/generated")
                    || last.ends_with("frontend-v2\\src\\i18n\\generated"),
                "dev candidate must point at the generated dir, got {last}"
            );
        }
        #[cfg(not(debug_assertions))]
        assert_eq!(cands.len(), 1, "release builds must not embed a repo path");
    }

    #[test]
    fn find_source_skips_unparseable_and_takes_first_parseable() {
        let tmp = tempfile::tempdir().unwrap();
        let broken = tmp.path().join("broken");
        let good = tmp.path().join("good");
        fs::create_dir_all(&broken).unwrap();
        write_catalog(&good, "rev1");
        let found = find_catalog_source(&[broken.clone(), good.clone()]);
        assert_eq!(found, Some(good));
        // Nothing parseable → None (never a blind guess).
        assert_eq!(find_catalog_source(&[broken]), None);
    }

    #[test]
    fn sync_copies_full_payload_and_is_idempotent_per_revision() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src");
        let cache = tmp.path().join("cache");
        write_catalog(&source, "rev1");

        let dest = sync_catalog_into(&source, &cache).unwrap();
        assert_eq!(dest.file_name().unwrap(), PROJECT_DIR_NAME);
        for name in ["catalog.en.json", "catalog.meta.json", "catalog.ru.json"] {
            assert!(dest.join(name).is_file(), "{name} must be copied");
        }
        // Recognition post-condition: the copy IS a catalog source.
        assert!(rimloc_services::ui_catalog::is_catalog_source(&dest));

        // Steady state: same revision → no rewrite (mtime-stable), dest returned.
        let en = dest.join("catalog.en.json");
        let before = fs::metadata(&en).unwrap().modified().unwrap();
        let again = sync_catalog_into(&source, &cache).unwrap();
        assert_eq!(again, dest);
        let after = fs::metadata(&en).unwrap().modified().unwrap();
        assert_eq!(before, after, "same revision must not rewrite the copy");

        // Revision move → wholesale replacement with the new payload.
        write_catalog(&source, "rev2");
        let replaced = sync_catalog_into(&source, &cache).unwrap();
        assert_eq!(replaced, dest);
        let meta = fs::read_to_string(dest.join("catalog.meta.json")).unwrap();
        assert!(meta.contains("rev2"), "new revision must land: {meta}");
    }

    #[test]
    fn broken_dest_is_repaired_even_at_the_same_revision() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src");
        let cache = tmp.path().join("cache");
        write_catalog(&source, "rev1");
        sync_catalog_into(&source, &cache).unwrap();

        // A torn/hand-broken copy (meta lost) must be repaired, not trusted:
        // `dst_rev` reads None → the up-to-date shortcut cannot fire.
        fs::remove_file(cache.join(PROJECT_DIR_NAME).join("catalog.meta.json")).unwrap();
        let repaired = sync_catalog_into(&source, &cache).unwrap();
        assert!(rimloc_services::ui_catalog::is_catalog_source(&repaired));
    }
}
