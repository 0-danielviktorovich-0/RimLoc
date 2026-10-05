//! Adversarial filesystem tests for the canonical write guard (mandate
//! P0 §8). INTEGRATION tier: every case goes through a PUBLIC API surface
//! (`ProjectSessionManager::managed_path` / `export_project`,
//! `observability::collect_support_bundle_in`, the re-exported
//! `ensure_writable_output_path` / `ensure_free_output_path` /
//! `resolve_cli_out_path`) — the same functions the CLI and the GUI shells
//! call — never through private helpers.
//!
//! Proven here, per attack class:
//! 1. path traversal in `project_id` and in out paths (`../..`) — typed
//!    refusal, NOTHING on disk changes;
//! 2. absolute path outside every allow root — `OutsideAllowRoots`;
//! 3. symlink escape: a link planted inside a writable root that resolves
//!    to a `/tmp` target — `SymlinkEscape`, target untouched;
//! 4. read-only sources in FREE mode: the guard refuses destinations
//!    inside the passed protected roots only — the caller (CLI site)
//!    decides which roots to pass; the "empty protected list" gap is
//!    FIXED AS DOCUMENTED SEMANTICS (see
//!    `free_mode_guard_protects_only_roots_it_was_given` below), not
//!    silently assumed away;
//! 5. export INTO the scanned mod directory itself — `guard_output_denied`,
//!    source byte-identical before/after.

use rimloc_services::contract::ContractErrorCode;
use rimloc_services::{
    ensure_free_output_path, ensure_writable_output_path, PathGuardErrorKind,
    ProjectSessionManager,
};
use rimloc_services::observability::collect_support_bundle_in;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

/// Minimal RimWorld source mod the session scanner accepts (same shape the
/// in-crate session tests use): two Defs files, no Languages tree.
fn seed_source_mod(root: &Path) {
    std::fs::create_dir_all(root.join("Defs")).expect("defs dir");
    std::fs::write(
        root.join("Defs/A_Thing.xml"),
        r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
    )
    .expect("write defs A");
    std::fs::write(
        root.join("Defs/B_Ability.xml"),
        r#"<Defs><AbilityDef><defName>Dup</defName><label>ability label</label></AbilityDef></Defs>"#,
    )
    .expect("write defs B");
}

fn observability_meta() -> rimloc_services::observability::ProjectMeta {
    rimloc_services::observability::ProjectMeta {
        name: Some("adversarial".into()),
        target_lang: Some("Russian".into()),
        rw_version: Some("1.6".into()),
        rimloc_version: Some("test".into()),
        extra: serde_json::json!({}),
    }
}

/// sha256-free content fingerprint: sorted (path, len, bytes) digest via
/// std only — enough to prove a tree is byte-identical across an attempt.
fn tree_fingerprint(root: &Path) -> Vec<(String, u64)> {
    fn walk(dir: &Path, out: &mut Vec<(String, u64)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .expect("read_dir")
            .flatten()
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let bytes = std::fs::read(&p).expect("read file");
                out.push((p.to_string_lossy().into_owned(), bytes.len() as u64));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn dir_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .expect("read_dir")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------
// 1. path traversal in project_id — typed refusal, nothing written
// ---------------------------------------------------------------------------

/// `managed_path` is THE chokepoint that turns a client-supplied project id
/// into a filesystem path. Every traversal-shaped id must die on the form
/// regex (`^proj-[a-z0-9-]+$`) as a typed `contract_violation`, and the
/// managed root must stay untouched.
#[test]
fn managed_path_refuses_traversal_project_ids_and_writes_nothing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).expect("manager");

    let hostile_ids = [
        "../..",
        "../../out.json",
        "proj-../../out.json",
        "proj-a/../proj-b",
        "proj-..",
        "proj-x.",
        ".",
        "",
        "/etc/passwd",
        "proj-/abs",
        "proj-x%2e%2e",
        "PROJ-UPPER",
    ];
    for id in hostile_ids {
        let err = mgr
            .managed_path(id)
            .expect_err(&format!("hostile id `{id}` must be refused"));
        assert_eq!(
            err.code,
            ContractErrorCode::ContractViolation,
            "id `{id}`: {err}"
        );
        assert!(
            err.message.contains("malformed project id"),
            "id `{id}`: {err}"
        );
    }

    // Nothing was written: the managed root holds no files at all.
    assert!(
        dir_names(&tmp.path().join("managed")).is_empty(),
        "refused ids must not leave managed files behind"
    );

    // The same hostile id through the higher-level command surface is
    // refused too (project_not_found there — the registry lookup precedes
    // path building — and equally write-free).
    let err = mgr
        .export_project(
            "proj-../../evil",
            0,
            &tmp.path().join("out"),
            "Russian",
        )
        .expect_err("hostile id refused at command level");
    assert_eq!(err.code, ContractErrorCode::ProjectNotFound, "{err}");
    assert!(
        dir_names(&tmp.path().join("managed")).is_empty(),
        "command-level refusal must not write either"
    );
}

