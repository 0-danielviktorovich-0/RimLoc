//! MCP 2026-07-28 wire surface: line-delimited JSON-RPC 2.0 over stdio.
//!
//! Covered here: the response envelope, `initialize` with protocol-version
//! negotiation, `notifications/initialized`, `tools/list`, `tools/call`,
//! `ping`, and stateless per-request `_meta` handling.
//!
//! Statelessness: the server keeps NO per-connection state — every inbound
//! line is interpreted on its own (the Phase 1 mandate "stateless
//! per-request `_meta`"). Clients still follow the MCP session sequence
//! (initialize → notifications/initialized → tools/...); the server does
//! not enforce it, because enforcing would require session state. `_meta`
//! in params (e.g. the `progressToken` the testlab client attaches to every
//! request, `testlab/rimworld/gabs_mcp.py`) is accepted and ignored — the
//! skeleton emits no progress notifications, and says so rather than
//! pretending.
//!
//! Protocol-version negotiation: if the client asks for a version we
//! actually support, we answer THAT version; otherwise we answer our own
//! latest. A version enters `SUPPORTED_PROTOCOL_VERSIONS` only with real
//! client tests behind it (AGENT_INTEROPERABILITY_QUALITY_STANDARD §1) —
//! hence a single honest entry today.
//!
//! Framing decisions (all typed refusals, never a hang or a crash):
//! - a line that is not valid JSON → parse error (-32700), id null;
//! - a line holding a JSON array (JSON-RPC batch) → invalid request
//!   (-32600): MCP 2026-07-28 does not use batching (removed from the
//!   protocol since 2025-06-18);
//! - a missing `jsonrpc` member is tolerated as 2.0 (logged to stderr);
//!   a WRONG `jsonrpc` value is an invalid request;
//! - notifications (no `id`) never produce a response, known or not.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RpcError;

/// The protocol revision this server is written against.
pub const PROTOCOL_VERSION: &str = "2026-07-28";

/// Versions we can answer with. A revision is added here ONLY after real
/// client tests (quality standard §1: "Test real clients for each
/// advertised revision") — do not list revisions we never ran.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[PROTOCOL_VERSION];

/// Server name reported in `initialize` results.
pub const SERVER_NAME: &str = "rimloc-mcp";

/// Answer the client's requested protocol version when we support it,
/// otherwise our own latest (`MCP_SERVER_PHASE1.md`: «отвечаем версией
/// клиента, если она нам известна, иначе своей»).
pub fn negotiate_protocol_version(client_version: Option<&str>) -> &'static str {
    match client_version {
        Some(requested) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|known| *known == requested)
            .unwrap_or(PROTOCOL_VERSION),
        None => PROTOCOL_VERSION,
    }
}

// ---------------------------------------------------------------------------
// Wire types (MCP 2026-07-28 shapes actually used by this server).
// ---------------------------------------------------------------------------

/// A JSON-RPC response: exactly one of `result` / `error` is set. `id` is
/// always serialized — `null` when the request id was undetectable
/// (JSON-RPC 2.0 §5.1 requires the null id on parse/invalid requests).
#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: Value, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// `initialize` params (camelCase per MCP). Everything but `method` is
/// optional on receive: a broken params object must degrade to a typed
/// invalid-params error, not to a panic.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    #[serde(default)]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub capabilities: Option<Value>,
    #[serde(default)]
    pub client_info: Option<Implementation>,
    /// Stateless per-request `_meta` — accepted, ignored.
    #[serde(default)]
    pub _meta: Option<Value>,
}

/// `{ name, version }` implementation descriptor (clientInfo / serverInfo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// `initialize` result: the negotiated version, server capabilities and
/// an honest instruction line for the model.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: &'static str,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
    pub instructions: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// The skeleton never adds tools mid-session.
    pub list_changed: bool,
}

/// `tools/call` params. `_meta` is accepted anywhere in params (stateless
/// per-request metadata) and ignored.
#[derive(Debug, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub _meta: Option<Value>,
}

/// `tools/list` result. Single page in Phase 1 — `nextCursor` is absent
/// until pagination actually exists; the field is not faked.
#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolDefinition>,
}

/// One tool advertisement: name, human title, description, a JSON Schema
/// (draft 2020-12) `inputSchema`, and honest annotations.
#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub annotations: ToolAnnotations,
}

/// MCP tool annotations (honest hints; auth decisions are never made from
/// them — quality standard §1 / MCP_SERVER_PHASE1.md).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    pub title: String,
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

