//! LLM translation subsystem for RimLoc.
//!
//! Layering: depends only on `rimloc-core` (+ HTTP behind the `http` feature).
//! Normal automated tests never touch the network — [`mock::MockProvider`]
//! provides deterministic behaviour; real providers are opt-in at runtime.

pub mod checkpoint;
pub mod cost;
pub mod engine;
pub mod glossary;
pub mod mock;
pub mod prompt;
pub mod provider;
pub mod validator;

#[cfg(feature = "http")]
pub mod anthropic;
#[cfg(feature = "http")]
pub mod openai_compat;
#[cfg(feature = "keychain")]
pub mod secrets;

pub use checkpoint::{CheckpointEntry, CheckpointStore};
pub use engine::{EngineOptions, EngineSummary, TranslationEngine, UnitOutcome};
pub use glossary::Glossary;
pub use provider::{
    Provider, ProviderInfo, ProviderPreset, TranslateRequest, TranslateUnit, TranslateUnitResult,
};
pub use validator::PlaceholderIssue;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error(
        "provider auth: no API key for `{0}` (set via keychain service `rimloc-llm` or env var)"
    )]
    MissingKey(String),
    #[error("provider `{0}` rejected the request (rate limited): retry after {1:?}")]
    RateLimited(String, std::time::Duration),
    #[error("provider `{0}` error: {1}")]
    Provider(String, String),
    /// Provider-side 4xx with the HTTP status preserved (§7 F7.2): the
    /// status is what a connection verdict needs — 401/403 mean auth, 404
    /// means the model, 429 rate limiting. Before this variant existed,
    /// 4xx collapsed into the unclassified [`LlmError::Provider`].
    #[error("provider `{provider}` rejected the request (HTTP {status}): {message}")]
    HttpStatus {
        provider: String,
        status: u16,
        message: String,
    },
    /// Provider-side 5xx: transient by nature, so the engine backs off
    /// exponentially before retrying (client 4xx stays on `Provider`).
    #[error("provider `{provider}` server error (HTTP {status}): {message}")]
    ServerError {
        provider: String,
        status: u16,
        message: String,
    },
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
    #[error("checkpoint io: {0}")]
    CheckpointIo(String),
    #[error("keychain: {0}")]
    #[cfg(feature = "keychain")]
    Keychain(String),
    #[error("http client: {0}")]
    #[cfg(feature = "http")]
    Http(#[from] reqwest::Error),
}

/// Coarse probe verdict (§7 F7.2 раскладка) — the connection-class an
/// [`LlmError`] falls into. Pure classification over the error value, so it
/// is unit-testable without any network. The success case never becomes a
/// verdict: a successful `test_connection()` simply means connected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionVerdict {
    /// 401/403, or no key at all — the credential is the problem.
    AuthFailed,
    /// 404, or a 4xx whose body names the model — the endpoint exists, the
    /// model does not.
    ModelNotFound,
    /// 429 — too many requests; retry later.
    RateLimited,
    /// 5xx, timeouts, DNS/connect failures, unexpected replies — everything
    /// that is neither credential, model, nor quota.
    NetworkFailed,
}

impl LlmError {
    /// The раскладка (owner spec §7): 401/403 → auth_failed; 404/model →
    /// model_not_found; 429 → rate_limited; 5xx/timeout/network →
    /// network_failed. `MissingKey` is auth (a probe cannot authenticate
    /// without a key); legacy [`LlmError::Provider`] and local keychain
    /// failures fall to [`ConnectionVerdict::NetworkFailed`].
    pub fn connection_verdict(&self) -> ConnectionVerdict {
        match self {
            LlmError::HttpStatus {
                status, message, ..
            } => match status {
                401 | 403 => ConnectionVerdict::AuthFailed,
                429 => ConnectionVerdict::RateLimited,
                404 => ConnectionVerdict::ModelNotFound,
                _ => {
                    if message.to_ascii_lowercase().contains("model") {
                        ConnectionVerdict::ModelNotFound
                    } else {
                        ConnectionVerdict::NetworkFailed
                    }
                }
            },
            LlmError::RateLimited(..) => ConnectionVerdict::RateLimited,
            LlmError::MissingKey(_) => ConnectionVerdict::AuthFailed,
            // 5xx, reqwest timeout/connect/DNS errors, unexpected payloads
            // and keychain read failures are all "the path to the provider
            // is broken or unanswerable" — never an auth/model verdict.
            LlmError::ServerError { .. }
            | LlmError::Http(_)
            | LlmError::InvalidResponse(_)
            | LlmError::Provider(..)
            | LlmError::CheckpointIo(_) => ConnectionVerdict::NetworkFailed,
            #[cfg(feature = "keychain")]
            LlmError::Keychain(_) => ConnectionVerdict::NetworkFailed,
        }
    }
}

/// Where an API key comes from. Tests and offline runs use [`SecretStore::None`].
#[derive(Debug, Clone, Default)]
pub enum KeySource {
    /// Keychain via the `keychain` feature (macOS Keychain / Secret Service / Win Credential Mgr),
    /// service = `rimloc-llm`, account = provider id; falls back to env var.
    #[default]
    Auto,
    /// Environment variable holding the key (e.g. `RIMLOC_ANTHROPIC_API_KEY`).
    Env(String),
    /// Plain value — discouraged, but needed for tests of the provider plumbing
    /// that do not hit the network.
    Inline(String),
    /// Never resolves a key (offline / mock usage).
    None,
}

