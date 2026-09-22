//! Deterministic provider for tests and offline dry runs. Never touches the
//! network. Behaviour: prefixes each source with `target_lang:`, echoing the
//! placeholder structure; configurable failure modes let the engine's retry
//! and checkpoint paths be exercised deterministically.

use crate::provider::{
    Provider, ProviderInfo, TranslateRequest, TranslateResponse, TranslateUnitResult,
};
use crate::LlmError;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct MockProvider {
    /// Fail the first N batch calls with a rate-limit error before succeeding.
    pub fail_first_calls: usize,
    /// Drop these unit ids from responses (engine must treat as failures).
    pub omit_ids: Vec<String>,
    pub calls: AtomicUsize,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Provider for MockProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-1"
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "mock".into(),
            model: "mock-1".into(),
            endpoint: "internal://mock".into(),
        }
    }

    fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_first_calls {
            return Err(LlmError::RateLimited(
                self.id().into(),
                std::time::Duration::from_millis(1),
            ));
        }
        let results = req
            .units
            .iter()
            .filter(|u| !self.omit_ids.contains(&u.id))
            .map(|u| TranslateUnitResult {
                id: u.id.clone(),
                translation: format!("{}: {}", req.target_lang, u.source),
            })
            .collect();
        Ok(TranslateResponse {
            results,
            usage: Some(crate::provider::Usage {
                input_tokens: 1,
                output_tokens: 1,
            }),
        })
    }

    fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
        Ok(self.info())
    }
}
