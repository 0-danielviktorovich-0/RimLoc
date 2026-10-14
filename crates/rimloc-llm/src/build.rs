//! Shared provider construction for GUI jobs and the CLI translate command.
//! One factory — no second engine. Keys resolve via [`KeySource`] at call
//! time (keychain first, env fallback) and never leak into logs.

use crate::provider::Provider;
#[cfg(feature = "http")]
use crate::KeySource;
use crate::{mock, LlmError};

#[cfg(feature = "http")]
use crate::provider::{builtin_presets, ProviderPreset};
#[cfg(feature = "http")]
use crate::{anthropic, openai_compat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProviderBuildArgs<'a> {
    pub provider: &'a str,
    pub model: Option<&'a str>,
    pub base_url: Option<&'a str>,
    /// `Some(env_name)` overrides the default key resolution with a named
    /// environment variable (CLI compatibility).
    pub key_env: Option<&'a str>,
}

pub fn build_provider(args: ProviderBuildArgs<'_>) -> Result<Box<dyn Provider>, LlmError> {
    #[cfg(feature = "http")]
    let key_source = match args.key_env {
        Some(name) => KeySource::Env(name.to_string()),
        None => KeySource::Auto,
    };
    #[cfg(feature = "http")]
    let presets = builtin_presets();
    let _ = args.key_env;
    #[cfg(not(feature = "http"))]
    if args.provider != "mock" {
        return Err(LlmError::Provider(
            args.provider.to_string(),
            "provider requires the `http` feature".to_string(),
        ));
    }
    #[allow(unused_mut)]
    let mut p: Box<dyn Provider> = match args.provider {
        "mock" => Box::new(mock::MockProvider::new()),
        #[cfg(feature = "http")]
        "anthropic" => {
            let model: String = match args.model {
                Some(m) => m.to_string(),
                None => presets["anthropic"].model.clone(),
            };
            Box::new(anthropic::AnthropicProvider::new(model, key_source))
        }
        #[cfg(feature = "http")]
        id @ ("openai" | "zai" | "ollama") => {
            let mut preset: ProviderPreset = presets[id].clone();
            if let Some(u) = args.base_url {
                preset.base_url = Some(u.to_string());
            }
            match args.model {
                Some(m) => preset.model = m.to_string(),
                None => {
                    if let Some(u) = preset.base_url.clone() {
                        // No explicit model: ask the server (bounded probe).
                        // The resolved key is passed so auth-gated /models
                        // endpoints work.
                        let key = crate::provider::resolve_key(&key_source, id).unwrap_or(None);
                        if let Some(detected) =
                            openai_compat::first_installed_model(&u, key.as_deref())
                        {
                            preset.model = detected;
                        }
                    }
                }
            }
            Box::new(openai_compat::OpenAiCompatProvider::from_preset(
                &preset, key_source,
            ))
        }
        other => {
            return Err(LlmError::Provider(
                other.to_string(),
                format!(
                    "unknown provider `{other}` (expected: mock, anthropic, openai, zai, ollama)"
                ),
            ))
        }
    };
    Ok(p)
}
