//! The Phase 2 broker_client mode (`docs/development/MCP_SERVER_PHASE2.md`,
//! «MCP-режим BrokerClient»): when a live RimLoc GUI hosts the broker on
//! this profile, the external MCP server becomes a THIN PROXY — every
//! project-scoped tool and every controlled write executes through the
//! GUI's single-writer broker instead of local session machinery.
//!
//! Connection discipline (`MCP_SERVER_PHASE2.md` «Broker client»): at
//! startup [`BrokerClient::connect`] probes the profile socket; a dead or
//! missing socket answers `None` and the Phase 1 headless rules apply
//! EXACTLY as before (fail-closed ladder untouched). Every subsequent
//! call opens a FRESH connection (stateless, one request per connection —
//! the per-call reconnect the phase ask pins); a broker that died between
//! startup and a call answers a TYPED `broker_unavailable` refusal, never
//! a panic and never a local fallback execution.
//!
//! Proxy shape (the ask: «те же имена, params→params, ответ
//! result→structuredContent»): the broker method name is the MCP tool
//! name minus the `rimloc.` prefix, the raw tool arguments ride the wire
//! as broker `params` verbatim (the broker's typed serde is the single
//! validator — no second validation dialect), the broker `result` becomes
//! the MCP `structuredContent`, and broker errors map onto the RPC codes
//! 1:1 (see [`broker_to_rpc_code`] for the one intentional exception).
//!
//! # PAID-GUARD IN BROKER MODE (не ослаблен — задокументировано в коде)
//!
//! `rimloc.ai_job.start` is PROXIED AS-IS. The MCP-side guard cannot run
//! here: the broker surface deliberately has NO provider-instance list
//! method (instances/keys never cross the broker wire), so the `local`
//! flag is unreachable from this process. That is sufficient BY DESIGN
//! (`MCP_SERVER_PHASE2.md` DoD: «paid-guard дублируется на брокере»): the
//! broker's `OpsImpl::ai_job_start` resolves the instance record and
//! refuses `paid_providers_disabled` BEFORE any launch — the same
//! fail-closed check, enforced on the side that actually spawns the
//! worker. The proxy never relaxes it, never retries around it, and the
//! refusal maps to `contract_error` with the stable
//! `data.code = "paid_providers_disabled"` (integration test:
//! `tests/broker_client.rs` pins the full path).
//!
//! # Bounds and secrets
//!
//! The client mirrors the broker's own bounds: one line at most
//! [`rimloc_broker::protocol::MAX_LINE_BYTES`], a per-response read
//! timeout ([`READ_TIMEOUT`]) so a hung broker degrades to a typed
//! refusal instead of an infinite stdio hang, and a frame cap guarding
//! against an event flood. No secrets traverse this module in either
//! direction: ai_job.start carries an instance ID only, and the proxy
//! forwards exactly the arguments the MCP client sent.

#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rimloc_broker::protocol::codes as broker_codes;
#[cfg(unix)]
use rimloc_broker::protocol::{encode_line, BrokerRequest, BrokerResponse};

use crate::error::{codes, RpcError};

/// The broker socket file name inside the PROFILE directory (the parent
/// of the managed root). Mirrors
/// `crates/rimloc-broker/src/server.rs::SOCKET_FILE_NAME`; that crate is
/// unix-gated as a module, so the constant is repeated here and the
/// integration tests pin both ends to the same path — drift fails loudly.
pub const BROKER_SOCKET_FILE_NAME: &str = "rimloc-broker.sock";

/// Broker-side refusal while the proxy waits for the answer
/// (`data.code == "broker_unavailable"`). New server code in the reserved
/// -32000..-32099 range: distinct from NOT_IMPLEMENTED (-32001),
/// CONTRACT_ERROR (-32002), OTHER_WRITER (-32003) and the
/// shared-profile code (-32004) — pinned by the unit test below, same
/// discipline as `runtime::CODE_PROFILE_SHARED`.
pub const CODE_BROKER_UNAVAILABLE: i64 = -32005;