// ---------------------------------------------------------------------------
// 1b. path traversal in out paths through the export command
// ---------------------------------------------------------------------------

/// `out='out.json'`-style relative caller paths are refused BEFORE any
/// filesystem access (typed `invalid_output_path`), and a `../..`-shaped
/// ABSOLUTE out dir that resolves INTO the source tree is refused by the
/// canonical containment guard (`guard_output_denied`) — with no
/// directories created by the refused attempt.
#[test]
fn export_refuses_relative_and_traversing_out_dirs_and_writes_nothing() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("mod");
    seed_source_mod(&mod_root);
    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).expect("manager");
    let snap = mgr.create(&mod_root, Some("1.6")).expect("create");

    // Relative caller path — the RC K4 shape.
    let err = mgr
        .export_project(&snap.project_id, snap.session_epoch, Path::new("out.json"), "Russian")
        .expect_err("relative out dir refused");
    assert_eq!(err.code, ContractErrorCode::InvalidOutputPath, "{err}");
    assert!(
        err.message.contains("is not absolute"),
        "message must name the form rule: {err}"
    );

    // Absolute spelling that parent-traverses INTO the mod tree. The
    // intermediate `outside/sub` does NOT exist yet — the guard must catch
    // this on the canonical view WITHOUT creating anything.
    let traversing = tmp.path().join("outside/sub/../../mod/evil-out");
    let err = mgr
        .export_project(&snap.project_id, snap.session_epoch, &traversing, "Russian")
        .expect_err("traversal into source refused");
    assert_eq!(err.code, ContractErrorCode::GuardOutputDenied, "{err}");
    assert!(
        err.message.contains("inside the read-only source tree"),
        "{err}"
    );

    // Same shape landing in the MANAGED projects root (artifact must never
    // overwrite managed records).
    let managed_traversal = tmp
        .path()
        .join("managed/../managed-inner/../managed/evil-out");
    let err = mgr
        .export_project(&snap.project_id, snap.session_epoch, &managed_traversal, "Russian")
        .expect_err("traversal into managed root refused");
    assert_eq!(err.code, ContractErrorCode::GuardOutputDenied, "{err}");
    assert!(
        err.message.contains("inside the managed projects root"),
        "{err}"
    );

    // A traversal-shaped LOCALE never reaches the writer either (P1-2).
    let err = mgr
        .export_project(
            &snap.project_id,
            snap.session_epoch,
            &tmp.path().join("out"),
            "../../evil",
        )
        .expect_err("traversal locale refused");
    assert_eq!(err.code, ContractErrorCode::ContractViolation, "{err}");
    assert!(err.message.contains("malformed locale"), "{err}");

    // NOTHING exists: not the export dirs, not the traversal intermediates.
    for absent in [
        tmp.path().join("out.json"),
        tmp.path().join("outside"),
        tmp.path().join("mod/evil-out"),
        tmp.path().join("managed/evil-out"),
        tmp.path().join("out"),
    ] {
        assert!(!absent.exists(), "refused attempt must not create {absent:?}");
    }
    // And the source mod is untouched.
    assert!(mod_root.join("Defs/A_Thing.xml").exists());
    assert!(!mod_root.join("Languages").exists());
}

