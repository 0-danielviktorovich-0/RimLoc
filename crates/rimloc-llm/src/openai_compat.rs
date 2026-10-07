//! OpenAI-compatible chat/completions provider (`http` feature).
//! One implementation covers OpenAI, Z.AI and Ollama via `base_url`.

use crate::prompt;
use crate::provider::{
    Provider, ProviderInfo, TranslateRequest, TranslateResponse, TranslateUnitResult, Usage,
};
use crate::{KeySource, LlmError};

pub struct OpenAiCompatProvider {
    pub id: String,
    pub model: String,
    pub base_url: String,
    pub key: KeySource,
    client: reqwest::blocking::Client,
}

/// Ask an OpenAI-compatible server for its first installed model id
/// (used so `--provider ollama` — and any OpenAI-compatible preset — works
/// without knowing model names). Bounded and best-effort: an unreachable
/// endpoint or an unexpected payload just yields `None`.
pub fn first_installed_model(base_url: &str, key: Option<&str>) -> Option<String> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let mut rb = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?
        .get(url);
    // Some hosted endpoints (e.g. api.openai.com/v1/models) require auth
    // even for listing; pass the resolved key when there is one. It is only
    // sent as a bearer header — never logged.
    if let Some(k) = key {
        rb = rb.bearer_auth(k);
    }
    let v: serde_json::Value = rb.send().ok()?.json().ok()?;
    v["data"]
        .as_array()?
        .iter()
        .filter_map(|m| m["id"].as_str())
        .next()
        .map(str::to_string)
}

impl OpenAiCompatProvider {
    pub fn from_preset(preset: &crate::provider::ProviderPreset, key: KeySource) -> Self {
        Self {
            id: preset.id.clone(),
            model: preset.model.clone(),
            base_url: preset
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            key,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("http client"),
        }
    }

    fn call(&self, system: &str, user: &str) -> Result<(String, Option<Usage>), LlmError> {
        let key = crate::provider::resolve_key(&self.key, &self.id)?;
        // Local inference servers (Ollama/LM Studio) need no API key. Only a
        // loopback endpoint may run keyless; anything remote requires one.
        let is_local = self.base_url.contains("localhost") || self.base_url.contains("127.0.0.1");
        if key.is_none() && !is_local {
            return Err(LlmError::MissingKey(self.id.clone()));
        }
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "temperature": 0.2,
        });
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut rb = self.client.post(url).json(&body);
        if let Some(k) = key {
            rb = rb.bearer_auth(k);
        }
        let resp = rb.send().map_err(LlmError::Http)?;
        let status = resp.status();
        if status.as_u16() == 429 {
            let after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(std::time::Duration::from_secs)
                .unwrap_or(std::time::Duration::from_secs(10));
            return Err(LlmError::RateLimited(self.id.clone(), after));
        }
        // 5xx is a provider-side transient: classified before the JSON parse
        // (error bodies here are often HTML from gateways, not JSON).
        if status.is_server_error() {
            let body = resp.text().unwrap_or_default();
            return Err(LlmError::ServerError {
                provider: self.id.clone(),
                status: status.as_u16(),
                message: crate::summarize_body(&body),
            });
        }
        let json: serde_json::Value = resp.json().map_err(LlmError::Http)?;
        if !status.is_success() {
            // Remaining non-success here is 4xx (429/5xx were peeled above):
            // the status is preserved so a probe can tell auth (401/403)
            // from a missing model (404) — §7 F7.2.
            let msg = json["error"]["message"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("unknown provider error")
                .to_string();
            return Err(LlmError::HttpStatus {
                provider: self.id.clone(),
                status: status.as_u16(),
                message: msg,
            });
        }
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| LlmError::InvalidResponse("missing choices[0].message.content".into()))?
            .to_string();
        let usage = json["usage"].as_object().map(|u| Usage {
            input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            output_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        });
        Ok((text, usage))
    }
}

impl Provider for OpenAiCompatProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id.clone(),
            model: self.model.clone(),
            endpoint: format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
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
        let results = parse_results_payload(&text, req)?;
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

/// Parse the model's results payload: strict JSON object
/// `{"results":[{"id","translation"}...]}`, tolerating markdown fences.
pub(crate) fn parse_results_payload(
    text: &str,
    req: &TranslateRequest,
) -> Result<Vec<TranslateUnitResult>, LlmError> {
    let stripped = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```");
    let stripped = stripped.trim_end_matches("```").trim();
    let value: serde_json::Value = serde_json::from_str(stripped)
        .map_err(|e| LlmError::InvalidResponse(format!("results payload: {e}")))?;
    let arr = value["results"]
        .as_array()
        .ok_or_else(|| LlmError::InvalidResponse("missing results array".into()))?;
    let valid_ids: std::collections::BTreeSet<&str> =
        req.units.iter().map(|u| u.id.as_str()).collect();
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let id = item["id"]
            .as_str()
            .ok_or_else(|| LlmError::InvalidResponse("result without id".into()))?;
        if !valid_ids.contains(id) {
            return Err(LlmError::InvalidResponse(format!(
                "unknown id `{id}` in response"
            )));
        }
        out.push(TranslateUnitResult {
            id: id.to_string(),
            translation: item["translation"]
                .as_str()
                .ok_or_else(|| {
                    LlmError::InvalidResponse(format!("result `{id}` missing translation"))
                })?
                .to_string(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::TranslateUnit;

    #[test]
    fn parses_fenced_json_and_rejects_unknown_ids() {
        let req = TranslateRequest {
            source_lang: "en".into(),
            target_lang: "ru".into(),
            units: vec![TranslateUnit {
                id: "a".into(),
                source: "Hi".into(),
                context: None,
            }],
            glossary: Default::default(),
        };
        let fenced = "```json\n{\"results\":[{\"id\":\"a\",\"translation\":\"Привет\"}]}\n```";
        let r = parse_results_payload(fenced, &req).unwrap();
        assert_eq!(r[0].translation, "Привет");

        let injected = "{\"results\":[{\"id\":\"EVIL\",\"translation\":\"x\"}]}";
        assert!(parse_results_payload(injected, &req).is_err());
    }
}
