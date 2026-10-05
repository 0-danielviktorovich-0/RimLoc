//! Cross-PROCESS restart proof for the OS keychain (feature `keychain`).
//!
//! The default test suite runs both phases as no-ops (env guard) so a plain
//! `cargo test` stays deterministic and never touches the keychain. The
//! proof itself is executed as TWO SEPARATE cargo-test PROCESSES against
//! the TEST namespace only (`rimloc-llm-selftest`):
//!
//! ```text
//! RIMLOC_KEYCHAIN_LANE_PHASE=1 cargo test -p rimloc-llm --features keychain \
//!     --test keychain_restart lane_phase1_write
//! RIMLOC_KEYCHAIN_LANE_PHASE=2 cargo test -p rimloc-llm --features keychain \
//!     --test keychain_restart lane_phase2_read
//! ```
//!
//! Phase 2 runs in a FRESH process: reading the exact value back proves the
//! secret persisted in the macOS Keychain across a process restart — the
//! property the provider-secret persistence contract rests on.
#![cfg(feature = "keychain")]

use keyring::Entry;

const SERVICE: &str = "rimloc-llm-selftest";
const ACCOUNT: &str = "lane-restart-proof";
const SECRET: &str = "sk-lane-restart-proof-2026-10-05";

fn entry() -> Entry {
    Entry::new(SERVICE, ACCOUNT).expect("keychain entry")
}

#[test]
fn lane_phase1_write() {
    if std::env::var("RIMLOC_KEYCHAIN_LANE_PHASE").as_deref() != Ok("1") {
        eprintln!("skipped: run with RIMLOC_KEYCHAIN_LANE_PHASE=1");
        return;
    }
    let e = entry();
    e.set_password(SECRET).expect("keychain write");
    // Immediate readback on the SAME process (sanity, not the proof).
    assert_eq!(
        e.get_password().expect("keychain readback").as_str(),
        SECRET
    );
    // The item is LEFT in the keychain for phase 2.
}

#[test]
fn lane_phase2_read_and_cleanup() {
    if std::env::var("RIMLOC_KEYCHAIN_LANE_PHASE").as_deref() != Ok("2") {
        eprintln!("skipped: run with RIMLOC_KEYCHAIN_LANE_PHASE=2");
        return;
    }
    // FRESH PROCESS: a brand-new Entry must see what the previous process
    // wrote — the OS keychain persisted it across the process boundary.
    let e = entry();
    assert_eq!(e.get_password().expect("keychain read").as_str(), SECRET);
    // Cleanup: the proof is complete, the test namespace is left empty.
    e.delete_credential().expect("keychain delete");
    assert!(
        matches!(e.get_password(), Err(keyring::Error::NoEntry)),
        "the secret must be gone after cleanup"
    );
}