impl KeySource {
    pub fn resolve(&self, provider: &str) -> Result<Option<String>, LlmError> {
        match self {
            KeySource::None => Ok(None),
            KeySource::Inline(v) => Ok(Some(v.clone())),
            KeySource::Env(name) => Ok(std::env::var(name).ok().filter(|v| !v.is_empty())),
            KeySource::Auto => {
                if let Ok(v) = std::env::var(env_key(provider)) {
                    if !v.is_empty() {
                        return Ok(Some(v));
                    }
                }
                #[cfg(feature = "keychain")]
                {
                    let entry = keyring::Entry::new("rimloc-llm", provider)
                        .map_err(|e| LlmError::Provider(provider.into(), e.to_string()))?;
                    match entry.get_password() {
                        Ok(v) => Ok(Some(v)),
                        Err(keyring::Error::NoEntry) => Ok(None),
                        Err(e) => Err(LlmError::Provider(provider.into(), e.to_string())),
                    }
                }
                #[cfg(not(feature = "keychain"))]
                Ok(None)
            }
        }
    }
}

/// Canonical env var name for a provider id (`anthropic` → `RIMLOC_ANTHROPIC_API_KEY`).
pub fn env_key(provider: &str) -> String {
    format!(
        "RIMLOC_{}_API_KEY",
        provider.to_uppercase().replace(['-', ' '], "_")
    )
}

/// Short, single-line summary of an HTTP error body for error reports.
/// Error bodies can be HTML from proxies or arbitrarily large; only the
/// first line, truncated, ever reaches an [`LlmError`].
#[cfg(feature = "http")]
pub(crate) fn summarize_body(body: &str) -> String {
    let first_line = body.lines().map(str::trim).find(|l| !l.is_empty());
    let line = first_line.unwrap_or("(empty body)");
    const MAX: usize = 200;
    if line.chars().count() <= MAX {
        line.to_string()
    } else {
        let cut: String = line.chars().take(MAX).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "http")]
    #[test]
    fn summarize_body_takes_first_line_and_truncates() {
        assert_eq!(summarize_body(""), "(empty body)");
        assert_eq!(summarize_body("\n\n  \n"), "(empty body)");
        assert_eq!(
            summarize_body("  upstream timeout  \nsecond line"),
            "upstream timeout"
        );
        let long = "x".repeat(500);
        let s = summarize_body(&long);
        assert_eq!(s.chars().count(), 201); // 200 + ellipsis
        assert!(s.ends_with('…'));
    }

    /// §7 F7.2 раскладка, pure — no network: 401/403 auth, 404/model
    /// model_not_found, 429 rate_limited, 5xx/timeout/network network_failed.
    #[test]
    fn connection_verdict_maps_http_statuses() {
        let status = |code: u16, message: &str| LlmError::HttpStatus {
            provider: "probe".into(),
            status: code,
            message: message.into(),
        };
        assert_eq!(
            status(401, "invalid x-api-key").connection_verdict(),
            ConnectionVerdict::AuthFailed
        );
        assert_eq!(
            status(403, "permission denied").connection_verdict(),
            ConnectionVerdict::AuthFailed
        );
        assert_eq!(
            status(404, "model not found: gpt-nope").connection_verdict(),
            ConnectionVerdict::ModelNotFound
        );
        // "404/модель": a non-404 4xx whose body names the model is still a
        // model verdict (several gateways answer 400 there).
        assert_eq!(
            status(400, "The model `qwen3` does not exist").connection_verdict(),
            ConnectionVerdict::ModelNotFound
        );
        assert_eq!(
            status(429, "slow down").connection_verdict(),
            ConnectionVerdict::RateLimited
        );
        assert_eq!(
            status(400, "max_tokens is too large").connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
        assert_eq!(
            status(422, "invalid request payload").connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
    }

    #[test]
    fn connection_verdict_maps_error_kinds() {
        assert_eq!(
            LlmError::RateLimited("p".into(), std::time::Duration::from_secs(1))
                .connection_verdict(),
            ConnectionVerdict::RateLimited
        );
        assert_eq!(
            LlmError::MissingKey("p".into()).connection_verdict(),
            ConnectionVerdict::AuthFailed
        );
        assert_eq!(
            LlmError::ServerError {
                provider: "p".into(),
                status: 502,
                message: "bad gateway".into(),
            }
            .connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
        assert_eq!(
            LlmError::Provider("p".into(), "legacy".into()).connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
        assert_eq!(
            LlmError::InvalidResponse("bad json".into()).connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
        assert_eq!(
            LlmError::CheckpointIo("io".into()).connection_verdict(),
            ConnectionVerdict::NetworkFailed
        );
    }

    /// HttpStatus Display carries provider + status + message (the CLI/GUI
    /// render this string as the probe detail) and never panics.
    #[test]
    fn http_status_display_names_provider_and_status() {
        let e = LlmError::HttpStatus {
            provider: "zai".into(),
            status: 401,
            message: "bad key".into(),
        };
        let s = e.to_string();
        assert!(s.contains("zai"), "{s}");
        assert!(s.contains("401"), "{s}");
        assert!(s.contains("bad key"), "{s}");
    }
}