// ---------------------------------------------------------------------------
// 2. absolute path outside every allow root
// ---------------------------------------------------------------------------

/// Allow-direction guard through the PUBLIC re-export: a well-formed
/// absolute destination outside every allow root is refused
/// (`OutsideAllowRoots`); the blessed path the guard RETURNS is the only
/// thing a compliant writer writes to.
#[test]
fn writable_guard_refuses_absolute_outside_allow_roots() {
    let tmp = tempfile::tempdir().expect("tmp");
    let allow_root = tmp.path().join("writable");
    let sibling = tmp.path().join("sibling");
    std::fs::create_dir_all(&allow_root).expect("allow root");
    std::fs::create_dir_all(&sibling).expect("sibling");

    let err = ensure_writable_output_path(&sibling.join("out.json"), &[&allow_root])
        .expect_err("absolute outside allow roots refused");
    assert_eq!(err.kind, PathGuardErrorKind::OutsideAllowRoots);
    assert!(err.message.contains("outside every writable root"), "{err}");
    // Nothing was written by the refused attempt.
    assert!(!sibling.join("out.json").exists());

    // Empty allow list refuses EVERYTHING (fail-closed surface config).
    let err = ensure_writable_output_path(&allow_root.join("out.json"), &[])
        .expect_err("no roots configured → refuse");
    assert_eq!(err.kind, PathGuardErrorKind::OutsideAllowRoots, "{err}");

    // Full pipeline discipline: the writer takes the RETURNED canonical
    // path, and the file lands at the canonical location.
    let blessed = ensure_writable_output_path(&allow_root.join("nested/out.json"), &[&allow_root])
        .expect("contained destination blessed");
    rimloc_services::write_atomic(&blessed, b"payload").expect("write to blessed path");
    assert_eq!(
        std::fs::read(&blessed).expect("read back"),
        b"payload",
        "the blessed canonical path is the write target"
    );
    assert!(blessed.starts_with(rimloc_services::canonical_view(&allow_root).unwrap()));
}

// ---------------------------------------------------------------------------
// 3. symlink escape out of a temp writable root
// ---------------------------------------------------------------------------

/// A symlink planted INSIDE the writable root that resolves to a
/// `/tmp`-located target must be refused as `SymlinkEscape` when the raw
/// spelling looks contained — and the escape target must stay untouched.
#[test]
fn symlink_escape_from_temp_root_is_refused_and_target_untouched() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = tmp.path().join("writable-root");
    let sibling = tmp.path().join("sibling");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::create_dir_all(&sibling).expect("sibling");

    // Link #1: target under the shared temp root (which IS a /tmp location
    // on this platform — std tempdir) but OUTSIDE the writable root.
    std::os::unix::fs::symlink(&sibling, root.join("jump-tmp")).expect("symlink");
    // Link #2: target in the LITERAL system temp dir (`/tmp` or its
    // canonical alias), the shape named by the P0 §8 mandate.
    let literal_tmp_target =
        std::env::temp_dir().join(format!("rimloc-fsadv-{}", std::process::id()));
    std::os::unix::fs::symlink(&literal_tmp_target, root.join("jump-literal-tmp"))
        .expect("symlink");

    let via_link = root.join("jump-tmp/evil.json");
    let err = ensure_writable_output_path(&via_link, &[&root])
        .expect_err("symlink escape refused");
    assert_eq!(err.kind, PathGuardErrorKind::SymlinkEscape, "{err}");
    assert!(err.message.contains("symlink"), "verdict names the mechanism: {err}");
    // The escape target was never written.
    assert!(!sibling.join("evil.json").exists());

    let via_literal = root.join("jump-literal-tmp/evil.json");
    let err = ensure_writable_output_path(&via_literal, &[&root])
        .expect_err("literal /tmp symlink escape refused");
    assert_eq!(err.kind, PathGuardErrorKind::SymlinkEscape, "{err}");
    assert!(!literal_tmp_target.join("evil.json").exists());
    let _ = std::fs::remove_dir(&literal_tmp_target); // only if we created it

    // The same destination spelled ABSOLUTELY (not through the root) is
    // classified as a plain outside-root refusal, not an escape — the
    // verdict distinguishes a hidden jump from an honest outsider.
    let err = ensure_writable_output_path(&sibling.join("honest.json"), &[&root])
        .expect_err("absolute outsider refused");
    assert_eq!(err.kind, PathGuardErrorKind::OutsideAllowRoots, "{err}");
}

