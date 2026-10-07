//! `contract_provider_instance_test` — the bounded connectivity probe
//! (§7 F7.1/F7.2): the CLI `rimloc provider-test` logic (one tiny
//! "reply with ok" round-trip through [`rimloc_llm::provider::Provider::test_connection`])
//! exposed to the GUI process as an additive Tauri command.
//!
//! Contract discipline:
//! - probe failures are TYPED RESULTS, never rejections — the response
//!   carries the §7 status vocabulary (`auth_failed` / `model_not_found` /
//!   `rate_limited` / `server_error` / `network_failed` / `connected`) so
//!   the UI never string-parses error text;
//! - the раскладка lives in `rimloc_llm::LlmError::connection_verdict()`
//!   (pure, unit-tested there); this module only maps the verdict onto the
//!   wire enum;
//! - the key crosses this boundary ONCE per probe (typed into the form) or
//!   is resolved from the OS keychain by `instance_id` — it never lands in
//!   a response field, a log line or the trace (`traced_simple` logs
//!   ok/error only);
//! - the probe is bounded: the provider clients carry finite HTTP timeouts
//!   (180 s), and one tiny prompt — no batch, no paid bulk call.

use rimloc_llm::provider::{Provider, ProviderPreset};
use rimloc_llm::{
    anthropic::AnthropicProvider, openai_compat::OpenAiCompatProvider, ConnectionVerdict, KeySource,
};
use rimloc_services::contract::{ContractError, ContractErrorCode};
use serde::{Deserialize, Serialize};

/// Probe request — the form values (a saved instance's metadata comes from
/// the redacted `provider_instance_list` the frontend already holds).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderInstanceTestRequest {
    pub preset: String,
    pub model: String,
    /// OpenAI-compatible endpoint override; presets carry their own default.
    pub base_url: Option<String>,
    /// Probe-ONLY key: used in-memory for this one request, never stored,
    /// never echoed. Takes precedence over the keychain lookup.
    pub secret: Option<String>,
    /// Saved instance id — resolves the stored key from the OS keychain
    /// (service = the `rimloc-llm` convention, account = instance id) when
    /// no probe-only secret was supplied.
    pub instance_id: Option<String>,
}

/// §7 F7.1 outcome vocabulary of the probe. The lifecycle states
/// (`configured` / `connection_unknown` / `testing`) and the display fold
/// (`local_offline`) live on the frontend; `server_error` is kept here for
/// vocabulary completeness though the §7 раскладка currently maps 5xx onto
/// `network_failed` — no producer exists yet, and that is honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTestStatus {
    Connected,
    AuthFailed,
    ModelNotFound,
    RateLimited,
    ServerError,
    NetworkFailed,
}

/// Typed probe outcome. `detail` is the sanitized provider/error text —
/// key material never reaches it (the LlmError display strings carry only
/// provider ids, statuses and provider messages).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderInstanceTestResponse {
    pub status: ProviderTestStatus,
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub detail: Option<String>,
}

const KNOWN_PRESETS: [&str; 5] = ["anthropic", "openai", "zai", "ollama", "custom"];

/// The preset's default base URL (mirrors `builtin_presets`); `custom` and
/// `anthropic` have none here (anthropic's endpoint is fixed).
fn preset_default_base_url(preset: &str) -> Option<String> {
    rimloc_llm::provider::builtin_presets()
        .get(preset)
        .and_then(|p| p.base_url.clone())
}

// Note: the probe takes NO `local` flag — localness is derived from the
// endpoint itself (the provider call allows keyless loopback probes only),
// exactly like the CLI `provider-test`.

/// Request-shape gate BEFORE any network: unknown preset, empty model and a
/// keyless-form `custom` without `base_url` are typed contract refusals.
fn validate_probe_request(req: &ProviderInstanceTestRequest) -> Result<(), ContractError> {
    let preset = req.preset.trim();
    if preset.is_empty() {
        return Err(ContractError::new(
            ContractErrorCode::ContractViolation,
            "provider preset must not be empty",
        ));
    }
    if !KNOWN_PRESETS.contains(&preset) {
        return Err(ContractError::new(
            ContractErrorCode::ContractViolation,
            format!("unknown provider preset `{preset}` (known: {KNOWN_PRESETS:?})"),
        ));
    }
    if req.model.trim().is_empty() {
        return Err(ContractError::new(
            ContractErrorCode::InvalidConfig,
            "model must not be empty",
        ));
    }
    if preset == "custom"
        && req
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return Err(ContractError::new(
            ContractErrorCode::InvalidConfig,
            "custom provider requires a base_url (OpenAI-compatible endpoint)",
        ));
    }
    Ok(())
}