/// Write timeout for one broker request (the request frames are
/// kilobytes; 30s is generous by orders of magnitude).
const WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Read timeout for one broker answer. The slowest legitimate calls are
/// `export.build` (reparse-verified serializer) and `ai_job.start` (sync
/// launch ack only — the worker is async); 120s bounds a hung broker
/// without cutting a real export short.
const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Maximum frames read while waiting for OUR response: the only legal
/// interleavings are `revision_changed` events fanned out to every
/// client. Past this the peer is misbehaving — typed internal refusal.
const MAX_INTERLEAVED_FRAMES: usize = 4096;

/// What is known about the GUI process behind the broker. The protocol v1
/// socket carries NO peer identity, and the broker writes no pid file, so
/// `pid` stays `None` and the surfaces report it honestly (`writer_pid:
/// null`, label `gui-broker`) instead of inventing an owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerPeerInfo {
    /// The live socket this client talks to (`<profile>/rimloc-broker.sock`).
    pub socket_path: PathBuf,
    /// The GUI's pid is NOT discoverable from the broker protocol v1 —
    /// always `None` today, reported as an honest null, never guessed.
    pub pid: Option<u32>,
}

impl BrokerPeerInfo {
    /// Stable human label for status surfaces: the peer is a RimLoc
    /// GUI-process broker, not a generic service.
    pub fn label(&self) -> &'static str {
        "gui-broker"
    }
}

/// A live broker connection seam for one profile. Construction proves a
/// broker was ALIVE at startup; the per-call reconnect keeps every
/// subsequent call honest about the broker's current state.
#[derive(Debug)]
pub struct BrokerClient {
    socket_path: PathBuf,
    next_id: AtomicU64,
}

impl BrokerClient {
    /// The profile socket this client serves.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// The honest peer description for status surfaces.
    pub fn peer_info(&self) -> BrokerPeerInfo {
        BrokerPeerInfo {
            socket_path: self.socket_path.clone(),
            // The broker protocol v1 exposes no peer identity — None is
            // the truth, not a gap to paper over.
            pid: None,
        }
    }
}

