//! `rimloc-mcp` library surface — the stdio binary in `main.rs` is a thin
//! shell over these modules. A lib target exists so integration tests
//! (`tests/`) can drive the real tool layer (tempdir profiles, the
//! testlab fixture mod) exactly the way a served session would.

pub mod error;
pub mod protocol;
pub mod runtime;
pub mod tools;

/// The single stderr logging seam for the whole crate — protocol
/// diagnostics never touch stdout (the protocol channel).
pub fn stderr_log(message: &str) {
    eprintln!("[rimloc-mcp] {message}");
}
