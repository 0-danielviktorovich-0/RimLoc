//! Shared surface of the RimLoc GUI shell.
//!
//! The binary (main.rs) owns the legacy command implementations; this lib
//! exposes ONLY the binding-wave contract adapter so enforcement tests can
//! build the real app against a mock runtime (lead review 029 / binding
//! wave 2: "built-app evidence beats a source regex").
pub mod contract_adapter;
<<<<<<< HEAD
/// Provider connectivity probe (§7 F7.1/F7.2): the bounded
/// `contract_provider_instance_test` command — the CLI `provider-test`
/// logic reused by the GUI. Pure logic + the command live here; the
/// registration joins the contract command lists.
pub mod provider_test;
=======
/// Session reveal guard (§8 F8.4): the allow-list of successful build/export
/// out dirs behind the shell-level `reveal_path` command. Pure logic lives
/// here (the contract adapter blesses acked out dirs); the thin
/// `#[tauri::command]` wrapper lives in the binary next to pick_directory.
pub mod reveal;
>>>>>>> feat/r4-buildexport
/// Self-localization entry (mandate D): the app-bundled UI catalog exposed
/// as an ordinary project source. Pure logic lives here; the thin
/// `#[tauri::command]` wrapper lives in the binary next to pick_directory.
pub mod selfloc_catalog;
/// Agent action-trace (RIMLOC_TRACE=1): JSONL command trace for the
/// agent-driven QA journeys — see `trace` docs.
pub mod trace;