#[cfg(unix)]
impl BrokerClient {
    /// Probe the profile socket: a live broker yields the client, a dead
    /// or missing socket yields `None` and the caller falls through to
    /// the Phase 1 headless rules (never an error — absence of the GUI is
    /// a normal state, not a failure).
    ///
    /// Per the design («stale (ECONNREFUSED у клиента) — клиент удаляет и
    /// работает headless»), a ConnectionRefused node is unlinked so the
    /// next GUI start can bind cleanly; any other connect error leaves
    /// the node untouched (only the provably-dead case is cleaned).
    pub fn connect(profile_dir: &Path) -> Option<Self> {
        // Path guard before any fs effect: the refused case would unlink a
        // socket node, so the profile directory itself is validated first
        // (absolute, traversal-free). A refused profile simply has no
        // broker — the headless fallthrough — never a cleanup attempt on
        // an unvalidated path.
        if !profile_dir.is_absolute()
            || profile_dir
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return None;
        }
        let socket_path = profile_dir.join(BROKER_SOCKET_FILE_NAME);
        match std::os::unix::net::UnixStream::connect(&socket_path) {
            // Live broker proved; the probe stream is dropped — real calls
            // reconnect per call (stateless discipline, module docs).
            Ok(_probe) => Some(Self {
                socket_path,
                next_id: AtomicU64::new(1),
            }),
            Err(err) if err.kind() == std::io::ErrorKind::ConnectionRefused => {
                // Stale node from a crashed broker: the design prescribes
                // client-side removal + headless fallthrough.
                let _ = std::fs::remove_file(&socket_path);
                None
            }
            Err(_) => None,
        }
    }

    /// Execute ONE broker method and return the `result` value. Fresh
    /// connection per call; every failure mode is a typed RPC error, so
    /// the stdio session above never panics and never hangs forever.
    pub fn call(
        &self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let mut stream = self.open_stream()?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = BrokerRequest {
            id: serde_json::json!(id),
            method: method.to_string(),
            params: params.clone(),
        };
        let line = encode_line(&request).map_err(|err| {
            RpcError::internal(format!("broker request serialization failed: {err}"))
                .with_data(serde_json::json!({ "code": "broker_request_encode_failed" }))
        })?;
        stream
            .write_all(&line)
            .and_then(|()| stream.flush())
            .map_err(|err| broker_unavailable_io(&self.socket_path, "write", err))?;

        let mut reader = BufReader::new(stream);
        for _ in 0..MAX_INTERLEAVED_FRAMES {
            let frame = match read_line_bounded(&mut reader) {
                Ok(Some(frame)) => frame,
                Ok(None) => {
                    return Err(broker_unavailable(
                        &self.socket_path,
                        "the broker closed the connection before answering",
                    ));
                }
                Err(err) => return Err(broker_unavailable_io(&self.socket_path, "read", err)),
            };
            let value: serde_json::Value = serde_json::from_slice(&frame).map_err(|err| {
                RpcError::internal(format!(
                    "broker protocol violation (frame is not JSON): {err}"
                ))
                .with_data(serde_json::json!({ "code": "broker_protocol_violation" }))
            })?;
            let Some(id_on_wire) = value.get("id") else {
                // Unsolicited event frame (`revision_changed` fan-out
                // reaches every client) — not our answer, keep reading.
                continue;
            };
            if *id_on_wire != request.id && value.get("error").is_none() {
                // A success frame for a foreign id cannot occur on a
                // dedicated per-call connection; skip defensively.
                continue;
            }
            let response: BrokerResponse = serde_json::from_value(value).map_err(|err| {
                RpcError::internal(format!("broker protocol violation (response): {err}"))
                    .with_data(serde_json::json!({ "code": "broker_protocol_violation" }))
            })?;
            return match response.result {
                Some(result) => Ok(result),
                None => Err(map_broker_error(
                    response
                        .error
                        .expect("exactly one of result/error rides the wire"),
                )),
            };
        }
        Err(RpcError::internal(format!(
            "broker sent more than {MAX_INTERLEAVED_FRAMES} frames without answering"
        ))
        .with_data(serde_json::json!({ "code": "broker_event_flood" })))
    }

    #[cfg(unix)]
    fn open_stream(&self) -> Result<std::os::unix::net::UnixStream, RpcError> {
        use std::os::unix::net::UnixStream;
        let stream = UnixStream::connect(&self.socket_path)
            .map_err(|err| broker_unavailable_io(&self.socket_path, "connect", err))?;
        stream
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .and_then(|()| stream.set_read_timeout(Some(READ_TIMEOUT)))
            .map_err(|err| broker_unavailable_io(&self.socket_path, "timeout setup", err))?;
        Ok(stream)
    }
}

#[cfg(not(unix))]
impl BrokerClient {
    /// Windows has no broker transport in Phase 2 (named pipe is Phase 3):
    /// no probe can ever succeed, so the headless rules always apply —
    /// same posture as the `#[cfg(unix)]`-gated broker server module.
    pub fn connect(_profile_dir: &Path) -> Option<Self> {
        None
    }

    pub fn call(
        &self,
        _method: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        Err(broker_unavailable(
            &self.socket_path,
            "the broker transport is unix-only in Phase 2 (windows named pipe: Phase 3)",
        ))
    }
}

// ---------------------------------------------------------------------------
// Error mapping: broker codes → RPC codes, 1:1 per the mirror table
// ---------------------------------------------------------------------------

/// The mirror table (`crates/rimloc-broker/src/protocol.rs` deliberately
/// reuses the rimloc-mcp numeric codes): reserved JSON-RPC codes and
/// CONTRACT_ERROR map identity. One intentional exception and one
/// fallback:
///
/// - broker `PAID_PROVIDERS_DISABLED` (-32004) COLLIDES NUMERICALLY with
///   the MCP-side `runtime::CODE_PROFILE_SHARED` (-32004) — mapping it
///   1:1 would report a paid-guard refusal as "profile shared", which is
///   false and dangerous. Semantically it is a services-contract refusal
///   (the paid guard), so it maps to `CONTRACT_ERROR` (-32002) with the
///   stable `data.code = "paid_providers_disabled"` preserved — exactly
///   how the local MCP guard refuses.
/// - broker-specific transport/admission codes (`BUSY`, `LINE_TOO_LONG`)
///   have no MCP twin; they fall back to `INTERNAL_ERROR` and the stable
///   `data.code` (`too_many_clients` / `line_too_long`) plus the raw
///   `broker_code` ride along — clients switch on the string, the number
///   stays traceable. Unknown codes do the same: never invented, never
///   swallowed.
fn broker_to_rpc_code(broker_code: i64) -> i64 {
    match broker_code {
        broker_codes::PARSE_ERROR => codes::PARSE_ERROR,
        broker_codes::INVALID_REQUEST => codes::INVALID_REQUEST,
        broker_codes::METHOD_NOT_FOUND => codes::METHOD_NOT_FOUND,
        broker_codes::INVALID_PARAMS => codes::INVALID_PARAMS,
        broker_codes::INTERNAL_ERROR => codes::INTERNAL_ERROR,
        broker_codes::CONTRACT_ERROR => codes::CONTRACT_ERROR,
        broker_codes::PAID_PROVIDERS_DISABLED => codes::CONTRACT_ERROR,
        broker_codes::BUSY | broker_codes::LINE_TOO_LONG => codes::INTERNAL_ERROR,
        _other => codes::INTERNAL_ERROR,
    }
}