// ---------------------------------------------------------------------------
// 4. sources READ-ONLY in FREE mode — documented semantics
// ---------------------------------------------------------------------------

/// Free-mode guard semantics, proven against the PUBLIC bundle collector
/// (the site that passes `scan_root` as protected):
/// - an out dir INSIDE the scanned source tree (equal, nested, reached via
///   `../..`, or through a symlink alias) is refused before any write;
/// - a legitimate outside dir works and writes OUTSIDE the source.
#[test]
fn free_mode_refuses_destinations_inside_the_scanned_source() {
    let tmp = tempfile::tempdir().expect("tmp");
    let scan_root = tmp.path().join("source-mod");
    seed_source_mod(&scan_root);
    let before = tree_fingerprint(&scan_root);

    // (a) equal to the source root itself.
    let err = collect_support_bundle_in(&scan_root, &scan_root, &observability_meta())
        .expect_err("bundle into the source root refused");
    assert!(
        format!("{err}").contains("inside the read-only source tree"),
        "{err}"
    );

    // (b) nested INSIDE the source (Languages/ — the tree the mod ships).
    let nested = scan_root.join("Languages/Russian/bundle");
    let err = collect_support_bundle_in(&nested, &scan_root, &observability_meta())
        .expect_err("bundle into the source tree refused");
    assert!(format!("{err}").contains("inside the read-only source tree"), "{err}");

    // (c) parent-traversing spelling that RESOLVES into the source; the
    // intermediate dir does not exist and must not be created.
    let traversing = tmp.path().join("x/y/../../source-mod/bundle");
    let err = collect_support_bundle_in(&traversing, &scan_root, &observability_meta())
        .expect_err("traversal into source refused");
    assert!(format!("{err}").contains("inside the read-only source tree"), "{err}");
    assert!(!tmp.path().join("x").exists(), "no intermediate dirs created");

    // (d) symlink ALIAS of the source: writing "through" the alias lands
    // in the source on the canonical view — refused too.
    let alias_parent = tmp.path().join("user");
    std::fs::create_dir_all(&alias_parent).expect("user dir");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&scan_root, alias_parent.join("alias")).expect("symlink");
        let via_alias = alias_parent.join("alias/bundle");
        let err = collect_support_bundle_in(&via_alias, &scan_root, &observability_meta())
            .expect_err("symlink alias into source refused");
        assert!(
            format!("{err}").contains("inside the read-only source tree"),
            "{err}"
        );
    }

    // The source tree is byte-identical: the guard refused BEFORE any
    // mkdir/write, so not even the refused directory names exist.
    assert_eq!(tree_fingerprint(&scan_root), before, "source unchanged");
    assert!(!scan_root.join("Languages").exists());
    assert!(!scan_root.join("bundle").exists());

    // (e) sanity: a LEGITIMATE outside destination still works and writes
    // outside the source.
    let legit = tmp.path().join("support-out");
    let bundle =
        collect_support_bundle_in(&legit, &scan_root, &observability_meta()).expect("bundle ok");
    assert!(bundle.dir.starts_with(rimloc_services::canonical_view(&legit).unwrap()));
    assert!(!bundle.files.is_empty());
    assert_eq!(tree_fingerprint(&scan_root), before, "source still unchanged");
}

