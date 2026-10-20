//! `rimloc-mcp` — RimLoc external MCP server, Phase 1
//! (`docs/development/MCP_SERVER_PHASE1.md`).
//!
//! Two exclusive modes:
//! - no argument: serve MCP 2026-07-28 over stdio — line-delimited
//!   JSON-RPC 2.0 in on stdin, one response line per request on stdout.
//!   The process is started ONLY by an external MCP client; installing
//!   RimLoc starts no service. stdout carries protocol bytes ONLY; every
//!   diagnostic goes to stderr. Startup raises the headless runtime: the
//!   `RIMLOC_DATA_DIR` managed-root profile + the exclusive writer lock
//!   (`rimloc_mcp::runtime`) — `rimloc.status` can therefore answer the
//!   mode question honestly from the very first call. Degradation is
//!   never a startup failure: a busy lock becomes `other_writer` mode,
//!   and the default/refused profile becomes `shared_profile` mode —
//!   both honest read-only diagnostics, fail-closed (writes and
//!   project-scoped reads refuse typed; disk diagnostics stay up).
//! - `rimloc-mcp doctor`: an environment report printed to stdout (this is
//!   a CLI subcommand, not a protocol session — the two modes never mix,
//!   and doctor stays side-effect free: no lock, no directories).
//!
//! The full v1 surface (nine read-only tools + the controlled-write layer)
//! is live; every write carries the session_epoch / expected_revision
//! guards of the services contract.

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::sync::Arc;

use rimloc_mcp::{protocol, runtime, stderr_log, tools};

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None => serve(),
        Some("doctor") => doctor(),
        Some(other) => {
            eprintln!(
                "[rimloc-mcp] unknown subcommand `{other}`; \
                 usage: rimloc-mcp [doctor]  (no argument = serve stdio MCP)"
            );
            ExitCode::from(2)
        }
    }
}

fn serve() -> ExitCode {
    // Headless runtime: profile resolution + exclusive writer lock. A busy
    // lock degrades to other_writer mode (honest read-only diagnostics),
    // the default/refused profile degrades to shared_profile mode
    // (fail-closed diagnostics, audit п.2/п.4), a real I/O failure aborts
    // before the first protocol byte.
    let rt = match runtime::Runtime::from_env() {
        Ok(rt) => Arc::new(rt),
        Err(err) => {
            stderr_log(&format!(
                "managed-root runtime failed to start ({err}); not serving"
            ));
            return ExitCode::FAILURE;
        }
    };
    match rt.mode() {
        runtime::AppMode::HeadlessWriter => stderr_log(&format!(
            "headless writer: exclusive lock held on `{}`; protocol {} over stdio \
             (stateless per-request _meta; logs on stderr only)",
            rt.managed_root().display(),
            protocol::PROTOCOL_VERSION
        )),
        runtime::AppMode::OtherWriter { owner } => stderr_log(&format!(
            "other_writer mode: the lock on `{}` is held elsewhere (pid {:?}); \
             write tools and project-scoped reads refuse, disk diagnostics stay up",
            rt.managed_root().display(),
            owner.pid
        )),
        runtime::AppMode::SharedProfile { reason } => {
            let cause = match reason {
                runtime::SharedProfileReason::DefaultProfile => {
                    "RIMLOC_DATA_DIR is not set: the default profile is shared with the \
                     live RimLoc GUI, which never takes rimloc-mcp.lock"
                }
                runtime::SharedProfileReason::Refused { requested, cause } => {
                    &format!("RIMLOC_DATA_DIR `{requested}` was refused: {cause}")
                }
            };
            stderr_log(&format!(
                "shared_profile mode (fail-closed): {cause}. Write tools and \
                 project-scoped reads refuse typed `profile_shared`; disk \
                 diagnostics (`rimloc.status`, `rimloc.capabilities`, \
                 `rimloc.projects.list`, `rimloc.jobs.status`) stay up on `{}`. \
                 Set RIMLOC_DATA_DIR to isolate.",
                rt.managed_root().display()
            ));
        }
    }
    let server = protocol::Server::with_runtime(rt);
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut out = std::io::stdout().lock();
    // A request line is JSON-RPC over stdio from a local client; protocol
    // frames here are kilobytes. Anything past this bound is a broken or
    // hostile writer — answer typed and keep serving instead of letting a
    // runaway line grow the process without limit.
    const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

    loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => break, // EOF: the client closed the pipe
            Ok(_) => {}
            Err(err) => {
                stderr_log(&format!("stdin read failed: {err}; stopping"));
                return ExitCode::FAILURE;
            }
        }
        if line.len() > MAX_LINE_BYTES {
            let response = protocol::line_too_long_error(MAX_LINE_BYTES);
            if let Err(err) = out
                .write_all(response.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .and_then(|()| out.flush())
            {
                stderr_log(&format!("stdout write failed: {err}; stopping"));
                return ExitCode::FAILURE;
            }
            continue;
        }
        // A panic inside a handler (e.g. a poisoned lock in service code
        // this crate does not control) must degrade to ONE typed error
        // response, never take the whole stdio session down with it.
        let handled =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| server.handle_line(&line)));
        let response = match handled {
            Ok(Some(response)) => response,
            Ok(None) => continue, // notification / empty line — nothing on stdout
            Err(panic) => {
                let reason = panic
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                stderr_log(&format!(
                    "handler panicked: {reason}; answering typed error"
                ));
                protocol::handler_panic_error(&reason)
            }
        };
        // Protocol line + explicit flush: a stdio peer cannot see the
        // answer until the bytes actually leave the buffer.
        if let Err(err) = out
            .write_all(response.as_bytes())
            .and_then(|()| out.write_all(b"\n"))
            .and_then(|()| out.flush())
        {
            stderr_log(&format!("stdout write failed: {err}; stopping"));
            return ExitCode::FAILURE;
        }
    }

    stderr_log("stdin closed — exiting");
    ExitCode::SUCCESS
}