/// `true` when the numeric mapping is lossy and the answer must carry the
/// raw broker code for traceability.
fn mapping_is_lossy(broker_code: i64) -> bool {
    !matches!(
        broker_code,
        broker_codes::PARSE_ERROR
            | broker_codes::INVALID_REQUEST
            | broker_codes::METHOD_NOT_FOUND
            | broker_codes::INVALID_PARAMS
            | broker_codes::INTERNAL_ERROR
            | broker_codes::CONTRACT_ERROR
    )
}

/// Map a broker error body onto the RPC surface: the mapped numeric code,
/// the broker's message verbatim, the broker's `data` verbatim (the
/// stable snake_case code inside survives — the MCP layer never invents
/// contract codes), and `broker_code` added only when the numeric mapping
/// is lossy.
fn map_broker_error(body: rimloc_broker::protocol::BrokerErrorBody) -> RpcError {
    let err = RpcError::new(broker_to_rpc_code(body.code), body.message);
    match body.data {
        Some(data) if mapping_is_lossy(body.code) => {
            let mut data = data;
            if let Some(obj) = data.as_object_mut() {
                obj.insert("broker_code".to_string(), serde_json::json!(body.code));
            }
            err.with_data(data)
        }
        Some(data) => err.with_data(data),
        None if mapping_is_lossy(body.code) => err.with_data(serde_json::json!({
            "broker_code": body.code,
        })),
        None => err,
    }
}

/// The typed "the broker is not usable right now" refusal. Startup proved
/// the broker alive; by call time it may be gone — that state gets a
/// stable code and remediation, never a panic and never a silent local
/// execution (which would break the single-writer invariant).
fn broker_unavailable(socket_path: &Path, detail: &str) -> RpcError {
    RpcError::new(
        CODE_BROKER_UNAVAILABLE,
        format!(
            "the RimLoc GUI broker at `{}` is unavailable: {detail}; \
             the GUI process is the single writer — restart the GUI (or stop \
             it to fall back to headless rules with an isolated \
             RIMLOC_DATA_DIR), then retry",
            socket_path.display()
        ),
    )
    .with_data(serde_json::json!({
        "code": "broker_unavailable",
        "socket_path": socket_path.display().to_string(),
    }))
}

#[cfg(unix)]
fn broker_unavailable_io(socket_path: &Path, phase: &str, err: std::io::Error) -> RpcError {
    let detail = match err.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => format!(
            "no {phase} progress within the timeout (write {WRITE_TIMEOUT:?}, read \
             {READ_TIMEOUT:?})"
        ),
        _ => format!("{phase} failed: {err}"),
    };
    broker_unavailable(socket_path, &detail)
}

#[cfg(not(unix))]
fn broker_unavailable_io(socket_path: &Path, phase: &str, err: std::io::Error) -> RpcError {
    broker_unavailable(socket_path, &format!("{phase} failed: {err}"))
}