// ---------------------------------------------------------------------------
// Server: line in → zero or one line out.
// ---------------------------------------------------------------------------

/// Protocol server. Framing stays pure (`handle_line` is string in, option
/// of string out, unit-testable without I/O); tool EXECUTION goes through
/// the process [`crate::runtime::Runtime`] (profile + writer lock) when
/// one was attached by `main`. `Default` (and `new()`) is the bare server:
/// every `tools/call` answers a typed `runtime_unavailable` internal
/// error — the framing tests construct it bare on purpose.
#[derive(Default)]
pub struct Server {
    runtime: Option<std::sync::Arc<crate::runtime::Runtime>>,
}

/// Typed refusal for an input line past the serve-loop size bound. The
/// id is unknowable (the line was never parsed), so it answers with a
/// null id like every other pre-parse refusal.
pub fn line_too_long_error(max_bytes: usize) -> String {
    RpcResponse::failure(
        Value::Null,
        RpcError::invalid_request(format!(
            "request line exceeds the {max_bytes}-byte stdio bound; \
             send one bounded JSON-RPC message per line"
        )),
    )
    .to_wire()
}

/// Typed refusal after the serve loop caught a handler panic: the
/// session survives, the one request that panicked gets an internal
/// error with the panic reason.
pub fn handler_panic_error(reason: &str) -> String {
    RpcResponse::failure(
        Value::Null,
        RpcError::internal(format!(
            "internal panic while handling the request ({reason}); \
             the server keeps serving"
        )),
    )
    .to_wire()
}

impl Server {
    /// Bare server: framing only, no tool execution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Served server: tools run against this runtime.
    pub fn with_runtime(runtime: std::sync::Arc<crate::runtime::Runtime>) -> Self {
        Self {
            runtime: Some(runtime),
        }
    }

