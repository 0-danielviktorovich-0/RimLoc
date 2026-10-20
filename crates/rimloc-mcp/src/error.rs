//! Typed JSON-RPC 2.0 errors for the RimLoc MCP server (Phase 1 skeleton).
//!
//! Reserved codes follow JSON-RPC 2.0 §5.1 (-32700..-32603); RimLoc-defined
//! refusals live in the reserved-for-implementations range -32000..-32099.
//! `data` is omitted from the wire when absent — the protocol channel never
//! carries null-valued decoration. Machine-readable string codes in `data`
//! mirror the services contract convention `{ code, message, details? }`
//! (see `rimloc_services::contract`), but the MCP layer never invents
//! contract codes: they map onto JSON-RPC codes declared here.

use serde::Serialize;
use serde_json::Value;

/// JSON-RPC 2.0 reserved codes plus the RimLoc server-defined code.
pub mod codes {
    /// Invalid JSON was received by the server (JSON-RPC 2.0 §5.1).
    pub const PARSE_ERROR: i64 = -32700;
    /// Valid JSON, but not a valid JSON-RPC 2.0 message (wrong shape,
    /// batching, unsupported `jsonrpc` version).
    pub const INVALID_REQUEST: i64 = -32600;
    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Invalid method parameter(s) — including an unknown tool name.
    pub const INVALID_PARAMS: i64 = -32602;
    /// Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i64 = -32603;

    /// A declared tool exists but this Phase 1 skeleton does not execute it
    /// yet. Server-defined range (-32000..-32099) per JSON-RPC 2.0 §5.1.
    pub const NOT_IMPLEMENTED: i64 = -32001;
    /// The services contract refused the operation (stale epoch, unknown
    /// project, guard denial, …). The STABLE snake_case contract code rides
    /// in `data.code` — the MCP layer never invents contract codes, it maps
    /// them (see `runtime::contract_error`).
    pub const CONTRACT_ERROR: i64 = -32002;
    /// Another process holds the exclusive writer lock on the managed root
    /// (`data.owner_pid` names it, when readable). Write tools and
    /// project-scoped reads refuse; lock-free disk diagnostics stay up.
    pub const OTHER_WRITER: i64 = -32003;
}

/// A JSON-RPC error object (`error` member of a response).
#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Attach structured `data` (machine-readable `code` string, tool name,
    /// …). Builder-style; never printed to stdout by the caller — it rides
    /// the JSON-RPC error object only.
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(codes::PARSE_ERROR, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(codes::INVALID_REQUEST, message)
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            codes::METHOD_NOT_FOUND,
            format!("method not found: {method}"),
        )
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(codes::INVALID_PARAMS, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(codes::INTERNAL_ERROR, message)
    }

    /// Honest refusal of a declared-but-unwired tool: the numeric JSON-RPC
    /// code plus a stable snake_case string code in `data`, so clients can
    /// distinguish "not implemented yet" from every other failure.
    pub fn not_implemented(tool: &str) -> Self {
        Self::new(
            codes::NOT_IMPLEMENTED,
            format!("tool `{tool}` is declared but not implemented in the Phase 1 skeleton"),
        )
        .with_data(serde_json::json!({ "code": "not_implemented", "tool": tool }))
    }

    #[cfg(test)]
    pub fn code_is_in_server_range(code: i64) -> bool {
        (-32099..=-32000).contains(&code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;

    #[test]
    fn reserved_code_values_match_json_rpc_2_0() {
        assert_eq!(codes::PARSE_ERROR, -32700);
        assert_eq!(codes::INVALID_REQUEST, -32600);
        assert_eq!(codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(codes::INVALID_PARAMS, -32602);
        assert_eq!(codes::INTERNAL_ERROR, -32603);
        // Server-defined code stays inside the reserved -32000..-32099 range.
        assert!(
            (-32099..=-32000).contains(&codes::NOT_IMPLEMENTED),
            "NOT_IMPLEMENTED must live in the JSON-RPC server-defined range"
        );
        for code in [codes::CONTRACT_ERROR, codes::OTHER_WRITER] {
            assert!(
                (-32099..=-32000).contains(&code),
                "server code {code} must live in the reserved range"
            );
        }
        // All server codes stay distinct — clients switch on them.
        let a = codes::NOT_IMPLEMENTED;
        let b = codes::CONTRACT_ERROR;
        let c = codes::OTHER_WRITER;
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn not_implemented_error_carries_typed_data() {
        let err = RpcError::not_implemented("rimloc.status");
        assert_eq!(err.code, codes::NOT_IMPLEMENTED);
        let data = err.data.expect("not_implemented must carry data");
        assert_eq!(data["code"], "not_implemented");
        assert_eq!(data["tool"], "rimloc.status");
        assert!(err.message.contains("rimloc.status"));
    }

    #[test]
    fn data_is_omitted_from_the_wire_when_absent() {
        let err = RpcError::internal("boom");
        let v = serde_json::to_value(&err).expect("plain error serializes");
        assert!(v.get("data").is_none(), "no null data on the wire: {v}");
        assert_eq!(v["code"], -32603);
        assert_eq!(v["message"], "boom");
    }
}