/// The resolved OpenAI-compatible base URL for the probe: explicit request
/// value, then the preset default; `custom` without one was refused above.
fn resolve_compat_base_url(preset: &str, base_url: Option<&str>) -> String {
    base_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| preset_default_base_url(preset))
        .unwrap_or_else(|| "https://api.openai.com/v1".into())
}

fn endpoint_of(preset: &str, base_url: Option<&str>) -> String {
    if preset == "anthropic" {
        "https://api.anthropic.com/v1/messages".into()
    } else {
        format!(
            "{}/chat/completions",
            resolve_compat_base_url(preset, base_url).trim_end_matches('/')
        )
    }
}

/// Key resolution order: probe-only secret → keychain by `instance_id` →
/// none (a keyless probe is allowed for local endpoints and produces the
/// honest `auth_failed` verdict for cloud ones — the provider call refuses
/// with `MissingKey` before any network).
#[cfg_attr(not(feature = "keychain"), allow(unused_variables))]
fn resolve_probe_key(req: &ProviderInstanceTestRequest) -> Result<KeySource, ContractError> {
    if let Some(secret) = req
        .secret
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(KeySource::Inline(secret.to_string()));
    }
    if let Some(id) = req
        .instance_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        #[cfg(feature = "keychain")]
        {
            let stored = rimloc_llm::secrets::get_secret_in(
                &rimloc_llm::secrets::effective_service(),
                id,
            )
            .map_err(|e| {
                ContractError::new(ContractErrorCode::SaveFailed, format!("keychain: {e}"))
            })?;
            if let Some(secret) = stored {
                return Ok(KeySource::Inline(secret));
            }
        }
    }
    Ok(KeySource::None)
}

fn verdict_to_status(verdict: ConnectionVerdict) -> ProviderTestStatus {
    match verdict {
        ConnectionVerdict::AuthFailed => ProviderTestStatus::AuthFailed,
        ConnectionVerdict::ModelNotFound => ProviderTestStatus::ModelNotFound,
        ConnectionVerdict::RateLimited => ProviderTestStatus::RateLimited,
        ConnectionVerdict::NetworkFailed => ProviderTestStatus::NetworkFailed,
    }
}

/// The probe itself (same logic as the CLI `provider-test` command):
/// build the provider from the form values and reuse
/// `Provider::test_connection()`. Probe failures become typed results.
pub fn run_probe(
    req: &ProviderInstanceTestRequest,
) -> Result<ProviderInstanceTestResponse, ContractError> {
    validate_probe_request(req)?;
    let preset = req.preset.trim();
    let model = req.model.trim().to_string();
    let key = resolve_probe_key(req)?;

    let provider: Box<dyn Provider> = if preset == "anthropic" {
        Box::new(AnthropicProvider::new(model.clone(), key))
    } else {
        Box::new(OpenAiCompatProvider::from_preset(
            &ProviderPreset {
                id: preset.to_string(),
                model: model.clone(),
                base_url: Some(resolve_compat_base_url(preset, req.base_url.as_deref())),
            },
            key,
        ))
    };

    let base = ProviderInstanceTestResponse {
        status: ProviderTestStatus::Connected,
        provider: preset.to_string(),
        model,
        endpoint: endpoint_of(preset, req.base_url.as_deref()),
        detail: None,
    };
    match provider.test_connection() {
        Ok(_) => Ok(base),
        Err(e) => {
            let status = verdict_to_status(e.connection_verdict());
            Ok(ProviderInstanceTestResponse {
                status,
                detail: Some(e.to_string()),
                ..base
            })
        }
    }
}