    /// One stdin line → an outbound protocol line, or `None` when nothing
    /// may be written (empty line, notification, unknown notification).
    /// Never panics on client input: every malformed input degrades to a
    /// typed error response.
    pub fn handle_line(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(err) => {
                return Some(
                    RpcResponse::failure(
                        Value::Null,
                        RpcError::parse_error(format!("line is not valid JSON: {err}")),
                    )
                    .to_wire(),
                );
            }
        };
        self.handle_message(value)
            .map(|response| response.to_wire())
    }

    fn handle_message(&self, value: Value) -> Option<RpcResponse> {
        if value.is_array() {
            return Some(RpcResponse::failure(
                Value::Null,
                RpcError::invalid_request(
                    "JSON-RPC batching is not supported by MCP 2026-07-28; send one message per line",
                ),
            ));
        }
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => {
                return Some(RpcResponse::failure(
                    Value::Null,
                    RpcError::invalid_request("request must be a JSON object"),
                ));
            }
        };

        // Best-effort id: echoed back verbatim when present, null when not.
        // `id` being present but null is a request with a null id, not a
        // notification (JSON-RPC 2.0 §4.1: notifications omit `id`).
        let has_id = obj.contains_key("id");
        let id = obj.get("id").cloned().unwrap_or(Value::Null);

        let method = match obj.get("method").and_then(Value::as_str) {
            Some(method) => method.to_string(),
            None => {
                return Some(RpcResponse::failure(
                    id,
                    RpcError::invalid_request("missing string `method`"),
                ));
            }
        };

        match obj.get("jsonrpc") {
            // Lenient receive: a missing `jsonrpc` member is tolerated as
            // 2.0 and logged; a WRONG version is a typed refusal.
            None => crate::stderr_log("request without `jsonrpc` member — treating as \"2.0\""),
            Some(version) if version.as_str() != Some("2.0") => {
                return Some(RpcResponse::failure(
                    id,
                    RpcError::invalid_request(format!(
                        "unsupported jsonrpc version {version} (expected \"2.0\")"
                    )),
                ));
            }
            Some(_) => {}
        }

        if !has_id {
            // Notifications never produce a response (JSON-RPC 2.0 §4.1),
            // known or unknown.
            self.handle_notification(&method);
            return None;
        }

        let params = obj.get("params").cloned().filter(|p| !p.is_null());
        Some(self.handle_request(&method, params, id))
    }

    fn handle_notification(&self, method: &str) {
        match method {
            "notifications/initialized" => {
                crate::stderr_log("client completed initialization (notifications/initialized)");
            }
            "notifications/cancelled" => {
                crate::stderr_log(
                    "notifications/cancelled received — the skeleton runs no long operations",
                );
            }
            other => crate::stderr_log(&format!("ignoring unknown notification: {other}")),
        }
    }

    fn handle_request(&self, method: &str, params: Option<Value>, id: Value) -> RpcResponse {
        match method {
            "initialize" => self.initialize(params, id),
            "ping" => RpcResponse::success(id, serde_json::json!({})),
            "tools/list" => RpcResponse::success(id, crate::tools::tools_list_result()),
            "tools/call" => self.tools_call(params, id),
            other => RpcResponse::failure(id, RpcError::method_not_found(other)),
        }
    }

    fn initialize(&self, params: Option<Value>, id: Value) -> RpcResponse {
        let params: InitializeParams = match params {
            Some(raw) => match serde_json::from_value(raw) {
                Ok(params) => params,
                Err(err) => {
                    return RpcResponse::failure(
                        id,
                        RpcError::invalid_params(format!("invalid initialize params: {err}")),
                    );
                }
            },
            None => InitializeParams::default(),
        };

        let answered = negotiate_protocol_version(params.protocol_version.as_deref());
        if params.protocol_version.as_deref() != Some(answered) {
            crate::stderr_log(&format!(
                "client protocolVersion {:?} is not supported; answering ours ({answered})",
                params.protocol_version
            ));
        }
        let client = params.client_info.clone().unwrap_or(Implementation {
            name: "unknown".to_string(),
            version: "0".to_string(),
        });
        crate::stderr_log(&format!(
            "initialize from client {} {}; negotiated protocolVersion {answered}",
            client.name, client.version
        ));
        // Client capabilities are accepted (and logged for handshake
        // diagnostics) but nothing is negotiated against them yet — the
        // server surface is fixed in Phase 1.
        if let Some(caps) = &params.capabilities {
            crate::stderr_log(&format!("client capabilities advertised: {caps}"));
        }

        value_or_internal(
            &InitializeResult {
                protocol_version: answered,
                capabilities: ServerCapabilities {
                    tools: ToolsCapability {
                        list_changed: false,
                    },
                },
                server_info: Implementation {
                    name: SERVER_NAME.to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                instructions: "RimLoc localization server, Phase 1: the full v1 surface is \
                               live — 9 read-only tools plus the controlled-write layer \
                               (rimloc.project.open, rimloc.translation.propose [durable \
                               drafts, never an apply], rimloc.translation.apply, \
                               rimloc.ai_job.start/status/cancel, \
                               rimloc.ai_job.accept_selected [apply ONLY the selected row \
                               identities; rejected is an explicit ledger], \
                               rimloc.export.build [writes ONLY the explicit absolute \
                               out_dir, never the source mod tree]). Sessions are stateless: \
                               a fresh session opens per call, so session_epoch is optional \
                               on reads and REQUIRED on every controlled write; stale \
                               epochs/revisions are typed refusals, never overwrites. While \
                               another process holds the exclusive writer lock, writes and \
                               project-scoped reads refuse with other_writer_active \
                               (-32003); disk diagnostics stay up. rimloc.ai_job.start \
                               launches paid providers ONLY through an existing provider \
                               instance (prov-*): the server never requests, stores or \
                               returns secrets — identifiers only. Transport is stdio only; \
                               no network listener.",
            },
            id,
        )
    }

    fn tools_call(&self, params: Option<Value>, id: Value) -> RpcResponse {
        let raw = params.unwrap_or(Value::Null);
        let params: ToolsCallParams = match serde_json::from_value(raw) {
            Ok(params) => params,
            Err(err) => {
                return RpcResponse::failure(
                    id,
                    RpcError::invalid_params(format!("invalid tools/call params: {err}")),
                );
            }
        };
        let arguments = match params.arguments {
            None | Some(Value::Null) => serde_json::json!({}),
            Some(args) if args.is_object() => args,
            Some(_) => {
                return RpcResponse::failure(
                    id,
                    RpcError::invalid_params("`arguments` must be an object"),
                );
            }
        };
        let Some(runtime) = &self.runtime else {
            return RpcResponse::failure(id, crate::runtime::runtime_unavailable(&params.name));
        };
        match crate::tools::call(&params.name, arguments, runtime) {
            Ok(result) => RpcResponse::success(id, result),
            Err(err) => RpcResponse::failure(id, err),
        }
    }
}

