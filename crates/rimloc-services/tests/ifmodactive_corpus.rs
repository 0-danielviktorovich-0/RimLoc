//! IfModActive active-mod-context acceptance against the REAL corpus mod
//! 3170653412 (Medieval Backstories Patch — `LoadFolders.xml` with
//! `IfModActive="Ludeon.RimWorld.Royalty"` branches on v1.4/v1.5/v1.6).
//!
//! Contract under test (RimTransAI parity):
//! - WITH an active-mod context naming Royalty → the conditional branches
//!   resolve (`1.5_1.6/Mods/Royalty`, `1.5_1.6/Patches/Core_Royalty`), their
//!   Defs are scanned and the units carry `conditional: true` with the REAL
//!   source file under the conditional dir;
//! - WITHOUT the context → the same content is NOT scanned (never guessed
//!   into the view), the branches are reported unresolved and the view is
//!   POTENTIAL.
//!
//! Skipped unless `RIMLOC_IFMOD_CORPUS_MOD` points at a local copy of the
//! workshop mod, so CI stays green without network/steamcmd artifacts
//! (same convention as `rimloc-parsers-xml` `RIMLOC_PATCH_CORPUS_MOD`).

use rimloc_services::scan::scan_units_effective_full;
use rimloc_services::{ActiveModContext, ConditionalState};
use std::collections::HashMap;
use std::path::PathBuf;

const ROYALTY: &str = "Ludeon.RimWorld.Royalty";

/// The corpus mod's translatable source lives in BackstoryDef `title`/
/// `description` fields — not in the default defs dictionary, so the probe
/// passes them as extra fields (the same the CLI does via `--defs-field`).
fn extra_fields() -> Vec<String> {
    vec!["title".to_string(), "description".to_string()]
}

fn corpus_root() -> Option<PathBuf> {
    let dir = std::env::var("RIMLOC_IFMOD_CORPUS_MOD").ok()?;
    let root = PathBuf::from(dir);
    assert!(root.is_dir(), "RIMLOC_IFMOD_CORPUS_MOD is not a directory");
    Some(root)
}

fn units_under_conditional(
    scan: &rimloc_services::scan::EffectiveScan,
) -> Vec<&rimloc_core::TransUnit> {
    let view = scan.view.as_ref().unwrap();
    scan.units
        .iter()
        .filter(|u| {
            view.conditional_dirs.iter().any(|d| {
                u.path.starts_with(d) || u.src.as_ref().is_some_and(|s| s.file.starts_with(d))
            })
        })
        .collect()
}

#[test]
fn corpus_ifmodactive_with_royalty_included_without_excluded() {
    let Some(root) = corpus_root() else {
        eprintln!("skip: set RIMLOC_IFMOD_CORPUS_MOD to a copy of workshop mod 3170653412");
        return;
    };

    // A: Royalty active — the conditional branches resolve and their content
    // is scanned, per-unit marked conditional with the REAL source file.
    let ctx = ActiveModContext::from_package_ids(vec![ROYALTY.to_string()]);
    let scan = scan_units_effective_full(&root, None, &HashMap::new(), &extra_fields(), Some(&ctx))
        .expect("scan with Royalty active");
    let view = scan.view.as_ref().unwrap();
    assert_eq!(view.version.as_deref(), Some("1.6"));
    assert_eq!(
        view.conditional_state,
        ConditionalState::Resolved {
            included: 2,
            excluded: 0
        },
        "both Royalty branches of v1.6 must resolve as included"
    );
    assert!(view.unresolved_conditionals.is_empty());
    assert!(
        view.conditional_dirs
            .contains(&root.join("1.5_1.6/Mods/Royalty")),
        "{:?}",
        view.conditional_dirs
    );
    assert!(
        view.conditional_dirs
            .contains(&root.join("1.5_1.6/Patches/Core_Royalty")),
        "{:?}",
        view.conditional_dirs
    );
    // The conditional Defs root joined the effective view.
    assert!(view
        .defs_roots()
        .contains(&root.join("1.5_1.6/Mods/Royalty/Defs")));
    // End to end: units from the conditional dir exist, marked conditional,
    // provenance points at the real file inside the conditional dir.
    let cond_units = units_under_conditional(&scan);
    assert!(
        cond_units.len() >= 10,
        "expected the Royalty backstory defs to surface as conditional units, got {}",
        cond_units.len()
    );
    assert!(cond_units.iter().all(|u| u.conditional));
    assert!(cond_units.iter().all(|u| u
        .src
        .as_ref()
        .is_some_and(|s| s.file.starts_with(root.join("1.5_1.6/Mods/Royalty")))));

    // B: no context — the same content is NOT guessed in: no conditional
    // dirs, none of its units, branches reported unresolved.
    let scan = scan_units_effective_full(&root, None, &HashMap::new(), &extra_fields(), None)
        .expect("scan without context");
    let view = scan.view.as_ref().unwrap();
    assert_eq!(view.version.as_deref(), Some("1.6"));
    assert!(
        view.conditional_dirs.is_empty(),
        "{:?}",
        view.conditional_dirs
    );
    assert_eq!(
        view.conditional_state,
        ConditionalState::Unresolved { entries: 2 }
    );
    assert_eq!(view.unresolved_conditionals.len(), 2);
    assert!(view
        .unresolved_conditionals
        .iter()
        .all(|e| e.packages.iter().any(|p| p.eq_ignore_ascii_case(ROYALTY))));
    assert!(!view
        .defs_roots()
        .contains(&root.join("1.5_1.6/Mods/Royalty/Defs")));
    assert!(
        units_under_conditional(&scan).is_empty(),
        "without a context no conditional unit may be scanned"
    );
    assert!(
        !scan.units.iter().any(|u| u.conditional),
        "without a context no unit may claim conditional inclusion"
    );
    // The A/B is a real difference, not two empty scans.
    assert!(!scan.units.is_empty());
}

#[test]
fn corpus_ifmodactive_context_is_case_insensitive_and_dlc_shaped() {
    let Some(root) = corpus_root() else {
        eprintln!("skip: set RIMLOC_IFMOD_CORPUS_MOD to a copy of workshop mod 3170653412");
        return;
    };
    // The game normalizes packageIds to lowercase; the DLC list is a
    // separate input in the real payload. Both spellings must resolve.
    let ctx = ActiveModContext {
        active_package_ids: Vec::new(),
        active_dlc: vec!["ludeon.rimworld.royalty".to_string()],
    };
    let scan = scan_units_effective_full(&root, None, &HashMap::new(), &extra_fields(), Some(&ctx))
        .expect("scan with lowercase DLC Royalty");
    assert_eq!(
        scan.view.as_ref().unwrap().conditional_state,
        ConditionalState::Resolved {
            included: 2,
            excluded: 0
        }
    );
    assert!(!units_under_conditional(&scan).is_empty());
}
