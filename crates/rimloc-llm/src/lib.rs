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
