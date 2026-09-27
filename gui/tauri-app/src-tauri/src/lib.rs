//! Shared surface of the RimLoc GUI shell.
//!
//! The binary (main.rs) owns the legacy command implementations; this lib
//! exposes ONLY the binding-wave contract adapter so enforcement tests can
//! build the real app against a mock runtime (lead review 029 / binding
//! wave 2: "built-app evidence beats a source regex").
pub mod contract_adapter;
/// Self-localization entry (mandate D): the app-bundled UI catalog exposed
/// as an ordinary project source. Pure logic lives here; the thin
/// `#[tauri::command]` wrapper lives in the binary next to pick_directory.
pub mod selfloc_catalog;
