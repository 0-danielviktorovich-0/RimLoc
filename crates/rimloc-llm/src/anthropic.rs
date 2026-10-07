//! Anthropic Messages API provider (`http` feature). Never called in tests.

use crate::prompt;
use crate::provider::{Provider, ProviderInfo, TranslateRequest, TranslateResponse, Usage};
use crate::{KeySource, LlmError};

pub struct AnthropicProvider {
    pub model: String,
    pub key: KeySource,
    pub max_tokens: u32,
    client: reqwest::blocking::Client,
}

impl AnthropicProvider {
    pub fn new(model: String, key: KeySource) -> Self {
        Self {
            model,
            key,
            max_tokens: 8192,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("http client"),
        }
    }

    fn call(&self, system: &str, user: &str) -> Result<(String, Option<Usage>), LlmError> {
        let Some(key) = crate::provider::resolve_key(&self.key, "anthropic")? else {
            return Err(LlmError::MissingKey("anthropic".into()));
        };
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
        });
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(LlmError::Http)?;
        let status = resp.status();
        if status.as_u16() == 429 {
            let after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(std::time::Duration::from_secs)
                .unwrap_or(std::time::Duration::from_secs(10));
            return Err(LlmError::RateLimited("anthropic".into(), after));
        }
        // 5xx is a provider-side transient: classified before the JSON parse
        // (error bodies here are often HTML from gateways, not JSON).
        if status.is_server_error() {
            let body = resp.text().unwrap_or_default();
            return Err(LlmError::ServerError {
                provider: "anthropic".into(),
                status: status.as_u16(),
                message: crate::summarize_body(&body),
            });
        }
        let json: serde_json::Value = resp.json().map_err(LlmError::Http)?;
        if !status.is_success() {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("unknown provider error")
                .to_string();
            return Err(LlmError::Provider("anthropic".into(), msg));
        }
        let text = json["content"][0]["text"]
            .as_str()
            .ok_or_else(|| LlmError::InvalidResponse("missing content[0].text".into()))?
            .to_string();
        let usage = json["usage"].as_object().map(|u| Usage {
            input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        });
        Ok((text, usage))
    }
}

impl Provider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "anthropic".into(),
            model: self.model.clone(),
            endpoint: "https://api.anthropic.com/v1/messages".into(),
        }
    }
    fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
        let user = prompt::build_user_payload(
            &req.source_lang,
            &req.target_lang,
            &req.units,
            &req.glossary,
        );
        let (text, usage) = self.call(&prompt::system_prompt(&req.target_lang), &user)?;
        let results = crate::openai_compat::parse_results_payload(&text, req)?;
        Ok(TranslateResponse { results, usage })
    }
    fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
        let (text, _) = self.call("Reply with the single word: ok", "{\"probe\": true}")?;
        if text.trim().is_empty() {
            return Err(LlmError::InvalidResponse("empty probe reply".into()));
        }
        Ok(self.info())
    }
}
