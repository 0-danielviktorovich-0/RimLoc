//! Provider trait and wire types. Providers translate a batch of units and
//! must preserve unit `id`s; the engine validates everything they return.

use crate::LlmError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single string to translate. `id` is engine-assigned and must round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslateUnit {
    pub id: String,
    pub source: String,
    /// Optional translator context: key path (DefInjected/Keyed), def type, field.
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub source_lang: String,
    pub target_lang: String,
    pub units: Vec<TranslateUnit>,
    /// EN → RU terminology that must be applied verbatim when present.
    pub glossary: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateUnitResult {
    pub id: String,
    pub translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResponse {
    pub results: Vec<TranslateUnitResult>,
    /// Provider-reported token usage if available (advisory only).
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub model: String,
    pub endpoint: String,
}

/// A provider turns a batch request into translations. Implementations must be
/// deterministic per request for tests; must never log or embed the API key;
/// must treat request content as opaque data (prompt-injection defense lives
/// in the prompt, providers only forward it).
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn model(&self) -> &str;
    fn info(&self) -> ProviderInfo;
    fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError>;
    /// Cheap reachability/auth probe (never called in tests).
    fn test_connection(&self) -> Result<ProviderInfo, LlmError>;
}

/// Registry presets resolvable from CLI/GUI config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub id: String,
    pub model: String,
    /// Base URL for OpenAI-compatible endpoints; ignored by Anthropic.
    pub base_url: Option<String>,
}

pub fn builtin_presets() -> BTreeMap<String, ProviderPreset> {
    let mut m = BTreeMap::new();
    m.insert(
        "anthropic".into(),
        ProviderPreset {
            id: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            base_url: None,
        },
    );
    m.insert(
        "openai".into(),
        ProviderPreset {
            id: "openai".into(),
            model: "gpt-5".into(),
            base_url: Some("https://api.openai.com/v1".into()),
        },
    );
    m.insert(
        "zai".into(),
        ProviderPreset {
            id: "zai".into(),
            model: "glm-4.7".into(),
            base_url: Some("https://api.z.ai/api/paas/v4".into()),
        },
    );
    m.insert(
        "ollama".into(),
        ProviderPreset {
            id: "ollama".into(),
            model: "qwen3".into(),
            base_url: Some("http://localhost:11434/v1".into()),
        },
    );
    m
}

/// Secret lookup the providers call just before a request.
pub fn resolve_key(
    key_source: &crate::KeySource,
    provider: &str,
) -> Result<Option<String>, LlmError> {
    key_source.resolve(provider)
}