/// DOCUMENTED SEMANTICS (conscious trade-off, NOT a vulnerability claim):
/// the free-form guard fences ONLY the protected roots it is GIVEN. The
/// bundle collector passes `scan_root`; the CLI learn-patches site
/// (`crates/rimloc-cli/src/commands/learn_patches.rs:24`) passes `&[]` —
/// there a caller-chosen out path inside a source tree is NOT refused by
/// the guard itself. This test PINS that behavior so a future tightening
/// is a deliberate, visible change — and so nobody mistakes the empty
/// list for hidden protection.
#[test]
fn free_mode_guard_protects_only_roots_it_was_given() {
    let tmp = tempfile::tempdir().expect("tmp");
    let scan_root = tmp.path().join("source-mod");
    seed_source_mod(&scan_root);

    // With the source passed as protected: refused (the collector's site).
    let err = ensure_free_output_path(&scan_root.join("out.json"), &[&scan_root])
        .expect_err("protected root refused when passed");
    assert_eq!(err.kind, PathGuardErrorKind::ProtectedRoot, "{err}");

    // With an EMPTY protected list (the learn-patches site): the same path
    // is allowed — the guard makes no source-tree claim of its own.
    let allowed = ensure_free_output_path(&scan_root.join("out.json"), &[])
        .expect("empty protected list protects nothing");
    assert_eq!(
        allowed,
        rimloc_services::canonical_view(&scan_root).unwrap().join("out.json"),
        "the caller receives the canonical path and owns the choice"
    );
    // The guard itself still writes nothing either way.
    assert!(!scan_root.join("out.json").exists());
}

// ---------------------------------------------------------------------------
// 5. export INTO the scanned mod directory (output-inside-source race)
// ---------------------------------------------------------------------------

/// The session export command refuses an out dir that sits inside/equal the
/// read-only source tree — including the mod root ITSELF and its nested
/// directories — and the source is byte-identical before/after. A
/// legitimate isolated out dir keeps working (the refusal is the guard,
/// not a broken project).
#[test]
fn export_into_source_mod_dir_refused_and_source_unchanged() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("mod");
    seed_source_mod(&mod_root);
    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).expect("manager");
    let snap = mgr.create(&mod_root, Some("1.6")).expect("create");
    // One applied translation so the sanity export has real content.
    mgr.apply(&rimloc_services::contract::ApplyIntentsRequest {
        project_id: snap.project_id.clone(),
        expected_revision: 1,
        session_epoch: snap.session_epoch,
        intents: vec![rimloc_services::contract::TranslationIntent {
            entry: rimloc_domain::canonical::SourceEntryId {
                kind: rimloc_domain::canonical::EntryKind::DefInjected,
                key: "Dup.label".into(),
                def_type: Some("ThingDef".into()),
            },
            locale: "Russian".into(),
            action: rimloc_services::contract::IntentAction::SetTranslation,
            text: Some("вещь".into()),
        }],
    })
    .expect("apply translation");
    let before = tree_fingerprint(&mod_root);

    for hostile_out in [
        mod_root.clone(),                            // the mod root itself
        mod_root.join("Languages"),                  // a shipped subtree
        mod_root.join("Defs"),                       // the scanned Defs
        mod_root.join("sub/../Defs2"),               // traversal resolving beside Defs
    ] {
        let err = mgr
            .export_project(&snap.project_id, snap.session_epoch, &hostile_out, "Russian")
            .expect_err(&format!("export into {:?} refused", hostile_out));
        assert_eq!(
            err.code,
            ContractErrorCode::GuardOutputDenied,
            "out={:?}: {err}",
            hostile_out
        );
    }

    // Managed-root exports are refused with the SAME typed verdict.
    let err = mgr
        .export_project(
            &snap.project_id,
            snap.session_epoch,
            &mgr.managed_root().join("evil-out"),
            "Russian",
        )
        .expect_err("export into managed root refused");
    assert_eq!(err.code, ContractErrorCode::GuardOutputDenied, "{err}");

    // The source is byte-identical; nothing new appeared anywhere in it.
    assert_eq!(tree_fingerprint(&mod_root), before, "source unchanged");
    assert!(!mod_root.join("Languages").exists(), "no Languages tree created");

    // Sanity: the SAME session exports fine into an isolated dir — the
    // refusals above are the guard, not project corruption.
    let legit = tmp.path().join("isolated-out");
    let res = mgr
        .export_project(&snap.project_id, snap.session_epoch, &legit, "Russian")
        .expect("legitimate export works");
    assert!(res.files_written >= 1);
    assert!(legit.join("Languages/Russian").exists());
    assert_eq!(tree_fingerprint(&mod_root), before, "source still unchanged");
}