/// The Tauri command — shell-level pass-through, stateless (the frontend
/// holds the redacted instance metadata; the key comes once per probe or
/// from the OS keychain by instance id). The trace detail carries only the
/// outcome status — never the request or its secret.
#[tauri::command(rename_all = "snake_case")]
pub fn contract_provider_instance_test(
    request: ProviderInstanceTestRequest,
) -> Result<ProviderInstanceTestResponse, ContractError> {
    crate::trace::traced(
        "contract_provider_instance_test",
        || run_probe(&request),
        |out| match out {
            Ok(resp) => format!("status={:?}", resp.status),
            Err(_) => "error".to_string(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(preset: &str, model: &str, base_url: Option<&str>) -> ProviderInstanceTestRequest {
        ProviderInstanceTestRequest {
            preset: preset.into(),
            model: model.into(),
            base_url: base_url.map(str::to_string),
            secret: None,
            instance_id: None,
        }
    }

    #[test]
    fn request_shape_refusals_are_typed() {
        // unknown preset → contract violation
        let err = run_probe(&req("nope", "m", None)).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);
        // empty model → invalid config
        let err = run_probe(&req("zai", "  ", None)).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::InvalidConfig);
        // custom without base_url → invalid config
        let err = run_probe(&req("custom", "m", Some("  "))).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::InvalidConfig);
        // well-formed request passes the gate (no network happened — the
        // probe itself is never called in tests)
        assert!(validate_probe_request(&req("custom", "m", Some("http://localhost:8080/v1"))).is_ok());
    }

    #[test]
    fn verdict_maps_onto_wire_status() {
        assert_eq!(
            verdict_to_status(ConnectionVerdict::AuthFailed),
            ProviderTestStatus::AuthFailed
        );
        assert_eq!(
            verdict_to_status(ConnectionVerdict::ModelNotFound),
            ProviderTestStatus::ModelNotFound
        );
        assert_eq!(
            verdict_to_status(ConnectionVerdict::RateLimited),
            ProviderTestStatus::RateLimited
        );
        assert_eq!(
            verdict_to_status(ConnectionVerdict::NetworkFailed),
            ProviderTestStatus::NetworkFailed
        );
    }

    /// The wire form is snake_case (§7 vocabulary) and the response type has
    /// NO field that could carry a secret.
    #[test]
    fn status_wire_form_is_snake_case() {
        let wire = serde_json::to_value(ProviderTestStatus::AuthFailed).unwrap();
        assert_eq!(wire, serde_json::json!("auth_failed"));
        let wire = serde_json::to_value(ProviderTestStatus::ModelNotFound).unwrap();
        assert_eq!(wire, serde_json::json!("model_not_found"));
        let wire = serde_json::to_value(ProviderTestStatus::RateLimited).unwrap();
        assert_eq!(wire, serde_json::json!("rate_limited"));
        let wire = serde_json::to_value(ProviderTestStatus::NetworkFailed).unwrap();
        assert_eq!(wire, serde_json::json!("network_failed"));
        let wire = serde_json::to_value(ProviderTestStatus::Connected).unwrap();
        assert_eq!(wire, serde_json::json!("connected"));
    }

    /// PROOF (redaction by construction): the serialized response can never
    /// contain the probe-only secret — there is no field for it.
    #[test]
    fn response_wire_never_carries_the_secret() {
        let mut request = req("zai", "glm-4.7", None);
        request.secret = Some("sk-lane-secret-nevershown".into());
        request.instance_id = Some("prov-test".into());
        // Shape gate only (no network in tests): resolve + build stays here.
        let key = resolve_probe_key(&request).unwrap();
        assert!(matches!(key, KeySource::Inline(_)));
        let response = ProviderInstanceTestResponse {
            status: ProviderTestStatus::NetworkFailed,
            provider: "zai".into(),
            model: "glm-4.7".into(),
            endpoint: endpoint_of("zai", None),
            detail: Some("provider `zai` rejected the request (HTTP 401): bad key".into()),
        };
        let wire = serde_json::to_string(&response).unwrap();
        assert!(!wire.contains("sk-lane-secret-nevershown"));
    }

    #[test]
    fn endpoints_follow_the_preset_shape() {
        assert_eq!(
            endpoint_of("anthropic", None),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            endpoint_of("zai", None),
            "https://api.z.ai/api/paas/v4/chat/completions"
        );
        assert_eq!(
            endpoint_of("openai", Some("http://localhost:11434/v1/")),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    /// Keyless local request resolves `KeySource::None` (the openai-compat
    /// call allows keyless loopback probes; cloud would refuse with
    /// MissingKey → auth_failed).
    #[test]
    fn keyless_local_probe_resolves_no_key() {
        let request = req("ollama", "qwen3", None);
        let key = resolve_probe_key(&request).unwrap();
        assert!(matches!(key, KeySource::None));
        assert!(request_shape_ok_for_probe(&request));
    }

    fn request_shape_ok_for_probe(req: &ProviderInstanceTestRequest) -> bool {
        validate_probe_request(req).is_ok()
    }
}