fn doctor() -> ExitCode {
    println!("rimloc-mcp doctor");
    println!(
        "build: v{}, MCP protocol {}",
        env!("CARGO_PKG_VERSION"),
        protocol::PROTOCOL_VERSION
    );
    println!(
        "supported protocol versions: {}",
        protocol::SUPPORTED_PROTOCOL_VERSIONS.join(", ")
    );
    // The profile verdict is resolved by the SAME decision serve() enforces
    // (resolve_profile), so doctor never reports a different truth. Pure
    // report only: doctor must stay side-effect free (no manager
    // construction, no lock, no directories).
    match runtime::resolve_profile() {
        runtime::ProfileResolution::Isolated {
            data_dir,
            managed_root,
        } => {
            println!("RIMLOC_DATA_DIR: {} (isolated profile)", data_dir.display());
            println!("resolved managed root: {}", managed_root.display());
        }
        runtime::ProfileResolution::Shared {
            reason,
            managed_root,
        } => {
            match reason {
                runtime::SharedProfileReason::DefaultProfile => println!(
                    "RIMLOC_DATA_DIR: not set (default profile — SHARED with the RimLoc \
                     GUI; writes and project-scoped reads refuse, diagnostics only)"
                ),
                runtime::SharedProfileReason::Refused { requested, cause } => println!(
                    "RIMLOC_DATA_DIR: {requested} — REFUSED ({cause}); shared-profile \
                     diagnostics mode, writes and project-scoped reads refuse"
                ),
            }
            println!(
                "resolved managed root (diagnostics-only): {}",
                managed_root.display()
            );
            println!("remedy: set RIMLOC_DATA_DIR to isolate");
        }
    }

    // Pure services report only: doctor must stay side-effect free (no
    // manager construction, no lock).
    let report = rimloc_services::contract::capability_report();
    println!(
        "rimloc-services ui contract version: {}",
        report.contract_version
    );
    println!(
        "supported slice operations: {} (rimloc.capabilities serves the full report)",
        report.supported.len()
    );
    println!("unsupported slice operations: {}", report.unsupported.len());

    println!("tools declared: {}", tools::REGISTRY.len());
    println!(
        "tools executable: {} ({} read-only + {} controlled writes; all wired)",
        tools::WIRED_READ_ONLY.len()
            + tools::WIRED_READ_ONLY_ALIASES.len()
            + tools::WIRED_WRITES.len(),
        tools::WIRED_READ_ONLY.len() + tools::WIRED_READ_ONLY_ALIASES.len(),
        tools::WIRED_WRITES.len()
    );
    println!(
        "serve mode: exclusive writer lock on ISOLATED managed roots only; \
         default/refused profiles are fail-closed shared diagnostics"
    );
    println!("doctor mode: side-effect free (no lock, no directories created)");
    println!(
        "transport: stdio only; no network listener; no secret access \
         (ai_job launches resolve provider keys in-process, identifiers only on the wire)"
    );
    ExitCode::SUCCESS
}
