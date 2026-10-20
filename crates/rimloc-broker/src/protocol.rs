//! Broker protocol v1: line-delimited JSON, both directions
//! (`MCP_SERVER_PHASE2.md` «Протокол v1»).
//!
//! - request  `{id, method, params}` → response `{id, result}` |
//!   `{id, error{code,message,data}}` — exactly one of `result`/`error` is
//!   present on the wire (the other is omitted, never null);
//! - unsolicited `{event, payload}` frames are pushed to EVERY connected
//!   client after each successful mutation
//!   ([`EVENT_REVISION_CHANGED`]);
//! - every frame is one compact JSON document terminated by `\n`; compact
//!   encoding guarantees no raw newline can ever appear inside a frame, so
//!   `\n` is a complete, unambiguous delimiter;
//! - error codes reuse the JSON-RPC 2.0 reserved numbers and the same
//!   server-defined values the rimloc-mcp layer already pinned
//!   (`crates/rimloc-mcp/src/error.rs`): clients switch on the same
//!   numbers across both transports.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use rimloc_services::ContractError;

/// Version of THIS broker wire protocol. A client that connects checks it
/// via the `broker.status`-style metadata on the answers it understands —
/// a mismatch is a client-side refusal, the server never negotiates
/// downwards.
pub const PROTOCOL_VERSION: u32 = 1;

/// The one unsolicited event kind in v1: emitted after EVERY successful
/// mutation so all connected clients (and the GUI host callback) refetch.
pub const EVENT_REVISION_CHANGED: &str = "revision_changed";

/// Hard per-line bound: a frame longer than this is refused
/// (`codes::LINE_TOO_LONG`) and the connection is closed — the framing
/// cannot be trusted past an oversized line.
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Numeric error codes. Reserved range follows JSON-RPC 2.0 §5.1; the
/// server-defined values live in -32000..-32099 and MIRROR the rimloc-mcp
/// codes so both transports share one client-side error vocabulary.
pub mod codes {
    /// The line is not valid JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// Valid JSON, but not a broker request object (missing/empty method).
    pub const INVALID_REQUEST: i64 = -32600;
    /// Unknown method name.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Params failed the typed shape of the method.
    pub const INVALID_PARAMS: i64 = -32602;
    /// Internal error — including a handler PANIC converted by
    /// `catch_unwind` (the server keeps living, this answer is typed).
    pub const INTERNAL_ERROR: i64 = -32603;

    /// The services contract refused the operation (stale epoch/revision,
    /// unknown project, guard denial, …). The STABLE snake_case contract
    /// code rides in `data.code` — this layer never invents contract codes
    /// (same discipline as `rimloc-mcp/src/runtime.rs::contract_error`).
    pub const CONTRACT_ERROR: i64 = -32002;
    /// The broker-side paid-guard: `ai_job.start` names a provider
    /// instance whose record has `local == false` — paid launches never
    /// run through the broker (`data.code =
    /// "paid_providers_disabled"`). Mirror of the MCP client guard.
    pub const PAID_PROVIDERS_DISABLED: i64 = -32004;
    /// The per-profile client bound is already at the maximum
    /// (`data.code = "too_many_clients"`).
    pub const BUSY: i64 = -32005;
    /// The line exceeded [`MAX_LINE_BYTES`] — the connection is closed
    /// after this answer.
    pub const LINE_TOO_LONG: i64 = -32006;
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// A client request. `id` is opaque and echoed verbatim; `params` is
/// method-specific (`Value::Null` when absent — argument-taking methods
/// then answer typed `invalid_params`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokerRequest {
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// The `error` member of a response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokerErrorBody {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// A server response: `id` echo plus EXACTLY ONE of `result` / `error`
/// (the absent side is skipped on the wire, never null).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokerResponse {
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<BrokerErrorBody>,
}

