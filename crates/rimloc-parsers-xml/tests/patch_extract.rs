//! Golden + corpus tests for PatchOperations value extraction
//! (MUST_FIX_BEFORE_BETA #1: competitors extract 54 player-visible strings
//! from mod 3170653412, RimLoc extracted 0).

use rimloc_parsers_xml::{is_patch_noise_field, scan_patch_values, PatchValueCandidate};
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/PatchOpsMod")
}

fn sig(c: &PatchValueCandidate) -> String {
    format!("{}={}", c.field, c.value)
}

#[test]
fn golden_patch_values() {
    let (cands, stats) = scan_patch_values(&fixture_root(), 1).expect("scan fixture");
    eprintln!("fixture stats: {stats:?}");

    let mut got: Vec<String> = cands.iter().map(sig).collect();
    got.sort();
    let mut expected = vec![
        "description=Golden one description text.".to_string(),
        "title=golden king".to_string(),
        "label=golden rod".to_string(),
        "description=Apparel description.".to_string(),
        "description=Plain text description replacement.".to_string(),
        "rulesStrings=rule one".to_string(),
        "rulesStrings=rule two".to_string(),
    ];
    expected.sort();
    assert_eq!(got, expected, "golden extraction mismatch");

    // Noise never leaks: every blacklist hit was filtered, none emitted.
    for c in &cands {
        assert!(
            !is_patch_noise_field(&c.field),
            "noise leaked: {} ({})",
            c.field,
            c.value
        );
    }
    assert_eq!(stats.emitted, 7);
    assert_eq!(
        stats.filtered_noise, 4,
        "spawnCategories x2 + bodyTypeMale + requiredWorkTags.li"
    );
    assert_eq!(
        stats.filtered_non_string, 0,
        "non-string guard must not fire: 1.5/true are stopped by the whitelist"
    );
    assert_eq!(
        stats.filtered_not_whitelisted, 2,
        "commonality + shuffleable"
    );
}

/// Corpus check against a local copy of workshop mod 3170653412
/// (RimThrone Divine Order — 25 patch files, competitor benchmark: 54
/// player-visible strings, RimLoc before the fix: 0).
/// Skipped when RIMLOC_PATCH_CORPUS_MOD is not set, so CI stays green
/// without the corpus.
#[test]
fn corpus_3170653412_player_visible() {
    let Ok(dir) = std::env::var("RIMLOC_PATCH_CORPUS_MOD") else {
        eprintln!(
            "skip: set RIMLOC_PATCH_CORPUS_MOD to a copy of workshop mod 3170653412 to run the corpus check"
        );
        return;
    };
    let root = PathBuf::from(dir);
    assert!(root.is_dir(), "RIMLOC_PATCH_CORPUS_MOD is not a directory");
    let versioned = root.join("1.5_1.6");
    let scan_root = if versioned.is_dir() { versioned } else { root };
    let (cands, stats) = scan_patch_values(&scan_root, 1).expect("scan corpus");
    eprintln!("corpus 3170653412 stats: {stats:?}");

    assert!(
        cands.len() >= 54,
        "expected >=54 player-visible patch strings, got {}",
        cands.len()
    );
    for c in &cands {
        assert!(
            !is_patch_noise_field(&c.field),
            "noise leaked into corpus output: {} ({})",
            c.field,
            c.value
        );
        assert!(!c.value.trim().is_empty());
    }
}
