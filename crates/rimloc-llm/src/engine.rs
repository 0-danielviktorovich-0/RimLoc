//! TranslationEngine: batches, retries, checkpoint/resume, output validation.
//! The engine never talks to the network itself — that is the Provider's job,
//! which keeps the whole pipeline testable with [`crate::mock::MockProvider`].

use crate::checkpoint::{CheckpointEntry, CheckpointStore};
use crate::glossary::Glossary;
use crate::provider::{Provider, TranslateRequest, TranslateUnit};
use crate::validator::check_pair;
use crate::LlmError;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub source_lang: String,
    pub target_lang: String,
    /// Approximate character budget per provider call.
    pub batch_char_budget: usize,
    pub max_retries: u32,
    /// Strict mode rejects translations whose placeholder set differs from source.
    pub strict_placeholders: bool,
    pub checkpoint_path: Option<std::path::PathBuf>,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            source_lang: "en".into(),
            target_lang: "ru".into(),
            batch_char_budget: 6000,
            max_retries: 3,
            strict_placeholders: true,
            checkpoint_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitOutcome {
    pub id: String,
    pub status: String, // translated | skipped-done | failed
    pub translation: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct EngineSummary {
    pub total: usize,
    pub translated: usize,
    pub skipped_done: usize,
    pub failed: usize,
    pub batches: usize,
    pub retries: usize,
}

pub struct TranslationEngine<'a> {
    provider: &'a dyn Provider,
    opts: EngineOptions,
    glossary: Glossary,
}

impl<'a> TranslationEngine<'a> {
    pub fn new(provider: &'a dyn Provider, opts: EngineOptions, glossary: Glossary) -> Self {
        Self {
            provider,
            opts,
            glossary,
        }
    }