/// One `\n`-terminated broker frame, accumulated with the broker's own
/// hard cap (the client must not be OOM-able by its peer). `Ok(None)` on
/// a clean EOF at a frame boundary; a read timeout surfaces as
/// `WouldBlock`/`TimedOut` and propagates to the typed refusal.
#[cfg(unix)]
fn read_line_bounded(
    reader: &mut BufReader<std::os::unix::net::UnixStream>,
) -> std::io::Result<Option<Vec<u8>>> {
    let cap = rimloc_broker::protocol::MAX_LINE_BYTES;
    let mut out = Vec::new();
    loop {
        let available = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };
        if available.is_empty() {
            return if out.is_empty() {
                Ok(None) // clean EOF at a frame boundary
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "broker closed the connection mid-frame",
                ))
            };
        }
        if let Some(pos) = available.iter().position(|&b| b == b'\n') {
            out.extend_from_slice(&available[..pos]);
            reader.consume(pos + 1);
            if out.len() > cap {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("broker frame exceeds the {cap}-byte bound"),
                ));
            }
            return Ok(Some(out));
        }
        let chunk = available.len();
        out.extend_from_slice(available);
        reader.consume(chunk);
        if out.len() > cap {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("broker frame exceeds the {cap}-byte bound"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Proxy entry used by tools::call
// ---------------------------------------------------------------------------

/// Map an MCP tool name onto the broker method (`rimloc.x.y` → `x.y`, the
/// protocol v1 names are 1:1 — `crates/rimloc-broker/src/ops.rs` table).
pub fn broker_method_for_tool(tool: &str) -> Result<&'static str, RpcError> {
    let method = tool.strip_prefix("rimloc.").ok_or_else(|| {
        RpcError::internal(format!(
            "tool `{tool}` is not a rimloc.* name; the broker proxy cannot route it"
        ))
        .with_data(serde_json::json!({ "code": "broker_route_failed", "tool": tool }))
    })?;
    // The registry is a fixed table; only names the broker protocol v1
    // defines may be proxied (exact match, same fixed-table discipline).
    const BROKER_V1_METHODS: &[&str] = &[
        "projects.list",
        "project.snapshot",
        "entries.search",
        "entry.context",
        "validation.run",
        "export.preview",
        "jobs.status",
        "project.open",
        "translation.propose",
        "translation.apply",
        "ai_job.start",
        "ai_job.status",
        "ai_job.cancel",
        "ai_job.accept_selected",
        "export.build",
    ];
    if BROKER_V1_METHODS.contains(&method) {
        // SAFE: `method` is one of the &'static strings above.
        Ok(BROKER_V1_METHODS
            .iter()
            .find(|m| **m == method)
            .expect("matched above"))
    } else {
        Err(RpcError::internal(format!(
            "tool `{tool}` has no broker v1 method; it must not proxy"
        ))
        .with_data(serde_json::json!({ "code": "broker_route_failed", "tool": tool })))
    }
}

/// Execute one MCP tool call through the broker: name → method, arguments
/// → params verbatim, result → `structuredContent` + a short summary,
/// broker error → mapped RPC error. `PAID GUARD` note: `rimloc.ai_job.start`
/// rides this path AS-IS — the guard is the broker's own
/// (`OpsImpl::ai_job_start`, `paid_providers_disabled` before any launch);
/// see the module docs for why the MCP-side guard cannot and must not run
/// here.
pub fn proxy_tool_call(
    tool: &str,
    arguments: &serde_json::Value,
    client: &BrokerClient,
) -> Result<serde_json::Value, RpcError> {
    let method = broker_method_for_tool(tool)?;
    let result = client.call(method, arguments)?;
    let summary = match (
        result.get("project_id").and_then(|v| v.as_str()),
        result.get("revision").and_then(|v| v.as_u64()),
    ) {
        (Some(project), Some(revision)) => format!(
            "{method} ok via the gui broker (project `{project}`, revision {revision}); \
             details in structuredContent"
        ),
        (Some(project), None) => format!(
            "{method} ok via the gui broker (project `{project}`); \
             details in structuredContent"
        ),
        _ => format!("{method} ok via the gui broker; details in structuredContent"),
    };
    crate::runtime::tool_ok(result, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CODE_PROFILE_SHARED;

    /// The new client-side code stays inside the JSON-RPC server-defined
    /// range and distinct from every other server code — clients switch
    /// on them (same pin as `runtime::CODE_PROFILE_SHARED`).
    #[test]
    fn broker_unavailable_code_is_in_range_and_distinct() {
        assert!(
            (-32099..=-32000).contains(&CODE_BROKER_UNAVAILABLE),
            "CODE_BROKER_UNAVAILABLE must live in the reserved range"
        );
        for code in [
            codes::NOT_IMPLEMENTED,
            codes::CONTRACT_ERROR,
            codes::OTHER_WRITER,
            CODE_PROFILE_SHARED,
        ] {
            assert_ne!(
                CODE_BROKER_UNAVAILABLE, code,
                "server codes must stay distinct"
            );
        }
    }

    /// The mirror table: reserved + contract codes map identity.
    #[test]
    fn broker_codes_map_one_to_one() {
        assert_eq!(
            broker_to_rpc_code(broker_codes::PARSE_ERROR),
            codes::PARSE_ERROR
        );
        assert_eq!(
            broker_to_rpc_code(broker_codes::INVALID_REQUEST),
            codes::INVALID_REQUEST
        );
        assert_eq!(
            broker_to_rpc_code(broker_codes::METHOD_NOT_FOUND),
            codes::METHOD_NOT_FOUND
        );
        assert_eq!(
            broker_to_rpc_code(broker_codes::INVALID_PARAMS),
            codes::INVALID_PARAMS
        );
        assert_eq!(
            broker_to_rpc_code(broker_codes::INTERNAL_ERROR),
            codes::INTERNAL_ERROR
        );
        assert_eq!(
            broker_to_rpc_code(broker_codes::CONTRACT_ERROR),
            codes::CONTRACT_ERROR
        );
    }

    /// The paid-guard refusal keeps its STABLE string code and lands on
    /// CONTRACT_ERROR — never on CODE_PROFILE_SHARED, despite the numeric
    /// -32004 collision between the two vocabularies.
    #[test]
    fn paid_guard_code_maps_to_contract_error_not_profile_shared() {
        let mapped = broker_to_rpc_code(broker_codes::PAID_PROVIDERS_DISABLED);
        assert_eq!(mapped, codes::CONTRACT_ERROR);
        assert_ne!(mapped, CODE_PROFILE_SHARED);

        let err = map_broker_error(rimloc_broker::protocol::BrokerErrorBody {
            code: broker_codes::PAID_PROVIDERS_DISABLED,
            message: "provider instance `prov-x` is not local".to_string(),
            data: Some(serde_json::json!({
                "code": "paid_providers_disabled",
                "provider_instance_id": "prov-x",
            })),
        });
        assert_eq!(err.code, codes::CONTRACT_ERROR);
        let data = err.data.expect("typed data");
        assert_eq!(data["code"], "paid_providers_disabled");
        assert_eq!(data["provider_instance_id"], "prov-x");
        assert_eq!(
            data["broker_code"], -32004,
            "lossy mapping keeps the raw code"
        );
    }

    /// Broker-specific and unknown codes fall back honestly: INTERNAL
    /// numeric, stable string preserved, raw broker code attached.
    #[test]
    fn broker_specific_codes_degrade_with_the_string_code_preserved() {
        for (broker_code, snake) in [
            (broker_codes::BUSY, "too_many_clients"),
            (broker_codes::LINE_TOO_LONG, "line_too_long"),
            (-32099, "something_new"),
        ] {
            let err = map_broker_error(rimloc_broker::protocol::BrokerErrorBody {
                code: broker_code,
                message: "detail".to_string(),
                data: Some(serde_json::json!({ "code": snake })),
            });
            assert_eq!(err.code, codes::INTERNAL_ERROR, "{snake}");
            let data = err.data.expect("data");
            assert_eq!(data["code"], snake, "{snake}");
            assert_eq!(data["broker_code"], broker_code, "{snake}");
        }
    }

    /// 1:1-mapped codes pass `data` through untouched (no broker_code
    /// noise) and absent data stays absent on the wire.
    #[test]
    fn identity_mapping_passes_data_through_untouched() {
        let err = map_broker_error(rimloc_broker::protocol::BrokerErrorBody {
            code: broker_codes::CONTRACT_ERROR,
            message: "stale".to_string(),
            data: Some(serde_json::json!({ "code": "stale_revision" })),
        });
        assert_eq!(err.code, codes::CONTRACT_ERROR);
        let data = err.data.expect("data");
        assert_eq!(data["code"], "stale_revision");
        assert!(
            data.get("broker_code").is_none(),
            "identity mapping adds nothing"
        );

        let err = map_broker_error(rimloc_broker::protocol::BrokerErrorBody {
            code: broker_codes::INVALID_PARAMS,
            message: "bad".to_string(),
            data: None,
        });
        assert_eq!(err.code, codes::INVALID_PARAMS);
        assert!(err.data.is_none(), "absent data stays absent");
    }

    /// No socket → `None` (headless rules), never an error; the profile
    /// directory does not even have to exist.
    #[cfg(unix)]
    #[test]
    fn connect_without_a_socket_answers_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let profile = dir.path().join("profile");
        assert!(BrokerClient::connect(&profile).is_none());
        assert!(
            !profile.join(BROKER_SOCKET_FILE_NAME).exists(),
            "a missing socket must not be created by the probe"
        );
    }

    /// A socket nobody serves (ConnectionRefused) → `None` AND the stale
    /// node is removed per the design, so the next GUI start can bind.
    #[cfg(unix)]
    #[test]
    fn connect_to_a_dead_socket_removes_the_stale_node_and_answers_none() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().expect("tempdir");
        let profile = dir.path().join("profile");
        std::fs::create_dir_all(&profile).expect("profile dir");
        let socket_path = profile.join(BROKER_SOCKET_FILE_NAME);
        let listener = UnixListener::bind(&socket_path).expect("bind");
        drop(listener); // dead node where the socket was — the crash shape

        assert!(BrokerClient::connect(&profile).is_none());
        assert!(
            !socket_path.exists(),
            "a ConnectionRefused node must be unlinked by the client"
        );
    }

    /// The broker surface is a fixed table: every v1 method routes, the
    /// status/capabilities surfaces and invented names refuse.
    #[test]
    fn tool_names_route_only_onto_broker_v1_methods() {
        for tool in [
            "rimloc.projects.list",
            "rimloc.project.snapshot",
            "rimloc.entries.search",
            "rimloc.entry.context",
            "rimloc.validation.run",
            "rimloc.export.preview",
            "rimloc.jobs.status",
            "rimloc.project.open",
            "rimloc.translation.propose",
            "rimloc.translation.apply",
            "rimloc.ai_job.start",
            "rimloc.ai_job.status",
            "rimloc.ai_job.cancel",
            "rimloc.ai_job.accept_selected",
            "rimloc.export.build",
        ] {
            assert!(
                broker_method_for_tool(tool).is_ok(),
                "{tool} must route to the broker"
            );
        }
        for tool in [
            "rimloc.status",
            "rimloc.capabilities",
            "rimloc.unknown",
            "status",
        ] {
            assert!(
                broker_method_for_tool(tool).is_err(),
                "{tool} must never route to the broker"
            );
        }
    }

    /// A client whose broker died between startup and the call answers a
    /// typed `broker_unavailable` — the exact state the integration test
    /// exercises end-to-end through the spawned binary.
    #[cfg(unix)]
    #[test]
    fn call_after_broker_death_is_typed_not_a_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let profile = dir.path().join("profile");
        std::fs::create_dir_all(&profile).expect("profile dir");
        let socket_path = profile.join(BROKER_SOCKET_FILE_NAME);
        let client = {
            use std::os::unix::net::UnixListener;
            let listener = UnixListener::bind(&socket_path).expect("bind");
            let client = BrokerClient::connect(&profile).expect("alive at startup");
            drop(listener); // the broker "dies"
            client
        };
        let err = client
            .call("projects.list", &serde_json::json!({}))
            .expect_err("the broker is gone");
        assert_eq!(err.code, CODE_BROKER_UNAVAILABLE);
        assert_eq!(err.data.expect("data")["code"], "broker_unavailable");
    }
}

#[cfg(test)]
mod path_guard_tests {
    use super::BrokerClient;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn connect_refuses_unvalidated_profile_dirs() {
        assert!(BrokerClient::connect(Path::new("relative/profile")).is_none());
        assert!(BrokerClient::connect(Path::new("/a/../escape")).is_none());
    }
}