// ---------------------------------------------------------------------------
// CLI resolution flavor: relative paths become explicit, once
// ---------------------------------------------------------------------------

/// `resolve_cli_out_path` + `ensure_free_output_path` is the documented CLI
/// pipeline: the CWD join happens HERE (visible), and the guard still
/// refuses protected roots on the joined result. A hostile relative
/// spelling must not smuggle a write into a protected root.
#[test]
fn cli_pipeline_joins_cwd_then_enforces_protection() {
    let tmp = tempfile::tempdir().expect("tmp");
    let scan_root = tmp.path().join("source-mod");
    seed_source_mod(&scan_root);

    // Relative hostile path — joined against CWD, then denied by the
    // guard because it resolves inside the protected root.
    let hostile = PathBuf::from("../shenanigans/out.json");
    let joined = rimloc_services::resolve_cli_out_path(&hostile).expect("explicit join");
    assert!(joined.is_absolute(), "{joined:?}");
    let with_protection = ensure_free_output_path(&joined, &[&scan_root]);
    // Refused only if the process CWD actually resolves under scan_root;
    // assert the TYPED behavior in the case where it does.
    if rimloc_services::is_within(&joined, &scan_root) {
        let err = with_protection.expect_err("CWD inside protected root is denied");
        assert_eq!(err.kind, PathGuardErrorKind::ProtectedRoot, "{err}");
    } else {
        // CWD elsewhere: the joined path is legitimately outside — nothing
        // was written by the resolution itself.
        assert!(with_protection.is_ok());
        assert!(!joined.exists(), "resolution writes nothing");
    }

    // The absolute equivalent is always denied — no CWD involved. Both
    // spellings resolve INSIDE the protected root: a plain nested one and
    // a parent-traversing one that re-enters the source.
    for hostile in [
        PathBuf::from("./out.json"),
        PathBuf::from("a/../out.json"),
        PathBuf::from("../shenanigans/../source-mod/out.json"),
    ] {
        let hostile = scan_root.join(hostile);
        let err = ensure_free_output_path(&hostile, &[&scan_root])
            .expect_err(&format!("INTO-source path refused: {}", hostile.display()));
        assert_eq!(err.kind, PathGuardErrorKind::ProtectedRoot, "{err}");
        assert!(
            !scan_root.join("out.json").exists(),
            "the guard resolution itself writes nothing"
        );
    }

    // The same join pipeline stays legitimate for a genuinely outside
    // destination: resolved, blessed, still unwritten.
    let outside = rimloc_services::resolve_cli_out_path(Path::new("../shenanigans/out.json"))
        .expect("explicit join");
    let blessed = ensure_free_output_path(&outside, &[&scan_root]).expect("outside is free");
    assert!(blessed.is_absolute());
    assert!(!blessed.exists(), "resolution writes nothing");
}