    /// Translate all units; resume from checkpoint when configured.
    /// `on_unit` receives every accepted translation as it lands.
    pub fn translate(
        &self,
        units: &[TranslateUnit],
        mut on_unit: impl FnMut(&UnitOutcome),
    ) -> Result<EngineSummary, LlmError> {
        let mut summary = EngineSummary {
            total: units.len(),
            ..Default::default()
        };
        let glossary = self.glossary.merged();

        let checkpoint = match &self.opts.checkpoint_path {
            Some(p) => Some(CheckpointStore::open(p)?),
            None => None,
        };
        let mut done: BTreeMap<String, String> = BTreeMap::new();
        if let Some(cp) = &checkpoint {
            done = cp.load_completed()?;
        }

        let mut pending: Vec<&TranslateUnit> = units
            .iter()
            .filter(|u| match done.get(&u.id) {
                Some(t) => {
                    let outcome = UnitOutcome {
                        id: u.id.clone(),
                        status: "skipped-done".into(),
                        translation: Some(t.clone()),
                        error: None,
                    };
                    on_unit(&outcome);
                    summary.skipped_done += 1;
                    false
                }
                None => true,
            })
            .collect();

        for batch in batch_by_budget(&mut pending, self.opts.batch_char_budget) {
            summary.batches += 1;
            let req = TranslateRequest {
                source_lang: self.opts.source_lang.clone(),
                target_lang: self.opts.target_lang.clone(),
                units: batch.to_vec(),
                glossary: glossary.clone(),
            };

            let mut attempt = 0u32;
            let response = loop {
                match self.provider.translate_batch(&req) {
                    Ok(r) => break Ok(r),
                    Err(LlmError::RateLimited(provider, retry_after)) => {
                        attempt += 1;
                        summary.retries += 1;
                        if attempt > self.opts.max_retries {
                            break Err(LlmError::RateLimited(provider, retry_after));
                        }
                        std::thread::sleep(retry_after.min(Duration::from_secs(5)));
                    }
                    Err(e) => {
                        attempt += 1;
                        summary.retries += 1;
                        if attempt > self.opts.max_retries {
                            break Err(e);
                        }
                    }
                }
            };

            match response {
                Ok(resp) => {
                    let by_id: BTreeMap<String, String> = resp
                        .results
                        .into_iter()
                        .map(|r| (r.id, r.translation))
                        .collect();
                    for unit in batch {
                        let outcome = match by_id.get(&unit.id) {
                            Some(tr) => {
                                let issues = check_pair(&unit.id, &unit.source, tr);
                                if self.opts.strict_placeholders && !issues.is_empty() {
                                    UnitOutcome {
                                        id: unit.id.clone(),
                                        status: "failed".into(),
                                        translation: None,
                                        error: Some(
                                            issues
                                                .iter()
                                                .map(|i| i.message.clone())
                                                .collect::<Vec<_>>()
                                                .join("; "),
                                        ),
                                    }
                                } else {
                                    if let Some(cp) = &checkpoint {
                                        cp.record(&CheckpointEntry {
                                            id: unit.id.clone(),
                                            translation: tr.clone(),
                                        })?;
                                    }
                                    done.insert(unit.id.clone(), tr.clone());
                                    UnitOutcome {
                                        id: unit.id.clone(),
                                        status: "translated".into(),
                                        translation: Some(tr.clone()),
                                        error: None,
                                    }
                                }
                            }
                            None => UnitOutcome {
                                id: unit.id.clone(),
                                status: "failed".into(),
                                translation: None,
                                error: Some("provider omitted this id".into()),
                            },
                        };
                        match outcome.status.as_str() {
                            "translated" | "skipped-done" => summary.translated += 0, // counted below
                            _ => summary.failed += 1,
                        }
                        if outcome.status == "translated" {
                            summary.translated += 1;
                        }
                        on_unit(&outcome);
                    }
                }
                Err(e) => {
                    for unit in batch {
                        summary.failed += 1;
                        on_unit(&UnitOutcome {
                            id: unit.id.clone(),
                            status: "failed".into(),
                            translation: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        Ok(summary)
    }
}

/// Split units into batches under an approximate character budget
/// (deterministic: input order preserved).
fn batch_by_budget(
    units: &mut Vec<&TranslateUnit>,
    budget: usize,
) -> Vec<Vec<TranslateUnit>> {
    let mut batches = Vec::new();
    let budget = budget.max(200);
    while !units.is_empty() {
        let mut size = 0usize;
        let mut take = 0usize;
        for u in units.iter() {
            let w = u.source.len() + u.context.as_deref().map_or(0, str::len) + 16;
            if size + w > budget && take > 0 {
                break;
            }
            size += w;
            take += 1;
        }
        let batch: Vec<TranslateUnit> = units.drain(..take).map(|u| (*u).clone()).collect();
        batches.push(batch);
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockProvider;
    use crate::provider::{Provider, ProviderInfo, TranslateResponse, TranslateUnitResult};

    fn unit(id: &str, source: &str) -> TranslateUnit {
        TranslateUnit {
            id: id.into(),
            source: source.into(),
            context: None,
        }
    }

    #[test]
    fn happy_path_translates_all_units() {
        let p = MockProvider::new();
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let units = vec![unit("a", "Hello"), unit("b", "World")];
        let summary = engine.translate(&units, |_| {}).unwrap();
        assert_eq!(summary.translated, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.skipped_done, 0);
    }

    #[test]
    fn rate_limit_is_retried_then_succeeds() {
        let p = MockProvider {
            fail_first_calls: 2,
            ..Default::default()
        };
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let summary = engine.translate(&[unit("a", "Hello")], |_| {}).unwrap();
        assert_eq!(summary.translated, 1);
        assert_eq!(summary.retries, 2);
    }

    #[test]
    fn checkpoint_resumes_across_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let cp = tmp.path().join("cp.jsonl");
        let p = MockProvider::new();

        let opts = EngineOptions {
            checkpoint_path: Some(cp.clone()),
            ..Default::default()
        };
        let engine = TranslationEngine::new(&p, opts.clone(), Glossary::default());
        let units = vec![unit("a", "Hello"), unit("b", "World")];
        engine.translate(&units, |_| {}).unwrap();

        // Second run: everything is already done.
        let engine2 = TranslationEngine::new(&p, opts, Glossary::default());
        let mut done_seen = 0;
        let summary = engine2
            .translate(&units, |o| {
                if o.status == "skipped-done" {
                    done_seen += 1;
                }
            })
            .unwrap();
        assert_eq!(done_seen, 2);
        assert_eq!(summary.skipped_done, 2);
        assert_eq!(summary.translated, 0);
    }

    struct OmittingProvider {
        omit: &'static str,
    }
    impl Provider for OmittingProvider {
        fn id(&self) -> &str {
            "omit"
        }
        fn model(&self) -> &str {
            "omit-1"
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: self.id().into(),
                model: self.model().into(),
                endpoint: "x".into(),
            }
        }
        fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
            Ok(TranslateResponse {
                results: req
                    .units
                    .iter()
                    .filter(|u| u.id != self.omit)
                    .map(|u| TranslateUnitResult {
                        id: u.id.clone(),
                        translation: format!("ru: {}", u.source),
                    })
                    .collect(),
                usage: None,
            })
        }
        fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
            Ok(self.info())
        }
    }

    #[test]
    fn omitted_ids_become_failures_not_hallucinations() {
        let p = OmittingProvider { omit: "b" };
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let units = vec![unit("a", "Hello"), unit("b", "World")];
        let summary = engine.translate(&units, |_| {}).unwrap();
        assert_eq!(summary.translated, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn placeholder_mismatch_is_rejected_in_strict_mode() {
        struct BrokenProvider;
        impl Provider for BrokenProvider {
            fn id(&self) -> &str {
                "broken"
            }
            fn model(&self) -> &str {
                "b"
            }
            fn info(&self) -> ProviderInfo {
                ProviderInfo {
                    id: self.id().into(),
                    model: self.model().into(),
                    endpoint: "x".into(),
                }
            }
            fn translate_batch(
                &self,
                req: &TranslateRequest,
            ) -> Result<TranslateResponse, LlmError> {
                Ok(TranslateResponse {
                    results: req
                        .units
                        .iter()
                        .map(|u| TranslateUnitResult {
                            id: u.id.clone(),
                            translation: u.source.replace("{0}", ""), // drops placeholder!
                        })
                        .collect(),
                    usage: None,
                })
            }
            fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
                Ok(self.info())
            }
        }
        let p = BrokenProvider;
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let units = vec![unit("a", "Drink {0}")];
        let summary = engine.translate(&units, |_| {}).unwrap();
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn batching_respects_budget() {
        let units: Vec<TranslateUnit> = (0..50)
            .map(|i| unit(&format!("u{i}"), &"x".repeat(300)))
            .collect();
        let mut v: Vec<&TranslateUnit> = units.iter().collect();
        let batches = batch_by_budget(&mut v, 1000);
        assert!(
            batches.len() >= 10,
            "expected ~15+ batches, got {}",
            batches.len()
        );
        let total: usize = batches.iter().map(|b| b.len()).sum();
        assert_eq!(total, 50);
    }
}