impl RpcResponse {
    /// Serialize to a protocol line (no trailing newline — the writer adds
    /// it). Serialization of these plain shapes cannot fail; the fallback
    /// keeps a runtime path unwrap-free anyway.
    fn to_wire(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|err| {
            format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{{\"code\":-32603,\
                 \"message\":\"response serialization failed: {err}\"}}}}"
            )
        })
    }
}

/// Serialize a result payload into a success response; an (unreachable for
/// these plain shapes) serialization failure degrades to a typed internal
/// error instead of a panic.
fn value_or_internal<T: Serialize>(value: &T, id: Value) -> RpcResponse {
    match serde_json::to_value(value) {
        Ok(result) => RpcResponse::success(id, result),
        Err(err) => RpcResponse::failure(
            id,
            RpcError::internal(format!("result serialization failed: {err}")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;
    use serde_json::json;

    fn parse(line: String) -> Value {
        serde_json::from_str(&line).expect("handler output is valid JSON")
    }

    // -- protocol-version negotiation --------------------------------------

    #[test]
    fn negotiation_answers_a_known_client_version() {
        assert_eq!(negotiate_protocol_version(Some("2026-07-28")), "2026-07-28");
    }

    #[test]
    fn negotiation_answers_our_version_for_unknown_client_version() {
        assert_eq!(
            negotiate_protocol_version(Some("2024-11-05")),
            PROTOCOL_VERSION
        );
        assert_eq!(negotiate_protocol_version(Some("")), PROTOCOL_VERSION);
        assert_eq!(
            negotiate_protocol_version(Some("9999-99-99-draft")),
            PROTOCOL_VERSION
        );
    }

    #[test]
    fn negotiation_answers_our_version_when_client_sends_none() {
        assert_eq!(negotiate_protocol_version(None), PROTOCOL_VERSION);
    }

    #[test]
    fn supported_list_is_the_tested_revision_only() {
        // Quality standard §1: only revisions actually tested with real
        // clients may be advertised.
        assert_eq!(SUPPORTED_PROTOCOL_VERSIONS, &["2026-07-28"]);
    }

    // -- framing -------------------------------------------------------------

    #[test]
    fn framing_valid_ping_gets_empty_result_and_echoed_id() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#)
            .expect("ping must answer");
        let msg = parse(out);
        assert_eq!(msg["jsonrpc"], "2.0");
        assert_eq!(msg["id"], 7);
        assert_eq!(msg["result"], json!({}));
        assert!(msg.get("error").is_none());
    }

    #[test]
    fn framing_valid_ping_tolerates_params_with_stateless_meta() {
        // The testlab client (gabs_mcp.py) attaches `_meta.progressToken` to
        // EVERY request, ping included; stateless handling must accept it.
        let out = Server::new()
            .handle_line(
                r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{"_meta":{"progressToken":1}}}"#,
            )
            .expect("ping with _meta must answer");
        assert_eq!(parse(out)["result"], json!({}));
    }

    #[test]
    fn framing_broken_json_is_parse_error_with_null_id() {
        let out = Server::new()
            .handle_line("this is not json {")
            .expect("broken JSON must get a parse error");
        let msg = parse(out);
        assert_eq!(msg["error"]["code"], codes::PARSE_ERROR);
        assert!(msg["id"].is_null());
        assert!(msg.get("result").is_none());
    }

    #[test]
    fn framing_batching_line_is_a_typed_invalid_request() {
        let out = Server::new()
            .handle_line(
                r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#,
            )
            .expect("a batch line must be refused, not executed");
        let msg = parse(out);
        assert_eq!(msg["error"]["code"], codes::INVALID_REQUEST);
        assert!(
            msg["error"]["message"]
                .as_str()
                .expect("message is a string")
                .contains("batching"),
            "refusal must name batching: {}",
            msg["error"]["message"]
        );
        assert!(msg["id"].is_null());
    }

    #[test]
    fn framing_unknown_method_is_method_not_found() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"2.0","id":9,"method":"resources/list"}"#)
            .expect("unknown method must answer");
        let msg = parse(out);
        assert_eq!(msg["error"]["code"], codes::METHOD_NOT_FOUND);
        assert_eq!(msg["id"], 9);
    }

    #[test]
    fn framing_wrong_jsonrpc_version_is_invalid_request() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"1.0","id":3,"method":"ping"}"#)
            .expect("wrong version must answer");
        assert_eq!(parse(out)["error"]["code"], codes::INVALID_REQUEST);
    }

    #[test]
    fn framing_missing_method_is_invalid_request() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"2.0","id":4}"#)
            .expect("missing method must answer");
        assert_eq!(parse(out)["error"]["code"], codes::INVALID_REQUEST);
    }

    #[test]
    fn framing_non_object_json_is_invalid_request() {
        let out = Server::new()
            .handle_line("42")
            .expect("a bare number must answer");
        assert_eq!(parse(out)["error"]["code"], codes::INVALID_REQUEST);
    }

    #[test]
    fn framing_empty_and_whitespace_lines_produce_nothing() {
        let server = Server::new();
        assert!(server.handle_line("").is_none());
        assert!(server.handle_line("   \t ").is_none());
    }

    #[test]
    fn framing_string_ids_are_echoed_verbatim() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"2.0","id":"abc-1","method":"ping"}"#)
            .expect("string id must answer");
        assert_eq!(parse(out)["id"], "abc-1");
    }

    // -- notifications -------------------------------------------------------

    #[test]
    fn notifications_never_produce_a_response() {
        let server = Server::new();
        assert!(server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none());
        // Unknown notifications are silently ignored (logged to stderr), per
        // JSON-RPC 2.0 §4.1 — no response to a notification, ever.
        assert!(server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/whatever"}"#)
            .is_none());
        // Unknown method WITHOUT id stays a notification: no response.
        assert!(server
            .handle_line(r#"{"jsonrpc":"2.0","method":"tools/unknown"}"#)
            .is_none());
    }

    // -- initialize over the wire ---------------------------------------------

    #[test]
    fn initialize_negotiates_over_the_wire() {
        let server = Server::new();
        let known = parse(
            server
                .handle_line(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
                )
                .expect("initialize must answer"),
        );
        assert_eq!(known["result"]["protocolVersion"], "2026-07-28");
        assert_eq!(known["result"]["serverInfo"]["name"], SERVER_NAME);
        assert_eq!(
            known["result"]["capabilities"]["tools"]["listChanged"], false,
            "the skeleton never adds tools mid-session"
        );

        let unknown = parse(
            server
                .handle_line(
                    r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
                )
                .expect("initialize with unknown version must still answer"),
        );
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOL_VERSION);
    }

    // -- tools/call over the wire ---------------------------------------------

    #[test]
    fn tools_call_with_meta_reaches_the_tool_layer() {
        // `_meta` must not break params parsing: the refusal comes from the
        // tool layer, not from invalid params. This bare server carries no
        // runtime, so the typed runtime_unavailable internal error answers.
        let out = Server::new()
            .handle_line(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"_meta":{"progressToken":5},"name":"rimloc.projects.list","arguments":{}}}"#,
            )
            .expect("tools/call must answer");
        let msg = parse(out);
        assert_eq!(msg["error"]["code"], codes::INTERNAL_ERROR);
        assert_eq!(msg["error"]["data"]["code"], "runtime_unavailable");
        assert_eq!(msg["error"]["data"]["tool"], "rimloc.projects.list");
    }

    #[test]
    fn tools_call_runs_against_an_attached_runtime() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runtime = std::sync::Arc::new(
            crate::runtime::Runtime::build(dir.path().join("data")).expect("runtime"),
        );
        let out = Server::with_runtime(runtime)
            .handle_line(
                r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"rimloc.status","arguments":{}}}"#,
            )
            .expect("tools/call must answer");
        let msg = parse(out);
        assert!(msg.get("error").is_none(), "{msg}");
        assert_eq!(msg["result"]["structuredContent"]["app"], "self");
        // The MCP CallToolResult shape: text summary + structuredContent.
        assert_eq!(msg["result"]["content"][0]["type"], "text");
    }

    #[test]
    fn tools_call_non_object_arguments_is_invalid_params() {
        let out = Server::new()
            .handle_line(
                r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"rimloc.status","arguments":[1,2]}}"#,
            )
            .expect("bad arguments must answer");
        assert_eq!(parse(out)["error"]["code"], codes::INVALID_PARAMS);
    }

    // -- tools/list over the wire ----------------------------------------------

    #[test]
    fn tools_list_over_the_wire_is_a_single_page_of_tools() {
        let out = Server::new()
            .handle_line(r#"{"jsonrpc":"2.0","id":8,"method":"tools/list"}"#)
            .expect("tools/list must answer");
        let msg = parse(out);
        let tools = msg["result"]["tools"]
            .as_array()
            .expect("tools is an array");
        assert!(!tools.is_empty());
        assert!(
            msg["result"].get("nextCursor").is_none(),
            "no fake pagination"
        );
    }
}