impl BrokerResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, err: &BrokerError) -> Self {
        Self {
            id,
            result: None,
            error: Some(BrokerErrorBody::from(err.clone())),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed broker error (server-side construction → wire body)
// ---------------------------------------------------------------------------

/// A typed broker refusal. `code` is one of [`codes`]; `data.code` (when
/// present) is the STABLE snake_case string clients switch on — the same
/// convention the MCP layer and the services contract use.
#[derive(Debug, Clone, PartialEq)]
pub struct BrokerError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl BrokerError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(codes::PARSE_ERROR, message).with_data(json!({ "code": "not_a_broker_request" }))
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(codes::INVALID_REQUEST, message).with_data(json!({ "code": "invalid_request" }))
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(codes::METHOD_NOT_FOUND, format!("unknown method: {method}"))
            .with_data(serde_json::json!({ "code": "method_not_found", "method": method }))
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(codes::INVALID_PARAMS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(codes::INTERNAL_ERROR, message)
    }

    /// A handler PANIC — caught, converted, never fatal (the ask: typed
    /// internal error, the server lives).
    pub fn handler_panicked(method: &str) -> Self {
        Self::internal(format!(
            "handler for `{method}` panicked; the broker keeps serving"
        ))
        .with_data(serde_json::json!({ "code": "handler_panicked", "method": method }))
    }

    /// Map a services contract refusal: one numeric code, the stable
    /// snake_case contract code in `data.code`, structured details passed
    /// through (they are sanitized upstream and never carry secrets).
    pub fn contract(err: ContractError) -> Self {
        Self::new(codes::CONTRACT_ERROR, err.to_string()).with_data(serde_json::json!({
            "code": err.code.as_str(),
            "details": err.details,
        }))
    }

    /// The broker-side paid-guard: the provider instance exists but is
    /// `local == false` (cloud/paid). Refused BEFORE any launch attempt;
    /// only identifiers ride the answer, never secrets.
    pub fn paid_providers_disabled(provider_instance_id: &str) -> Self {
        Self::new(
            codes::PAID_PROVIDERS_DISABLED,
            format!(
                "provider instance `{provider_instance_id}` is not local — paid providers \
                 are disabled on the broker; use a local instance (ollama-class) or run \
                 headless with your own profile"
            ),
        )
        .with_data(serde_json::json!({
            "code": "paid_providers_disabled",
            "provider_instance_id": provider_instance_id,
        }))
    }

    pub fn too_many_clients(limit: usize) -> Self {
        Self::new(
            codes::BUSY,
            format!("broker already serves the maximum of {limit} clients"),
        )
        .with_data(serde_json::json!({ "code": "too_many_clients", "limit": limit }))
    }

    pub fn line_too_long(limit: usize) -> Self {
        Self::new(
            codes::LINE_TOO_LONG,
            format!("line exceeds the {limit}-byte broker bound; connection closed"),
        )
        .with_data(serde_json::json!({ "code": "line_too_long", "limit": limit }))
    }
}

impl From<BrokerError> for BrokerErrorBody {
    fn from(err: BrokerError) -> Self {
        Self {
            code: err.code,
            message: err.message,
            data: err.data,
        }
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Payload of [`EVENT_REVISION_CHANGED`] (`MCP_SERVER_PHASE2.md`): what
/// changed, at which durable revision, under which session epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionChangedPayload {
    pub project_id: String,
    pub revision: u64,
    pub session_epoch: u64,
    /// Stable snake_case mutation kind (`project_open`, `propose`,
    /// `apply`, `ai_job_start`, `ai_job_cancel`, `accept_selected`,
    /// `export_build`) — see `ops.rs` for the per-method source of the
    /// coordinates.
    pub kind: String,
}

/// An unsolicited server→client frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrokerEvent {
    pub event: String,
    pub payload: Value,
}

impl BrokerEvent {
    pub fn revision_changed(payload: RevisionChangedPayload) -> Self {
        Self {
            event: EVENT_REVISION_CHANGED.to_string(),
            payload: serde_json::to_value(payload).unwrap_or(Value::Null),
        }
    }
}

// ---------------------------------------------------------------------------
// Line framing
// ---------------------------------------------------------------------------

/// Encode one frame: compact JSON + `\n`. Compact encoding is the newline
/// safety argument (see module docs) — never switch to pretty.
pub fn encode_line<T: Serialize>(value: &T) -> serde_json::Result<Vec<u8>> {
    let mut buf = serde_json::to_vec(value)?;
    buf.push(b'\n');
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codes;

    #[test]
    fn request_round_trips_through_a_line() {
        let req = BrokerRequest {
            id: serde_json::json!(7),
            method: "project.snapshot".to_string(),
            params: serde_json::json!({ "project_id": "proj-x", "session_epoch": 3 }),
        };
        let line = encode_line(&req).expect("encode");
        assert!(line.ends_with(b"\n"));
        // Compact: a raw newline inside the frame would break the delimiter.
        assert_eq!(line.iter().filter(|b| **b == b'\n').count(), 1);
        let back: BrokerRequest = serde_json::from_slice(&line).expect("decode");
        assert_eq!(back, req);
    }

    #[test]
    fn response_round_trips_and_omits_the_absent_side() {
        let ok = BrokerResponse::ok(serde_json::json!("a"), serde_json::json!({ "v": 1 }));
        let line = encode_line(&ok).expect("encode");
        let wire: Value = serde_json::from_slice(&line).expect("wire");
        assert!(wire.get("result").is_some());
        assert!(
            wire.get("error").is_none(),
            "absent side must be omitted, never null"
        );

        let err = BrokerResponse::err(serde_json::json!(null), &BrokerError::internal("boom"));
        let wire: Value =
            serde_json::from_slice(&encode_line(&err).expect("encode")).expect("wire");
        assert!(wire.get("result").is_none());
        assert_eq!(wire["error"]["code"], codes::INTERNAL_ERROR);
        assert_eq!(wire["error"]["message"], "boom");
        let back: BrokerResponse = serde_json::from_slice(&encode_line(&err).unwrap()).unwrap();
        assert_eq!(back, err);
    }

    #[test]
    fn revision_changed_event_carries_the_design_payload() {
        let event = BrokerEvent::revision_changed(RevisionChangedPayload {
            project_id: "proj-1".to_string(),
            revision: 2,
            session_epoch: 1,
            kind: "apply".to_string(),
        });
        assert_eq!(event.event, "revision_changed");
        let wire: Value =
            serde_json::from_slice(&encode_line(&event).expect("encode")).expect("wire");
        assert_eq!(wire["event"], "revision_changed");
        assert_eq!(wire["payload"]["project_id"], "proj-1");
        assert_eq!(wire["payload"]["revision"], 2);
        assert_eq!(wire["payload"]["session_epoch"], 1);
        assert_eq!(wire["payload"]["kind"], "apply");
        let back: BrokerEvent = serde_json::from_slice(&encode_line(&event).unwrap()).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn contract_error_maps_to_the_stable_snake_case_code() {
        let err = BrokerError::contract(ContractError::stale_revision(7, 9));
        assert_eq!(err.code, codes::CONTRACT_ERROR);
        let data = err.data.expect("typed data");
        assert_eq!(data["code"], "stale_revision");
    }

    #[test]
    fn paid_guard_error_is_typed_and_secret_free() {
        let err = BrokerError::paid_providers_disabled("prov-abc");
        assert_eq!(err.code, codes::PAID_PROVIDERS_DISABLED);
        assert_eq!(
            err.data.as_ref().expect("data")["code"],
            "paid_providers_disabled"
        );
        assert_eq!(
            err.data.as_ref().expect("data")["provider_instance_id"],
            "prov-abc"
        );
        // No secret can leak: the constructor takes the ID only, and the
        // answer names the instance, not any credential material.
        assert!(err.message.contains("prov-abc"));
        assert!(!err.message.contains("sk-"));
    }

    #[test]
    fn server_codes_stay_in_the_reserved_range_and_distinct() {
        for code in [
            codes::CONTRACT_ERROR,
            codes::PAID_PROVIDERS_DISABLED,
            codes::BUSY,
            codes::LINE_TOO_LONG,
        ] {
            assert!(
                (-32099..=-32000).contains(&code),
                "server code {code} must live in the reserved range"
            );
        }
        let all = [
            codes::PARSE_ERROR,
            codes::INVALID_REQUEST,
            codes::METHOD_NOT_FOUND,
            codes::INVALID_PARAMS,
            codes::INTERNAL_ERROR,
            codes::CONTRACT_ERROR,
            codes::PAID_PROVIDERS_DISABLED,
            codes::BUSY,
            codes::LINE_TOO_LONG,
        ];
        let mut sorted = all.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "codes must stay distinct");
    }
}
