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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    /// Cooperative cancellation (review 2026-10-14 P1): when set and the
    /// flag flips to `true`, the engine stops at the NEXT boundary — before
    /// each new batch, before each provider request/retry attempt, right
    /// after an in-flight request completes, and during every backoff
    /// sleep — and returns [`LlmError::Cancelled`]. Results of batches
    /// that completed BEFORE the flip keep their checkpoint records;
    /// the in-flight batch at flip time is discarded (never published,
    /// never checkpointed), so a resumed run re-translates exactly the
    /// unfinished tail.
    pub cancel: Option<Arc<AtomicBool>>,
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
            cancel: None,
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

    fn is_cancelled(&self) -> bool {
        self.opts
            .cancel
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
    }

    /// Sleep that wakes early on cancellation: `true` = elapsed fully,
    /// `false` = cancelled mid-wait. Sliced so a cancel observed during a
    /// rate-limit/backoff wait stops the retry within one slice (25ms)
    /// instead of sleeping the full provider-advised window.
    fn wait_or_cancel(&self, dur: Duration) -> bool {
        const SLICE: Duration = Duration::from_millis(25);
        let mut left = dur;
        loop {
            if self.is_cancelled() {
                return false;
            }
            if left.is_zero() {
                return true;
            }
            let step = left.min(SLICE);
            std::thread::sleep(step);
            left = left.saturating_sub(step);
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
            // Boundary 1: before each NEW batch — a cancel between batches
            // never dispatches the next provider request.
            if self.is_cancelled() {
                return Err(LlmError::Cancelled);
            }
            summary.batches += 1;
            let req = TranslateRequest {
                source_lang: self.opts.source_lang.clone(),
                target_lang: self.opts.target_lang.clone(),
                units: batch.to_vec(),
                glossary: glossary.clone(),
            };

            let mut attempt = 0u32;
            let response = loop {
                // Boundary 2+3: before each provider request AND before
                // each retry attempt (every attempt enters here first).
                if self.is_cancelled() {
                    return Err(LlmError::Cancelled);
                }
                match self.provider.translate_batch(&req) {
                    Ok(r) => {
                        // Boundary 4: after the in-flight request completes.
                        // Results arriving after the flip are discarded —
                        // never published, never checkpointed — so nothing
                        // new becomes visible after Cancel.
                        if self.is_cancelled() {
                            return Err(LlmError::Cancelled);
                        }
                        break Ok(r);
                    }
                    Err(LlmError::RateLimited(provider, retry_after)) => {
                        attempt += 1;
                        summary.retries += 1;
                        if attempt > self.opts.max_retries {
                            break Err(LlmError::RateLimited(provider, retry_after));
                        }
                        // Boundary 5: after backoff/sleep — the wait wakes
                        // early on cancel and the next loop iteration (or
                        // the return below) stops before any new request.
                        if !self.wait_or_cancel(retry_after.min(Duration::from_secs(5))) {
                            return Err(LlmError::Cancelled);
                        }
                    }
                    Err(e) => {
                        attempt += 1;
                        summary.retries += 1;
                        if attempt > self.opts.max_retries {
                            break Err(e);
                        }
                        // Transient failures (server 5xx, network) back off
                        // exponentially before the next attempt; deterministic
                        // client errors (4xx and the like) retry immediately.
                        if is_transient(&e) && !self.wait_or_cancel(retry_backoff(attempt)) {
                            return Err(LlmError::Cancelled);
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
                        // Boundary 6 (review round 2, P1-B): before EACH
                        // unit's publication within a batch. The in-flight
                        // check above covers the response arrival, but a
                        // flip landing mid-batch must not keep writing
                        // checkpoints / publishing outcomes that the user
                        // cancelled after the first results landed.
                        if self.is_cancelled() {
                            return Err(LlmError::Cancelled);
                        }
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
                        // Boundary 6, failure branch: a flip landing while
                        // a failed response is being reported must stop
                        // the remaining per-unit failures and surface
                        // Err(Cancelled) instead of a silent Ok.
                        if self.is_cancelled() {
                            return Err(LlmError::Cancelled);
                        }
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
fn batch_by_budget(units: &mut Vec<&TranslateUnit>, budget: usize) -> Vec<Vec<TranslateUnit>> {
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

/// Failures worth sleeping over before the next attempt: transport/network
/// errors and provider-side 5xx. Client errors (4xx, auth, malformed
/// payloads) are deterministic — a retry hits the same wall, so they are
/// retried without backoff.
fn is_transient(e: &LlmError) -> bool {
    if matches!(e, LlmError::ServerError { .. }) {
        return true;
    }
    #[cfg(feature = "http")]
    if matches!(e, LlmError::Http(_)) {
        return true;
    }
    false
}

/// Exponential backoff before retry `attempt` (1-based): 0.5s, 1s, 2s, …
/// capped at 8s so a stalled provider never freezes a batch for minutes.
fn retry_backoff(attempt: u32) -> Duration {
    let exp = attempt.saturating_sub(1).min(4);
    Duration::from_millis(500)
        .saturating_mul(2u32.saturating_pow(exp))
        .min(Duration::from_secs(8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockProvider;
    use crate::provider::{Provider, ProviderInfo, TranslateResponse, TranslateUnitResult};
    use std::sync::atomic::AtomicUsize;

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

    /// Fails the first N calls with a provider-side 5xx, then succeeds —
    /// drives the transient-error backoff path deterministically.
    struct FlakyServer {
        fail_first: usize,
        calls: AtomicUsize,
    }
    impl Provider for FlakyServer {
        fn id(&self) -> &str {
            "flaky"
        }
        fn model(&self) -> &str {
            "flaky-1"
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: self.id().into(),
                model: self.model().into(),
                endpoint: "x".into(),
            }
        }
        fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
            let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < self.fail_first {
                return Err(LlmError::ServerError {
                    provider: self.id().into(),
                    status: 503,
                    message: "upstream unavailable".into(),
                });
            }
            Ok(TranslateResponse {
                results: req
                    .units
                    .iter()
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
    fn server_error_is_backed_off_then_retried_to_success() {
        let p = FlakyServer {
            fail_first: 2,
            calls: AtomicUsize::new(0),
        };
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let summary = engine.translate(&[unit("a", "Hello")], |_| {}).unwrap();
        assert_eq!(summary.translated, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.retries, 2);
    }

    #[test]
    fn server_error_exhausts_retries_and_fails_the_batch() {
        let p = FlakyServer {
            fail_first: usize::MAX,
            calls: AtomicUsize::new(0),
        };
        let engine = TranslationEngine::new(&p, EngineOptions::default(), Glossary::default());
        let summary = engine.translate(&[unit("a", "Hello")], |_| {}).unwrap();
        assert_eq!(summary.translated, 0);
        assert_eq!(summary.failed, 1);
        // Contract preserved: max_retries=3 → exactly 4 provider calls
        // (initial + 3 retries). summary.retries counts every failed
        // attempt, including the terminal one.
        assert_eq!(summary.retries, 4);
        assert_eq!(
            p.calls.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "max 3 retries as before"
        );
    }

    #[test]
    fn only_transport_and_server_errors_are_transient() {
        // Transport errors exist only with the `http` feature.
        #[cfg(feature = "http")]
        {
            // A real reqwest connect error, produced offline: loopback port 9
            // (discard) refuses immediately on any dev/CI machine.
            let http_err = reqwest::blocking::Client::new()
                .get("http://127.0.0.1:9/")
                .timeout(Duration::from_secs(2))
                .send()
                .expect_err("loopback port 9 must refuse connections");
            let http = LlmError::Http(http_err);
            assert!(is_transient(&http));
        }
        let server = LlmError::ServerError {
            provider: "p".into(),
            status: 502,
            message: "bad gateway".into(),
        };
        assert!(is_transient(&server));
        // 4xx-class provider errors and malformed payloads: no backoff.
        assert!(!is_transient(&LlmError::Provider(
            "p".into(),
            "HTTP 401 unauthorized".into()
        )));
        assert!(!is_transient(&LlmError::InvalidResponse("bad json".into())));
        assert!(!is_transient(&LlmError::MissingKey("p".into())));
    }

    #[test]
    fn backoff_grows_exponentially_and_is_capped() {
        assert_eq!(retry_backoff(1), Duration::from_millis(500));
        assert_eq!(retry_backoff(2), Duration::from_secs(1));
        assert_eq!(retry_backoff(3), Duration::from_secs(2));
        // Cap: long retry chains never sleep longer than 8s.
        assert_eq!(retry_backoff(10), Duration::from_secs(8));
        assert_eq!(retry_backoff(u32::MAX), Duration::from_secs(8));
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

    // --- cancellation (review 2026-10-14 P1) ---
    use std::sync::Mutex;

    /// Provider that blocks the Nth call (0-based) on a channel and counts
    /// every dispatch. The blocking call signals `started` BEFORE waiting,
    /// so the test interleaves deterministically.
    struct BlockingGate {
        block_call: usize,
        started_tx: std::sync::mpsc::Sender<()>,
        release_rx: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
        calls: Arc<AtomicUsize>,
    }
    impl Provider for BlockingGate {
        fn id(&self) -> &str {
            "gate"
        }
        fn model(&self) -> &str {
            "gate-1"
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: self.id().into(),
                model: self.model().into(),
                endpoint: "x".into(),
            }
        }
        fn translate_batch(&self, req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
            let n = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == self.block_call {
                let _ = self.started_tx.send(());
                if let Some(rx) = self.release_rx.lock().unwrap().take() {
                    // A stuck regression must fail the test, not hang CI.
                    let _ = rx.recv_timeout(Duration::from_secs(10));
                }
            }
            Ok(TranslateResponse {
                results: req
                    .units
                    .iter()
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

    fn long_units(n: usize) -> Vec<TranslateUnit> {
        (0..n)
            .map(|i| unit(&format!("u{i}"), &"x".repeat(300)))
            .collect()
    }

    /// The reviewer's deterministic scenario: multi-batch job, the FIRST
    /// provider request is blocked, cancel flips while it is in flight,
    /// release — the engine must return Cancelled having dispatched
    /// exactly ONE request, with no retry and no further batch, and the
    /// checkpoint file must stay parseable (zero records: nothing had
    /// completed before the flip).
    #[test]
    fn cancel_while_first_request_in_flight_stops_all_further_dispatch() {
        let tmp = tempfile::tempdir().unwrap();
        let cp = tmp.path().join("cp.jsonl");
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (rel_tx, rel_rx) = std::sync::mpsc::channel();
        let p = BlockingGate {
            block_call: 0,
            started_tx,
            release_rx: Mutex::new(Some(rel_rx)),
            calls: calls.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            batch_char_budget: 200, // one 316-weight unit per batch → 6 batches
            checkpoint_path: Some(cp.clone()),
            cancel: Some(cancel.clone()),
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || {
            let engine = TranslationEngine::new(&p, opts, Glossary::default());
            let _ = tx.send(engine.translate(&long_units(6), |_| {}));
        });
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("first provider call must start");
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        drop(rel_tx); // release the blocked call
        let result = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("engine must stop promptly after cancel");
        t.join().unwrap();
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "no second batch, no retry after cancel"
        );
        let lines = if cp.exists() {
            std::fs::read_to_string(&cp).unwrap()
        } else {
            String::new() // lazy store: zero records → no file yet
        };
        assert_eq!(
            lines.lines().count(),
            0,
            "nothing completed before the flip, so no records"
        );
    }

    /// Cancel with batch 1 already COMPLETED: its checkpoint record must
    /// survive (resume stays possible), the in-flight batch 2 discarded,
    /// and batch 3 never dispatched.
    #[test]
    fn cancel_preserves_completed_checkpoint_and_discards_inflight_batch() {
        let tmp = tempfile::tempdir().unwrap();
        let cp = tmp.path().join("cp.jsonl");
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (rel_tx, rel_rx) = std::sync::mpsc::channel();
        let p = BlockingGate {
            block_call: 1, // block the SECOND call; batch 1 completes first
            started_tx,
            release_rx: Mutex::new(Some(rel_rx)),
            calls: calls.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            batch_char_budget: 200,
            checkpoint_path: Some(cp.clone()),
            cancel: Some(cancel.clone()),
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || {
            let engine = TranslationEngine::new(&p, opts, Glossary::default());
            let _ = tx.send(engine.translate(&long_units(6), |_| {}));
        });
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("second provider call must start");
        // Happens-before via the channel: batch 1 (call 0) fully processed,
        // so its checkpoint record is already on disk.
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        drop(rel_tx);
        let result = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("engine must stop promptly after cancel");
        t.join().unwrap();
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "batch 2 was in flight before the flip; batch 3 must never dispatch"
        );
        let lines = std::fs::read_to_string(&cp).unwrap();
        let records: Vec<&str> = lines.lines().collect();
        assert_eq!(records.len(), 1, "exactly batch 1's record: {records:?}");
        assert!(records[0].contains("\"u0\""), "{records:?}");
    }

    /// Provider that always answers 429 with a 30s advisory (capped to 5s
    /// by the engine) — drives the rate-limit wait path.
    struct RateLimitAlways {
        calls: Arc<AtomicUsize>,
    }
    impl Provider for RateLimitAlways {
        fn id(&self) -> &str {
            "rl"
        }
        fn model(&self) -> &str {
            "rl-1"
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: self.id().into(),
                model: self.model().into(),
                endpoint: "x".into(),
            }
        }
        fn translate_batch(&self, _req: &TranslateRequest) -> Result<TranslateResponse, LlmError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(LlmError::RateLimited(
                self.id().into(),
                Duration::from_secs(30),
            ))
        }
        fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
            Ok(self.info())
        }
    }

    #[test]
    fn cancel_during_rate_limit_wait_stops_without_new_request() {
        let calls = Arc::new(AtomicUsize::new(0));
        let p = RateLimitAlways {
            calls: calls.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            cancel: Some(cancel.clone()),
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || {
            let engine = TranslationEngine::new(&p, opts, Glossary::default());
            let _ = tx.send(engine.translate(&[unit("a", "Hello")], |_| {}));
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while calls.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "provider call never started"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let started = std::time::Instant::now();
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        let result = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("cancel must cut the 5s rate-limit wait (bounded latency)");
        t.join().unwrap();
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "no retry attempt after the flip"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "cancellation latency must be bounded well under the 5s cap, took {:?}",
            started.elapsed()
        );
    }

    /// Same contract for the transient 5xx backoff path: cancel during the
    /// exponential backoff sleep stops before the next attempt.
    #[test]
    fn cancel_during_transient_backoff_stops_retry() {
        struct ServerErrorAlways {
            calls: Arc<AtomicUsize>,
        }
        impl Provider for ServerErrorAlways {
            fn id(&self) -> &str {
                "srv"
            }
            fn model(&self) -> &str {
                "srv-1"
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
                _req: &TranslateRequest,
            ) -> Result<TranslateResponse, LlmError> {
                self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(LlmError::ServerError {
                    provider: self.id().into(),
                    status: 503,
                    message: "down".into(),
                })
            }
            fn test_connection(&self) -> Result<ProviderInfo, LlmError> {
                Ok(self.info())
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let p = ServerErrorAlways {
            calls: calls.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            cancel: Some(cancel.clone()),
            ..Default::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let t = std::thread::spawn(move || {
            let engine = TranslationEngine::new(&p, opts, Glossary::default());
            let _ = tx.send(engine.translate(&[unit("a", "Hello")], |_| {}));
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while calls.load(std::sync::atomic::Ordering::SeqCst) == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "provider call never started"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let started = std::time::Instant::now();
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        // First backoff is 500ms; the engine must wake inside it.
        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("cancel must cut the 500ms backoff");
        t.join().unwrap();
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn cancel_before_start_never_calls_the_provider() {
        struct Counting {
            calls: Arc<AtomicUsize>,
        }
        impl Provider for Counting {
            fn id(&self) -> &str {
                "cnt"
            }
            fn model(&self) -> &str {
                "cnt-1"
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
                self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(TranslateResponse {
                    results: req
                        .units
                        .iter()
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
        let calls = Arc::new(AtomicUsize::new(0));
        let p = Counting {
            calls: calls.clone(),
        };
        let cancel = Arc::new(AtomicBool::new(true));
        let opts = EngineOptions {
            cancel: Some(cancel),
            ..Default::default()
        };
        let engine = TranslationEngine::new(&p, opts, Glossary::default());
        let result = engine.translate(&[unit("a", "Hello")], |_| {});
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    /// Review round 2, P1-B: the flip lands INSIDE the on_unit callback of
    /// a multi-unit batch. Units not yet published must never reach
    /// on_unit or the checkpoint, and the engine must surface
    /// Err(Cancelled) instead of a silent Ok.
    #[test]
    fn cancel_from_callback_stops_remaining_publications_of_batch() {
        let tmp = tempfile::tempdir().unwrap();
        let cp = tmp.path().join("cp.jsonl");
        let p = MockProvider::new();
        let flag = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            checkpoint_path: Some(cp.clone()),
            cancel: Some(flag.clone()),
            ..Default::default()
        };
        let engine = TranslationEngine::new(&p, opts, Glossary::default());
        let units = vec![unit("a", "Hello"), unit("b", "World"), unit("c", "Again")];
        let mut seen = 0usize;
        let result = engine.translate(&units, |o| {
            if o.status == "translated" {
                seen += 1;
                if seen == 1 {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(
            seen, 1,
            "units after the flip must never be published: {seen}"
        );
        let lines = std::fs::read_to_string(&cp).unwrap();
        let records: Vec<&str> = lines.lines().collect();
        assert_eq!(
            records.len(),
            1,
            "only the unit processed before the flip is checkpointed: {records:?}"
        );
        assert!(records[0].contains("\"a\""), "{records:?}");
    }

    /// Same mid-batch contract for the FAILURE branch: a flip during the
    /// failed-response publication stops the remaining per-unit failures
    /// and turns the overall result into Err(Cancelled), never Ok.
    #[test]
    fn cancel_from_callback_stops_failure_publication() {
        let p = FlakyServer {
            fail_first: usize::MAX,
            calls: AtomicUsize::new(0),
        };
        let flag = Arc::new(AtomicBool::new(false));
        let opts = EngineOptions {
            cancel: Some(flag.clone()),
            ..Default::default()
        };
        let engine = TranslationEngine::new(&p, opts, Glossary::default());
        let units = vec![unit("a", "Hello"), unit("b", "World"), unit("c", "Again")];
        let mut seen_failed = 0usize;
        let result = engine.translate(&units, |o| {
            if o.status == "failed" {
                seen_failed += 1;
                if seen_failed == 1 {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }
        });
        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert_eq!(
            seen_failed, 1,
            "remaining failures must not be published after the flip"
        );
        // The four attempts (initial + max_retries retries) predate the
        // flip — they happened while the retry loop exhausted on the
        // always-failing provider. The flip cannot retract them; the
        // contract under test is that publication stops and the outcome
        // is Err(Cancelled).
        assert_eq!(p.calls.load(std::sync::atomic::Ordering::SeqCst), 4);
    }
}
