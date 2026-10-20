//! RimLoc GUI-session broker (`docs/development/MCP_SERVER_PHASE2.md`,
//! Phase 2): the ONE authoritative writer is the GUI process, and this
//! crate is its data-plane — a Unix-socket server speaking line-delimited
//! JSON v1 so external `rimloc-mcp` clients execute operations through the
//! GUI's `ProjectSessionManager` instead of racing it.
//!
//! Deliberately window-free: the broker module takes an
//! [`ops::BrokerOps`] seam (the default [`ops::OpsImpl`] drives
//! `rimloc-services` directly; the GUI may host it over its own
//! `ContractState`-style manager) plus a notification callback — the Tauri
//! `emit("broker-changed")` wiring lives in the host, never here. Unit
//! tests run without any window.
//!
//! Hard boundaries (design doc «Границы» + DoD):
//! - the socket is `rimloc-broker.sock` in the PROFILE (the parent of the
//!   managed root), permissions 0600 — never a network listener;
//! - at most 4 concurrent clients, one line at most
//!   [`protocol::MAX_LINE_BYTES`] (8 MiB), per-line handling on its own
//!   thread behind `catch_unwind` so a panicking handler costs one typed
//!   internal error, not the server;
//! - the paid-guard is DUPLICATED here: `ai_job.start` on a non-local
//!   provider instance refuses `paid_providers_disabled` before any launch
//!   (trust the MCP client, but verify);
//! - secrets never cross the wire in either direction (ai_job.start takes
//!   a provider-instance ID only; provider CRUD is not on the surface at
//!   all).
//!
//! The MCP client mode (broker_client in `rimloc-mcp`) is the other half
//! of Phase 2 and lives in its own lane/crate; this crate never depends on
//! `rimloc-mcp`.

pub mod ops;
pub mod protocol;
pub mod server;
