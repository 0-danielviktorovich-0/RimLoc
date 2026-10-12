//! Chat batch (external-AI workflow WITHOUT an API) — the manager side of
//! the mandate's canonical path:
//!
//! canonical selection → deterministic batching → export batch →
//! `not_started/exported/imported/stale/done` → copy prompt → the user
//! pastes the LLM-chat response → strict parser → identity/revision/stale
//! gates → preview → apply (`ApplyOrigin::Import`) → validate (the session
//! per-intent gates) → persist-before-ack.
//!
//! Responsibilities split:
//! - THIS module owns batch semantics: selection pinning, prompt building,
//!   strict response parsing, the gates, and the durable batch file in the
//!   managed store (`<batch-id>.chatbatch.json`);
//! - the actual translation write goes through the ONE mutating seam
//!   ([`ProjectSessionManager::apply`]) — persist-before-ack, epoch +
//!   revision guards, eligibility and provenance stay exactly where every
//!   other path has them. This module NEVER writes translations itself and
//!   reads live state only through the crate-internal
//!   [`ProjectSessionManager::with_live_project`] read seam.
//!
//! Honest boundaries: nothing here performs a network call — the transport
//! is the user's clipboard; the parser is strict by mandate (malformed,
//! unknown, extra, duplicate, missing, stale revision, wrong project are
//! WHOLE-import refusals, never a partial apply); identities are resolved
//! ONCE at create and pinned structurally, so the string keys in a chat
//! response can never steer the apply to an unintended entry.

use crate::contract::{
    ApplyIntentsRequest, ApplyOrigin, ChatBatch, ChatBatchApplyRequest, ChatBatchApplyResponse,
    ChatBatchCreateRequest, ChatBatchExportRequest, ChatBatchExportResponse,
    ChatBatchImportRequest, ChatBatchImportResponse, ChatBatchItem, ChatBatchListRequest,
    ChatBatchListResponse, ChatBatchPreview, ChatBatchPreviewItem, ChatBatchStatus,
    ChatBatchStatusRequest, ChatBatchStatusResponse, ChatGlossaryPair, ContractError,
    ContractErrorCode, IntentAction, ProjectId, TranslationIntent, CHAT_BATCH_FILE_VERSION,
};
use crate::observability::{generate_operation_id, sha256_hex};
use crate::session::ProjectSessionManager;
use rimloc_domain::canonical::Origin;
use rimloc_domain::eligibility::Decision;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

/// Managed batch file extension (inside the manager's managed root, next to
/// the `proj-*.rimloc.json` envelopes).
const CHAT_BATCH_EXT: &str = "chatbatch.json";

/// Delimiter between the key and the translation on one line, as the
/// prompt prescribes it (`<key>: <translation>`). Keys are
/// `display_identity()` forms (`keyed·Save`,
/// `def_injected·ThingDef·X.label`) and NEVER contain a colon, so the
/// pair boundary is the FIRST colon; the parser treats a missing space
/// after it as the same pair (no ambiguity is possible) and translations
/// may freely contain further colons.
const PAIR_DELIM: &str = ": ";

// --- structured response format v1 (mandate §8: a versioned schema) ---
// Line-oriented blocks instead of JSON: multi-line translations survive
// verbatim and models hold the shape far more reliably than escaped JSON
// strings. The legacy one-line format stays accepted forever (§9
// compatibility); detection is the first non-blank line.

/// The header line the structured response MUST start with.
pub const CHAT_RESPONSE_VERSION: &str = "RIMLOC-CHAT-RESPONSE v1";
/// The family prefix — a different `RIMLOC-CHAT-RESPONSE <ver>` header is
/// a typed refusal naming the unsupported version, never a guess.
const CHAT_RESPONSE_FAMILY: &str = "RIMLOC-CHAT-RESPONSE";
const DIR_BATCH: &str = "batch:";
const DIR_LOCALE: &str = "locale:";
const DIR_BEGIN: &str = "BEGIN_ENTRY";
const DIR_END: &str = "END_ENTRY";
const DIR_KEY: &str = "key:";
const DIR_REVIEW: &str = "needs_review:";
const DIR_NOTE: &str = "note:";
const DIR_TRANSLATION: &str = "translation:";
const DIR_END_RESPONSE: &str = "END_RESPONSE";
/// Deterministic cap on glossary pairs baked into one prompt (mandate §6:
/// keep token usage efficient; the project glossary can be arbitrarily
/// large while one batch carries at most this many pairs).
const PROMPT_GLOSSARY_MAX: usize = 50;

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The wire string of a domain [`Origin`] (serde snake_case vocabulary,
/// mirrors the frontend `TranslationDto.origin`).
fn origin_str(o: &Origin) -> &'static str {
    match o {
        Origin::Unknown => "unknown",
        Origin::Human => "human",
        Origin::Tm => "tm",
        Origin::Llm => "llm",
        Origin::Imported => "imported",
    }
}

impl ProjectSessionManager {
    /// The managed file for a batch id. FAIL-CLOSED like
    /// [`ProjectSessionManager::managed_path`]: the id must match the mint
    /// form (`batch-<a-z0-9->`) — client strings with `/`, `..` or
    /// absolute prefixes are a typed violation BEFORE any filesystem
    /// access — and the path must stay inside the managed root.
    fn chat_batch_path(&self, batch_id: &str) -> Result<PathBuf, ContractError> {
        static BATCH_ID_FORM: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new(r"^batch-[a-z0-9-]+$").unwrap());
        if !BATCH_ID_FORM.is_match(batch_id) {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed batch id: expected the minted `batch-<id>` form".to_string(),
            ));
        }
        let path = self
            .managed_root()
            .join(format!("{batch_id}.{CHAT_BATCH_EXT}"));
        if !crate::util::is_within_allow(&path, self.managed_root()) {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "chat batch path escapes the managed root".to_string(),
            ));
        }
        Ok(path)
    }

    fn load_batch(&self, batch_id: &str) -> Result<ChatBatch, ContractError> {
        let path = self.chat_batch_path(batch_id)?;
        if !path.is_file() {
            return Err(ContractError::new(
                ContractErrorCode::ProjectNotFound,
                format!("no chat batch with id `{batch_id}`"),
            ));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| {
            ContractError::new(
                ContractErrorCode::Internal,
                format!("chat batch `{batch_id}` unreadable: {e}"),
            )
        })?;
        let batch: ChatBatch = serde_json::from_str(&text).map_err(|e| {
            ContractError::new(
                ContractErrorCode::Internal,
                format!("chat batch `{batch_id}` corrupt: {e}"),
            )
        })?;
        if batch.schema_version != CHAT_BATCH_FILE_VERSION {
            return Err(ContractError::new(
                ContractErrorCode::SchemaVersion,
                format!(
                    "chat batch `{batch_id}` schema_version {} not supported (expected {CHAT_BATCH_FILE_VERSION})",
                    batch.schema_version
                ),
            ));
        }
        Ok(batch)
    }

    fn save_batch(&self, batch: &ChatBatch) -> Result<(), ContractError> {
        let path = self.chat_batch_path(&batch.batch_id)?;
        let bytes = serde_json::to_vec_pretty(batch).map_err(|e| {
            ContractError::new(
                ContractErrorCode::Internal,
                format!("chat batch `{}` serialize failed: {e}", batch.batch_id),
            )
        })?;
        crate::util::write_atomic(&path, &bytes).map_err(|e| {
            ContractError::new(
                ContractErrorCode::SaveFailed,
                format!("chat batch `{}` persist failed: {e}", batch.batch_id),
            )
        })
    }

    fn mint_batch_id(&self) -> String {
        loop {
            let id = generate_operation_id().replacen("op-", "batch-", 1);
            // Generated ids match the form by construction; only the
            // on-disk collision needs a retry.
            if !self
                .managed_root()
                .join(format!("{id}.{CHAT_BATCH_EXT}"))
                .exists()
            {
                return id;
            }
        }
    }

    /// `chat_batch_create` (mandate: `create_batch(project_id, entry_keys[],
    /// locale) -> ChatBatch`): pin a selection into a fresh batch. Every
    /// key must resolve 1:1 against the LIVE inventory
    /// (`display_identity()` exact match) and be translatable — unknown,
    /// duplicate and non-translatable keys refuse the WHOLE create, so the
    /// batch is exactly the contract of what the prompt will contain.
    pub fn chat_batch_create(
        &self,
        req: &ChatBatchCreateRequest,
    ) -> Result<ChatBatch, ContractError> {
        self.managed_path(&req.project_id)?;
        let locale = req.locale.trim();
        if !crate::util::lang_dir_form_ok(locale) {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!("malformed locale `{locale}`: expected the language-folder form (letters, digits, `_`, `-`)"),
            ));
        }
        if req.entry_keys.is_empty() {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                "chat batch needs at least one entry key".to_string(),
            ));
        }
        // Duplicates in the SELECTION are a client bug — refuse strictly
        // (the parser's duplicate rule covers the response side; this is
        // the selection side of the same honesty).
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for k in &req.entry_keys {
            if !seen.insert(k.as_str()) {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!("duplicate entry key in the selection: `{k}`"),
                ));
            }
        }

        // Pin the items + record the gates from ONE consistent view of the
        // live trusted state (epoch guard included). Per item the
        // TARGET-locale translation state at selection time is pinned too —
        // the conflict gate at import compares the live state against this
        // snapshot, so a human fix that lands after the selection (through
        // ANY path, including before a re-export) can never be silently
        // overwritten by a stale response.
        let prepared = self.with_live_project(&req.project_id, |epoch, revision, project| {
            if epoch != req.session_epoch {
                return Err(ContractError::stale_epoch(req.session_epoch, epoch));
            }
            let engine = crate::eligibility_engine::EligibilityEngine::new();
            let mut items = Vec::with_capacity(req.entry_keys.len());
            for key in &req.entry_keys {
                let entry = project
                    .entries
                    .iter()
                    .find(|e| e.id.display_identity() == *key)
                    .ok_or_else(|| {
                        ContractError::new(
                            ContractErrorCode::ContractViolation,
                            format!(
                                "entry key `{key}` not found in the inventory (exact `display_identity` match required)"
                            ),
                        )
                    })?;
                let verdict = engine.evaluate(entry, None);
                if verdict.decision == Decision::NonTranslatable {
                    return Err(ContractError::new(
                        ContractErrorCode::ContractViolation,
                        format!(
                            "entry key `{key}` is classified non-translatable and cannot join a chat batch"
                        ),
                    ));
                }
                let current = project.translation(&entry.id, locale);
                items.push(ChatBatchItem {
                    key: key.clone(),
                    source: entry.text.clone(),
                    entry: entry.id.clone(),
                    // ALWAYS Some for batches written by this build (empty
                    // string = untranslated at select); `None` on load means
                    // a v1-era file without the snapshot — the conflict gate
                    // never claims anything about what it cannot know.
                    translation_at_select: Some(
                        current
                            .and_then(|t| t.text.clone())
                            .unwrap_or_default(),
                    ),
                    origin_at_select: current.map(|t| origin_str(&t.origin).to_string()),
                });
            }
            Ok((items, revision))
        })?;
        let (items, revision) = prepared;

        let batch = ChatBatch {
            schema_version: CHAT_BATCH_FILE_VERSION,
            batch_id: self.mint_batch_id(),
            project_id: req.project_id.clone(),
            locale: locale.to_string(),
            revision_at_export: revision,
            source_hash: chat_batch_source_hash(&items),
            items,
            status: ChatBatchStatus::NotStarted,
            preview: None,
            created_at_ms: now_ms(),
            exported_at_ms: None,
            applied_at_ms: None,
            applied_revision: None,
            source_locale: None,
            glossary_used: Vec::new(),
        };
        self.save_batch(&batch)?;
        Ok(batch)
    }

    /// `chat_batch_export` (`export_prompt(batch) -> structured text for
    /// LLM chat`): build the prompt from the CURRENT project state and
    /// persist `exported` with the fresh revision + source hash. The
    /// pinned sources must still match the live inventory (any drift
    /// refuses and asks for a fresh batch — a prompt that lies about the
    /// source text is worse than no prompt). Export is the recovery path
    /// for `stale` too: it re-records the gates. A `done` batch is
    /// terminal — recreate instead.
    pub fn chat_batch_export(
        &self,
        req: &ChatBatchExportRequest,
    ) -> Result<ChatBatchExportResponse, ContractError> {
        let mut batch = self.load_batch(&req.batch_id)?;
        let live = self.with_live_project(&batch.project_id, |epoch, revision, project| {
            if epoch != req.session_epoch {
                return Err(ContractError::stale_epoch(req.session_epoch, epoch));
            }
            if batch.status == ChatBatchStatus::Done {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "chat batch `{}` is done; create a new batch instead of re-exporting applied work",
                        batch.batch_id
                    ),
                ));
            }
            // The pinned (key, source) lines must still describe the live
            // inventory: an identity that vanished or a source text that
            // changed makes the exported prompt a lie.
            let hash = match chat_batch_inventory_hash(project, &batch.items) {
                Some(h) if h == chat_batch_source_hash(&batch.items) => h,
                Some(_) => {
                    return Err(ContractError::new(
                        ContractErrorCode::ContractViolation,
                        "batch basis drifted: a source text changed since the selection; create a new batch"
                            .to_string(),
                    ))
                }
                None => {
                    return Err(ContractError::new(
                        ContractErrorCode::ContractViolation,
                        "batch basis drifted: an entry is missing from the inventory; rescan and create a new batch"
                            .to_string(),
                    ))
                }
            };
            // Prompt context (mandate §6/§7), ONE consistent live view: the
            // source language of the pinned entries and the glossary pairs
            // RELEVANT to this batch — a term whose source-language form
            // appears (case-insensitively) in at least one pinned source.
            // Irrelevant terms stay out (token efficiency).
            let source_locale = batch.items.first().and_then(|i| {
                project
                    .entries
                    .iter()
                    .find(|e| e.id == i.entry)
                    .map(|e| e.source_locale.clone())
            });
            let sources: Vec<String> = batch
                .items
                .iter()
                .map(|i| i.source.to_lowercase())
                .collect();
            let mut glossary_used = Vec::new();
            for g in &project.glossary {
                let term = g.term.trim();
                if term.is_empty() {
                    continue;
                }
                let needle = term.to_lowercase();
                if sources.iter().any(|s| s.contains(&needle)) {
                    glossary_used.push(ChatGlossaryPair {
                        term: term.to_string(),
                        translation: g.translation.clone(),
                    });
                    if glossary_used.len() >= PROMPT_GLOSSARY_MAX {
                        break;
                    }
                }
            }
            Ok((hash, revision, source_locale, glossary_used))
        })?;
        let (hash, revision, source_locale, glossary_used) = live;

        batch.revision_at_export = revision;
        batch.source_hash = hash;
        batch.source_locale = source_locale;
        batch.glossary_used = glossary_used;
        batch.status = ChatBatchStatus::Exported;
        batch.exported_at_ms = Some(now_ms());
        batch.preview = None;
        let prompt = build_chat_prompt(&batch);
        self.save_batch(&batch)?;
        Ok(ChatBatchExportResponse {
            job_id: generate_operation_id(),
            batch,
            prompt,
        })
    }

    /// `chat_batch_import` (`import_response(batch_id, response_text)`):
    /// strict-parse the pasted response and store it as the pending
    /// preview. Gates in order: known batch, live session + epoch, status
    /// `exported`/`imported`, project revision (stale gate), source hash
    /// (mandate), then the parser (malformed / unknown / duplicate / empty
    /// / missing are WHOLE-import refusals). On success the batch persists
    /// as `imported` — nothing durable has landed yet; the apply is a
    /// separate, human-confirmed step.
    pub fn chat_batch_import(
        &self,
        req: &ChatBatchImportRequest,
    ) -> Result<ChatBatchImportResponse, ContractError> {
        let mut batch = self.load_batch(&req.batch_id)?;
        match batch.status {
            ChatBatchStatus::Exported | ChatBatchStatus::Imported => {}
            ChatBatchStatus::NotStarted => {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "chat batch `{}` was never exported; export the prompt before importing a response",
                        batch.batch_id
                    ),
                ));
            }
            ChatBatchStatus::Stale => {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "chat batch `{}` is stale (the project moved under it); re-export the prompt and translate THAT",
                        batch.batch_id
                    ),
                ));
            }
            ChatBatchStatus::Done => {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!("chat batch `{}` is already applied", batch.batch_id),
                ));
            }
        }

        let checked = self.with_live_project(&batch.project_id, |epoch, revision, project| {
            if epoch != req.session_epoch {
                return Err(ContractError::stale_epoch(req.session_epoch, epoch));
            }
            // Stale gate (revision): any acked project change after the
            // export invalidates the paste — the model translated the OLD
            // lines.
            if revision != batch.revision_at_export {
                return Err(ContractError::stale_revision(
                    batch.revision_at_export,
                    revision,
                ));
            }
            // Source hash gate (mandate): belt-and-braces over the
            // revision — the pinned sources must still be exactly what the
            // prompt showed. A missing identity is drift, never "in sync".
            match chat_batch_inventory_hash(project, &batch.items) {
                Some(h) if h == batch.source_hash => {}
                _ => {
                    return Err(ContractError::new(
                        ContractErrorCode::ContractViolation,
                        "source hash mismatch: the batch lines no longer match the inventory; re-export the batch"
                            .to_string(),
                    ))
                }
            }
            // Live target-locale state per pinned key, for the per-row
            // conflict analysis (the conflict gate compares the response
            // against the SELECTION-time snapshot, so a human fix that
            // landed after the selection — through any path — is surfaced
            // instead of silently overwritten).
            let live = batch
                .items
                .iter()
                .map(|i| {
                    let t = project.translation(&i.entry, &batch.locale);
                    (
                        i.key.clone(),
                        t.map(|t| {
                            (
                                t.text.clone().unwrap_or_default(),
                                origin_str(&t.origin).to_string(),
                            )
                        }),
                    )
                })
                .collect::<Vec<_>>();
            Ok(live)
        });
        let live = match checked {
            Ok(live) => live,
            Err(e) => {
                // A drift refusal at import means the batch is honestly STALE
                // (persisted) — re-export is the recovery path. Other refusals
                // (stale epoch) leave the status alone.
                if matches!(
                    e.code,
                    ContractErrorCode::StaleRevision | ContractErrorCode::ContractViolation
                ) {
                    batch.status = ChatBatchStatus::Stale;
                    let _ = self.save_batch(&batch);
                }
                return Err(e);
            }
        };

        let parsed = parse_chat_response(&req.response_text, &batch).map_err(|e| {
            ContractError::new(ContractErrorCode::ContractViolation, e.message)
                .with_details(e.details)
        })?;
        // Structural validation (mandate §14): the deterministic core —
        // the placeholder set of every response row must match the pinned
        // source exactly (lost / renamed / unexpected tokens name the row).
        // A mismatching row becomes a PROBLEM row (held back from apply
        // unless overridden), never a silent apply.
        let mut parsed = parsed;
        for p in &mut parsed {
            if let Some(item) = batch.items.iter().find(|i| i.key == p.key) {
                if let Some(mismatch) =
                    rimloc_validate::placeholder_set_mismatch(&item.source, &p.text)
                {
                    p.structural_problem = Some(mismatch);
                }
            }
        }
        let preview_items = attach_conflicts(&batch, parsed, &live);

        let preview = ChatBatchPreview {
            items: preview_items,
            imported_at_ms: now_ms(),
        };
        batch.status = ChatBatchStatus::Imported;
        batch.preview = Some(preview.clone());
        self.save_batch(&batch)?;
        Ok(ChatBatchImportResponse {
            job_id: generate_operation_id(),
            batch,
            preview,
        })
    }

    /// `chat_batch_status`: read the durable batch + the live drift
    /// verdict. PURE: never flips the persisted status (a refused
    /// import/apply does that); the badge renders `stale` from here
    /// without waiting for an operation to refuse.
    pub fn chat_batch_status(
        &self,
        req: &ChatBatchStatusRequest,
    ) -> Result<ChatBatchStatusResponse, ContractError> {
        let batch = self.load_batch(&req.batch_id)?;
        let (stale, revision_now) =
            self.with_live_project(&batch.project_id, |_, revision, project| {
                let stale = match batch.status {
                    // Persisted verdicts stand.
                    ChatBatchStatus::Stale => true,
                    ChatBatchStatus::Done | ChatBatchStatus::NotStarted => false,
                    // Live drift: revision moved OR the pinned sources no
                    // longer match (missing identity is drift, never
                    // "in sync" — the M3 honesty rule).
                    ChatBatchStatus::Exported | ChatBatchStatus::Imported => {
                        revision != batch.revision_at_export
                            || match chat_batch_inventory_hash(project, &batch.items) {
                                Some(h) => h != batch.source_hash,
                                None => true,
                            }
                    }
                };
                Ok((stale, revision))
            })?;
        Ok(ChatBatchStatusResponse {
            batch,
            stale,
            revision_now,
        })
    }

    /// `chat_batch_apply`: build the intents from the STORED preview
    /// (never re-parse, never re-resolve — the pinned identities ride
    /// along), stamp `origin = ApplyOrigin::Import`, and go through the ONE
    /// mutating seam ([`ProjectSessionManager::apply`]):
    /// persist-before-ack with the full epoch/revision guard set. A
    /// stale-revision / external-change refusal flips the batch `stale`
    /// (persisted) before the typed error returns. `done` requires
    /// `applied > 0`; a fully-skipped apply leaves `imported` so the user
    /// can inspect the per-intent refusals and retry.
    pub fn chat_batch_apply(
        &self,
        req: &ChatBatchApplyRequest,
    ) -> Result<ChatBatchApplyResponse, ContractError> {
        let mut batch = self.load_batch(&req.batch_id)?;
        // Epoch guard + status gates from the live view.
        self.with_live_project(&batch.project_id, |epoch, _, _| {
            if epoch != req.session_epoch {
                return Err(ContractError::stale_epoch(req.session_epoch, epoch));
            }
            match batch.status {
                ChatBatchStatus::Exported | ChatBatchStatus::Imported => Ok(()),
                ChatBatchStatus::NotStarted => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "chat batch `{}` has no pending preview; import a response first",
                        batch.batch_id
                    ),
                )),
                ChatBatchStatus::Stale => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!(
                        "chat batch `{}` is stale; re-export and re-import before applying",
                        batch.batch_id
                    ),
                )),
                ChatBatchStatus::Done => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!("chat batch `{}` is already applied", batch.batch_id),
                )),
            }
        })?;
        let Some(preview) = batch.preview.clone() else {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "chat batch `{}` has no pending preview; import a response first",
                    batch.batch_id
                ),
            ));
        };

        // Safe-apply partition (mandate §15): clean rows apply, problem
        // rows (live-state conflicts / the model's own needs_review flag)
        // are held back UNLESS the human listed their key explicitly —
        // a problem row is never applied silently.
        let overrides: BTreeSet<&str> = req.override_keys.iter().map(String::as_str).collect();
        for k in &req.override_keys {
            if !preview.items.iter().any(|p| p.key == *k) {
                return Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!("override key `{k}` is not part of the pending preview"),
                ));
            }
        }
        // Map preview rows back onto the pinned identities, in batch order.
        // A preview row whose key is not pinned is impossible by
        // construction (the parser only accepts batch keys); if a future
        // edit breaks that, refuse honestly instead of guessing.
        let by_key: HashMap<&str, &ChatBatchItem> =
            batch.items.iter().map(|i| (i.key.as_str(), i)).collect();
        let mut intents = Vec::with_capacity(preview.items.len());
        let mut held_back = 0usize;
        for p in &preview.items {
            let item = by_key.get(p.key.as_str()).ok_or_else(|| {
                ContractError::new(
                    ContractErrorCode::Internal,
                    format!("preview key `{}` is not pinned in the batch", p.key),
                )
            })?;
            let problem = p.needs_review || p.conflict.is_some() || p.structural_problem.is_some();
            if problem && !overrides.contains(p.key.as_str()) {
                held_back += 1;
                continue;
            }
            intents.push(TranslationIntent {
                entry: item.entry.clone(),
                locale: batch.locale.clone(),
                action: IntentAction::SetTranslation,
                text: Some(p.text.clone()),
            });
        }
        if intents.is_empty() {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "chat batch `{}` has no applicable rows: {} need review/conflict resolution; override explicitly or fix by hand",
                    batch.batch_id, held_back
                ),
            ));
        }

        let apply_req = ApplyIntentsRequest {
            project_id: batch.project_id.clone(),
            expected_revision: req.expected_revision,
            session_epoch: req.session_epoch,
            intents,
            origin: Some(ApplyOrigin::Import),
        };
        match self.apply(&apply_req) {
            Ok(res) => {
                if res.applied > 0 {
                    batch.status = ChatBatchStatus::Done;
                    batch.applied_at_ms = Some(now_ms());
                    batch.applied_revision = Some(res.revision);
                }
                self.save_batch(&batch)?;
                Ok(ChatBatchApplyResponse {
                    job_id: res.job_id,
                    batch,
                    revision: res.revision,
                    applied: res.applied,
                    skipped: res.skipped,
                    cancelled: res.cancelled,
                    held_back,
                })
            }
            Err(e) => {
                // The project moved under the preview between import and
                // apply: the batch is honestly stale (persisted), the
                // preview stays for inspection.
                if matches!(
                    e.code,
                    ContractErrorCode::StaleRevision | ContractErrorCode::ProjectChangedOnDisk
                ) {
                    batch.status = ChatBatchStatus::Stale;
                    let _ = self.save_batch(&batch);
                }
                Err(e)
            }
        }
    }

    /// `chat_batch_list`: every persisted batch of ONE project (oldest
    /// first). Corrupt batch files are skipped here (they cannot join a
    /// workflow); the durable status surface (`chat_batch_status`) still
    /// reports them by id with the typed corruption error.
    pub fn chat_batch_list(&self, project_id: &ProjectId) -> Result<Vec<ChatBatch>, ContractError> {
        self.managed_path(project_id)?;
        let mut out = Vec::new();
        let suffix = format!(".{CHAT_BATCH_EXT}");
        if let Ok(entries) = std::fs::read_dir(self.managed_root()) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !name.ends_with(&suffix) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(batch) = serde_json::from_str::<ChatBatch>(&text) else {
                    continue;
                };
                if batch.project_id == *project_id {
                    out.push(batch);
                }
            }
        }
        out.sort_by(|a, b| {
            (a.created_at_ms, a.batch_id.as_str()).cmp(&(b.created_at_ms, b.batch_id.as_str()))
        });
        Ok(out)
    }

    /// `contract_chat_batch_list`: the batch-history op (mandate §27) over
    /// the same durable list — the typed request/response wrapper the
    /// adapter passes through.
    pub fn chat_batch_list_op(
        &self,
        req: &ChatBatchListRequest,
    ) -> Result<ChatBatchListResponse, ContractError> {
        Ok(ChatBatchListResponse {
            job_id: generate_operation_id(),
            batches: self.chat_batch_list(&req.project_id)?,
        })
    }
}

/// sha256 over the pinned lines in batch order — the deterministic source
/// hash the import gate re-verifies.
pub fn chat_batch_source_hash(items: &[ChatBatchItem]) -> String {
    let mut buf = String::new();
    for i in items {
        buf.push_str(&i.key);
        buf.push('\n');
        buf.push_str(&i.source);
        buf.push('\n');
    }
    sha256_hex(buf.as_bytes())
}

/// The same hash computed against the LIVE inventory: `None` when any
/// pinned identity is missing (drift by definition — never reported as
/// "in sync", the M3 honesty rule).
fn chat_batch_inventory_hash(
    project: &rimloc_domain::canonical::Project,
    items: &[ChatBatchItem],
) -> Option<String> {
    let mut live = Vec::with_capacity(items.len());
    for i in items {
        let entry = project.entries.iter().find(|e| e.id == i.entry)?;
        live.push(ChatBatchItem {
            key: i.key.clone(),
            source: entry.text.clone(),
            entry: i.entry.clone(),
            translation_at_select: i.translation_at_select.clone(),
            origin_at_select: i.origin_at_select.clone(),
        });
    }
    Some(chat_batch_source_hash(&live))
}

/// The structured prompt the user copies into the LLM chat (mandate §6/§7):
/// explicit source AND target languages, the batch's protected tokens as an
/// explicit list, the glossary pairs relevant to these lines, the explicit
/// constraints, and the versioned structured response format. The legacy
/// one-line format stays documented as accepted for compatibility.
pub fn build_chat_prompt(batch: &ChatBatch) -> String {
    let mut p = String::new();
    p.push_str("You are a game localization translator. Translate every source string below.\n\n");
    if let Some(src) = &batch.source_locale {
        p.push_str(&format!("Source language (locale folder): {src}\n"));
    }
    p.push_str(&format!(
        "Target language (locale folder): {}\n",
        batch.locale
    ));
    p.push_str(&format!("Strings: {}\n\n", batch.items.len()));

    // Protected tokens (mandate §6/§7): the explicit per-batch list beats a
    // generic "keep placeholders" line — the model sees exactly what to
    // preserve.
    let mut tokens: BTreeSet<String> = BTreeSet::new();
    for i in &batch.items {
        for tok in rimloc_llm::validator::extract_placeholders(&i.source) {
            tokens.insert(tok);
        }
    }
    if tokens.is_empty() {
        p.push_str(
            "Protected tokens: none detected; still keep XML tags (<i>, <b>, …) intact.\n\n",
        );
    } else {
        p.push_str("Protected tokens — copy these EXACTLY, every occurrence, unchanged:\n");
        for tok in &tokens {
            p.push_str(&format!("- {tok}\n"));
        }
        p.push('\n');
    }

    if !batch.glossary_used.is_empty() {
        p.push_str("Glossary — use EXACTLY these translations for these terms:\n");
        for g in &batch.glossary_used {
            p.push_str(&format!("- {} => {}\n", g.term, g.translation));
        }
        p.push('\n');
    }

    p.push_str("Constraints:\n");
    p.push_str("- Copy each key exactly; never invent, reorder, merge or drop keys.\n");
    p.push_str("- Translate ALL strings; never skip one silently.\n");
    p.push_str("- Preserve every protected token, placeholder and XML tag exactly.\n");
    if !batch.glossary_used.is_empty() {
        p.push_str("- Follow the glossary above.\n");
    }
    p.push_str("- Keep RimWorld grammar constructs intact.\n");
    p.push_str("- If you are NOT confident about a translation, set needs_review to yes and say why in note — do not guess.\n");
    p.push('\n');

    p.push_str("Source strings follow. The key before the \": \" is an identifier — copy it EXACTLY, unchanged, into your answer.\n\n");
    for i in &batch.items {
        p.push_str(&format!("{}{PAIR_DELIM}{}\n", i.key, i.source));
    }

    p.push_str(
        "\nRespond in EXACTLY this versioned format (one block per string, ALL strings, any order):\n\n",
    );
    p.push_str(&format!("{CHAT_RESPONSE_VERSION}\n"));
    p.push_str(&format!(
        "{} <batch id you were given, or omit this line>\n",
        DIR_BATCH
    ));
    p.push_str(&format!(
        "{} <target locale, or omit this line>\n",
        DIR_LOCALE
    ));
    p.push_str(&format!("{DIR_BEGIN}\n"));
    p.push_str(&format!("{} <the key, copied exactly>\n", DIR_KEY));
    p.push_str(&format!("{} no\n", DIR_REVIEW));
    p.push_str("# note: <optional one line — required when needs_review is yes>\n");
    p.push_str(&format!("{DIR_TRANSLATION}\n"));
    p.push_str("<the translation — may span several lines>\n");
    p.push_str(&format!("{DIR_END}\n"));
    p.push_str("# … repeat BEGIN_ENTRY/END_ENTRY for every string, then finish with:\n");
    p.push_str(&format!("{DIR_END_RESPONSE}\n\n"));
    p.push_str("Rules for the response: no commentary, no code fences, no extra lines outside the blocks; the translation text must not contain a line equal to END_ENTRY.\n");
    p.push_str(
        "Compatibility: the older one-line format (`<key>: <translation>` per string) is also accepted.\n",
    );
    p
}

/// Strict-parse failure: a human message plus structured details
/// (line number / offending fragment / reason) for the UI.
struct ChatParseRejection {
    message: String,
    details: serde_json::Value,
}

fn reject(message: String, details: serde_json::Value) -> ChatParseRejection {
    ChatParseRejection { message, details }
}

/// The parse entry (mandate §8/§9 + §30): BOTH formats are accepted.
/// Detection is the first non-blank line — `RIMLOC-CHAT-RESPONSE v1` runs
/// the structured parser (multi-line translations, per-row needs_review +
/// note), anything else runs the legacy one-line parser with its exact
/// historical strictness. Both refuse the WHOLE import on any violation
/// and both normalize to batch order.
fn parse_chat_response(
    text: &str,
    batch: &ChatBatch,
) -> Result<Vec<ChatBatchPreviewItem>, ChatParseRejection> {
    // Responses pasted from terminals can carry a leading UTF-8 BOM; it is
    // transport noise, not content — strip it once instead of failing the
    // first line for an invisible character.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first.starts_with(CHAT_RESPONSE_FAMILY) {
        parse_structured_response(text, batch)
    } else {
        parse_legacy_response(text, batch)
    }
}

/// The legacy STRICT parser: one `<key>: <translation>` line per batch
/// item. Blank lines are ignored; EVERY other line must parse against a
/// known batch key. Refusals (whole import, never partial):
/// - malformed — a non-blank line without the `<known-key>: <text>` shape,
///   or an empty translation;
/// - unknown/extra — a well-shaped line whose key is not in the batch;
/// - duplicate — one key answered twice;
/// - missing — a batch key the response never answered.
fn parse_legacy_response(
    text: &str,
    batch: &ChatBatch,
) -> Result<Vec<ChatBatchPreviewItem>, ChatParseRejection> {
    let keys: BTreeSet<&str> = batch.items.iter().map(|i| i.key.as_str()).collect();
    let mut parsed: Vec<ChatBatchPreviewItem> = Vec::new();
    let mut answered: BTreeSet<&str> = BTreeSet::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim_end_matches(['\r', ' ']);
        if line.trim().is_empty() {
            continue;
        }
        // Keys never contain a colon (`display_identity` uses `·`), so the
        // FIRST colon is the pair boundary; the prompt prescribes
        // `<key>: <translation>`, a missing space after the colon is the
        // same pair (no ambiguity is possible), and `key:` with nothing
        // after it is the empty-translation refusal — not a shape error.
        let Some((raw_key, rest)) = line.split_once(':') else {
            return Err(reject(
                format!("malformed line {line_no}: expected `<key>: <translation>`"),
                serde_json::json!({ "line": line_no, "line_text": raw, "reason": "malformed" }),
            ));
        };
        let key = raw_key.trim();
        if !keys.contains(key) {
            return Err(reject(
                format!("unknown key on line {line_no}: `{key}` is not part of this batch"),
                serde_json::json!({ "line": line_no, "key": key, "reason": "unknown_key" }),
            ));
        }
        let translation = rest.trim();
        if translation.is_empty() {
            return Err(reject(
                format!("empty translation for `{key}` on line {line_no}"),
                serde_json::json!({ "line": line_no, "key": key, "reason": "empty_translation" }),
            ));
        }
        if !answered.insert(key) {
            return Err(reject(
                format!("duplicate key on line {line_no}: `{key}` answered more than once"),
                serde_json::json!({ "line": line_no, "key": key, "reason": "duplicate" }),
            ));
        }
        parsed.push(ChatBatchPreviewItem {
            key: key.to_string(),
            text: translation.to_string(),
            note: None,
            needs_review: false,
            conflict: None,
            structural_problem: None,
        });
    }
    if answered.len() != keys.len() {
        let missing: Vec<&str> = keys.difference(&answered).copied().collect();
        return Err(reject(
            format!(
                "incomplete response: {} of {} keys answered; missing: {}",
                answered.len(),
                keys.len(),
                missing.join(", ")
            ),
            serde_json::json!({ "reason": "missing", "missing": missing }),
        ));
    }
    // Batch order (deterministic preview + deterministic apply order), not
    // response order.
    let mut ordered = Vec::with_capacity(parsed.len());
    for item in &batch.items {
        let pos = parsed
            .iter()
            .position(|p| p.key == item.key)
            .expect("parser guarantees every batch key is answered");
        ordered.push(parsed.swap_remove(pos));
    }
    Ok(ordered)
}

/// The versioned structured parser (format v1, see [`build_chat_prompt`]):
///
/// ```text
/// RIMLOC-CHAT-RESPONSE v1
/// batch: <batch-id>            (optional; when present must match)
/// locale: <locale>             (optional; when present must match)
/// BEGIN_ENTRY
/// key: <known batch key>
/// needs_review: yes|no         (optional, default no)
/// note: <one line>             (optional)
/// translation:
/// <raw translation lines — may span several lines, interior blanks kept>
/// END_ENTRY
/// … one block per key …
/// END_RESPONSE                 (optional terminator)
/// ```
///
/// The same semantic strictness as the legacy parser (unknown key,
/// duplicate, missing, empty translation = whole-import refusal) plus the
/// header/version gate: a foreign `RIMLOC-CHAT-RESPONSE <ver>` is a typed
/// refusal naming the unsupported version, never a guess. A wrong
/// `batch:`/`locale:` line is refused (mandate §11: wrong batch id / wrong
/// target locale).
fn parse_structured_response(
    text: &str,
    batch: &ChatBatch,
) -> Result<Vec<ChatBatchPreviewItem>, ChatParseRejection> {
    let mut lines = text.lines().peekable();
    // Header gate: exact version. The dispatcher guarantees the family
    // prefix; here the VERSION is checked and anything else refuses.
    let mut line_no = 0usize;
    for raw in lines.by_ref() {
        line_no += 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line != CHAT_RESPONSE_VERSION {
            let version = line.strip_prefix(CHAT_RESPONSE_FAMILY).unwrap_or("").trim();
            return Err(reject(
                format!(
                    "unsupported response format version `{version}` on line {line_no}: this build speaks `{CHAT_RESPONSE_VERSION}`"
                ),
                serde_json::json!({ "line": line_no, "reason": "unsupported_version", "version": version }),
            ));
        }
        break;
    }

    let keys: BTreeSet<&str> = batch.items.iter().map(|i| i.key.as_str()).collect();
    let mut answered: BTreeSet<String> = BTreeSet::new();
    let mut parsed: Vec<ChatBatchPreviewItem> = Vec::new();
    // Block under construction.
    let mut cur_key: Option<String> = None;
    let mut cur_review = false;
    let mut cur_note: Option<String> = None;
    let mut cur_text: Vec<String> = Vec::new();
    let mut in_translation = false;
    let mut ended = false;

    macro_rules! finish_block {
        () => {{
            let Some(key) = cur_key.take() else {
                return Err(reject(
                    format!("malformed block ending on line {line_no}: no `key:` line inside BEGIN_ENTRY"),
                    serde_json::json!({ "line": line_no, "reason": "malformed_block" }),
                ));
            };
            if !keys.contains(key.as_str()) {
                return Err(reject(
                    format!("unknown key `{key}`: not part of this batch"),
                    serde_json::json!({ "key": key, "reason": "unknown_key" }),
                ));
            }
            if !answered.insert(key.clone()) {
                return Err(reject(
                    format!("duplicate key `{key}`: answered more than once"),
                    serde_json::json!({ "key": key, "reason": "duplicate" }),
                ));
            }
            while cur_text.first().is_some_and(|l| l.trim().is_empty()) {
                cur_text.remove(0);
            }
            while cur_text.last().is_some_and(|l| l.trim().is_empty()) {
                cur_text.pop();
            }
            if cur_text.is_empty() {
                return Err(reject(
                    format!("empty translation for `{key}` (block closed on line {line_no})"),
                    serde_json::json!({ "line": line_no, "key": key, "reason": "empty_translation" }),
                ));
            }
            parsed.push(ChatBatchPreviewItem {
                key,
                text: cur_text.join("\n"),
                note: cur_note.take(),
                needs_review: cur_review,
                conflict: None,
                structural_problem: None,
            });
            cur_review = false;
            cur_text.clear();
            in_translation = false;
        }};
    }

    for raw in lines {
        line_no += 1;
        let line = raw.trim_end_matches(['\r', ' ']);
        if in_translation {
            // RAW text mode: everything until END_ENTRY is content.
            if line.trim() == DIR_END {
                finish_block!();
            } else {
                cur_text.push(line.to_string());
            }
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if ended {
            return Err(reject(
                format!("malformed line {line_no}: content after {DIR_END_RESPONSE}"),
                serde_json::json!({ "line": line_no, "reason": "malformed" }),
            ));
        }
        if trimmed == DIR_BEGIN {
            if cur_key.is_some() {
                return Err(reject(
                    format!("malformed line {line_no}: nested BEGIN_ENTRY without END_ENTRY"),
                    serde_json::json!({ "line": line_no, "reason": "malformed" }),
                ));
            }
            continue;
        }
        if trimmed == DIR_END {
            finish_block!();
            continue;
        }
        if trimmed == DIR_END_RESPONSE {
            ended = true;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(DIR_KEY) {
            if cur_key.is_some() {
                return Err(reject(
                    format!("malformed line {line_no}: duplicate `key:` in one block"),
                    serde_json::json!({ "line": line_no, "reason": "malformed" }),
                ));
            }
            let key = rest.trim();
            if key.is_empty() {
                return Err(reject(
                    format!("malformed line {line_no}: empty key"),
                    serde_json::json!({ "line": line_no, "reason": "malformed" }),
                ));
            }
            if !keys.contains(key) {
                return Err(reject(
                    format!("unknown key `{key}`: not part of this batch"),
                    serde_json::json!({ "line": line_no, "key": key, "reason": "unknown_key" }),
                ));
            }
            cur_key = Some(key.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(DIR_REVIEW) {
            match rest.trim().to_ascii_lowercase().as_str() {
                "yes" => cur_review = true,
                "no" => cur_review = false,
                other => {
                    return Err(reject(
                        format!(
                            "malformed line {line_no}: needs_review must be yes|no, got `{other}`"
                        ),
                        serde_json::json!({ "line": line_no, "reason": "malformed" }),
                    ))
                }
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(DIR_NOTE) {
            if cur_note.is_some() {
                return Err(reject(
                    format!("malformed line {line_no}: duplicate `note:` in one block"),
                    serde_json::json!({ "line": line_no, "reason": "malformed" }),
                ));
            }
            cur_note = Some(rest.trim().to_string());
            continue;
        }
        if trimmed == DIR_TRANSLATION {
            in_translation = true;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(DIR_BATCH) {
            let id = rest.trim();
            if !id.is_empty() && id != batch.batch_id {
                return Err(reject(
                    format!(
                        "wrong batch id on line {line_no}: `{id}` does not match this batch (`{}`)",
                        batch.batch_id
                    ),
                    serde_json::json!({ "line": line_no, "reason": "wrong_batch" }),
                ));
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(DIR_LOCALE) {
            let loc = rest.trim();
            if !loc.is_empty() && loc != batch.locale {
                return Err(reject(
                    format!(
                        "wrong target locale on line {line_no}: `{loc}` does not match the batch locale (`{}`)",
                        batch.locale
                    ),
                    serde_json::json!({ "line": line_no, "reason": "wrong_locale" }),
                ));
            }
            continue;
        }
        return Err(reject(
            format!("malformed line {line_no}: unrecognized content outside a translation block"),
            serde_json::json!({ "line": line_no, "line_text": raw, "reason": "malformed" }),
        ));
    }
    if cur_key.is_some() || in_translation {
        return Err(reject(
            format!(
                "malformed response: the last BEGIN_ENTRY block was never closed with {DIR_END}"
            ),
            serde_json::json!({ "reason": "unclosed_block" }),
        ));
    }
    if answered.len() != keys.len() {
        let missing: Vec<String> = keys
            .iter()
            .filter(|k| !answered.contains(**k))
            .map(|k| k.to_string())
            .collect();
        return Err(reject(
            format!(
                "incomplete response: {} of {} keys answered; missing: {}",
                answered.len(),
                keys.len(),
                missing.join(", ")
            ),
            serde_json::json!({ "reason": "missing", "missing": missing }),
        ));
    }
    // Batch order (deterministic preview + deterministic apply order), not
    // response order.
    let mut ordered = Vec::with_capacity(parsed.len());
    for item in &batch.items {
        let pos = parsed
            .iter()
            .position(|p| p.key == item.key)
            .expect("parser guarantees every batch key is answered");
        ordered.push(parsed.swap_remove(pos));
    }
    Ok(ordered)
}

/// The per-row conflict gate (mandate §12 hardened): compare each parsed
/// row against the SELECTION-time snapshot carried on the batch item.
///
/// - live text equals the proposed text → no conflict (nothing would
///   change);
/// - live text differs from `translation_at_select` →
///   `changed_after_select` — the row's translation moved since the batch
///   was pinned (a human fix through any path; re-exports do NOT launder
///   the snapshot, it is taken once at create);
/// - live text equals the snapshot but the snapshot origin is `human` and
///   the response proposes something else → `human_translation` — the row
///   already carried human work at selection time.
///
/// Items WITHOUT a selection snapshot (`None` — batch files written before
/// this gate) can never be claimed conflicting — honest absence, not a
/// guess. A cleared translation (live `None` after a snapshot with text)
/// leaves nothing human to overwrite → no conflict.
fn attach_conflicts(
    batch: &ChatBatch,
    mut items: Vec<ChatBatchPreviewItem>,
    live: &[(String, Option<(String, String)>)],
) -> Vec<ChatBatchPreviewItem> {
    let live_by_key: HashMap<&str, Option<&(String, String)>> =
        live.iter().map(|(k, v)| (k.as_str(), v.as_ref())).collect();
    let items_by_key: HashMap<&str, &ChatBatchItem> =
        batch.items.iter().map(|i| (i.key.as_str(), i)).collect();
    for p in &mut items {
        let Some(snapshot_text) = items_by_key
            .get(p.key.as_str())
            .and_then(|i| i.translation_at_select.clone())
        else {
            continue;
        };
        let Some((current_text, current_origin)) =
            live_by_key.get(p.key.as_str()).copied().flatten()
        else {
            continue;
        };
        if current_text.trim() == p.text.trim() {
            continue;
        }
        let reason = if *current_text != snapshot_text {
            "changed_after_select"
        } else if current_origin == "human" {
            "human_translation"
        } else {
            continue;
        };
        p.conflict = Some(crate::contract::ChatRowConflict {
            reason: reason.to_string(),
            current_text: current_text.clone(),
            current_origin: current_origin.clone(),
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ChatBatchExportRequest, ChatBatchImportRequest, ChatBatchStatusRequest};
    use crate::project_store::load_project_with_meta;
    use rimloc_domain::canonical::{EntryKind, Origin, SourceEntryId};
    use std::path::Path;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// A two-DefType mod (same key across types — the identity-vs-key
    /// distinction the batch must never blur) plus Keyed entries.
    fn test_mod(root: &Path) {
        write(
            &root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
        );
        write(
            &root.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability label</label></AbilityDef></Defs>"#,
        );
        write(
            &root.join("Languages/English/Keyed/Words.xml"),
            r#"<RimWorld-Strings><SaveGame>Save game</SaveGame><LoadGame>Load game</LoadGame></RimWorld-Strings>"#,
        );
    }

    fn setup() -> (
        tempfile::TempDir,
        ProjectSessionManager,
        crate::contract::ProjectSnapshot,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        test_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        (dir, mgr, snap)
    }

    fn keyed_key(key: &str) -> String {
        SourceEntryId {
            kind: EntryKind::Keyed,
            key: key.into(),
            def_type: None,
        }
        .display_identity()
    }

    fn definjected_key(def_type: &str, key: &str) -> String {
        SourceEntryId {
            kind: EntryKind::DefInjected,
            key: key.into(),
            def_type: Some(def_type.into()),
        }
        .display_identity()
    }

    fn create_req(pid: &str, epoch: u64, keys: &[String]) -> ChatBatchCreateRequest {
        ChatBatchCreateRequest {
            project_id: pid.into(),
            session_epoch: epoch,
            locale: "Russian".into(),
            entry_keys: keys.to_vec(),
        }
    }

    fn export(mgr: &ProjectSessionManager, batch_id: &str, epoch: u64) -> ChatBatchExportResponse {
        mgr.chat_batch_export(&ChatBatchExportRequest {
            batch_id: batch_id.into(),
            session_epoch: epoch,
        })
        .unwrap()
    }

    fn import(
        mgr: &ProjectSessionManager,
        batch_id: &str,
        epoch: u64,
        text: &str,
    ) -> Result<ChatBatchImportResponse, ContractError> {
        mgr.chat_batch_import(&ChatBatchImportRequest {
            batch_id: batch_id.into(),
            session_epoch: epoch,
            response_text: text.into(),
        })
    }

    fn status(mgr: &ProjectSessionManager, batch_id: &str) -> ChatBatchStatusResponse {
        mgr.chat_batch_status(&ChatBatchStatusRequest {
            batch_id: batch_id.into(),
        })
        .unwrap()
    }

    // ---------- round-trip: create → export → import → apply ----------

    #[test]
    fn chat_batch_full_cycle_round_trip_persists_and_applies_as_import() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let k_thing = definjected_key("ThingDef", "Dup.label");

        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_thing.clone(), k_save.clone(), k_load.clone()],
            ))
            .unwrap();
        assert_eq!(batch.items.len(), 3);
        assert_eq!(batch.status, ChatBatchStatus::NotStarted);
        // Identities pinned structurally — the DefInjected line carries its
        // discriminator; the same-key AbilityDef stays out.
        assert_eq!(batch.items[0].entry.def_type.as_deref(), Some("ThingDef"));

        // Durable at create.
        assert!(mgr.chat_batch_path(&batch.batch_id).unwrap().is_file());
        assert_eq!(mgr.load_batch(&batch.batch_id).unwrap(), batch);

        // Export records the gates and builds a parseable prompt.
        let exported = export(&mgr, &batch.batch_id, snap.session_epoch);
        assert_eq!(exported.batch.status, ChatBatchStatus::Exported);
        assert_eq!(exported.batch.revision_at_export, snap.revision);
        for k in [&k_thing, &k_save, &k_load] {
            assert!(exported.prompt.contains(&format!("{k}: ")), "{k} in prompt");
        }
        assert!(exported.prompt.contains("Save game"));

        // The user pastes the response (order shuffled + blank lines +
        // trailing newline — the parser normalizes to batch order).
        let response = format!(
            "\n{k_save}: Сохранить игру\n\n{k_thing}: метка предмета\n{k_load}: Загрузить игру\n"
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        assert_eq!(imported.batch.status, ChatBatchStatus::Imported);
        assert_eq!(imported.preview.items.len(), 3);
        // Batch order, not response order.
        assert_eq!(imported.preview.items[0].key, k_thing);
        assert_eq!(imported.preview.items[0].text, "метка предмета");

        // Status reads the durable record; not stale at the export revision.
        let status = status(&mgr, &batch.batch_id);
        assert!(!status.stale);
        assert_eq!(status.revision_now, snap.revision);

        // Apply through the session seam: origin=import lands in the
        // durable project, persist-before-ack.
        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 3);
        assert_eq!(applied.batch.status, ChatBatchStatus::Done);
        assert_eq!(applied.revision, snap.revision + 1);

        // Durable: the batch file says done, the project carries the
        // translations with origin=imported and the acked revision moved.
        let done = mgr.load_batch(&batch.batch_id).unwrap();
        assert_eq!(done.status, ChatBatchStatus::Done);
        assert_eq!(done.applied_revision, Some(snap.revision + 1));
        let env = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        assert_eq!(env.meta.revision, Some(snap.revision + 1));
        let ru = env
            .project
            .translations
            .iter()
            .find(|t| t.locale == "Russian" && t.source_id.key == "SaveGame")
            .expect("SaveGame translation applied");
        assert_eq!(ru.text.as_deref(), Some("Сохранить игру"));
        assert_eq!(ru.origin, Origin::Imported);

        // Terminal: further imports refuse.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: x"),
        )
        .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);
        assert!(err.message.contains("already applied"), "{err}");
    }

    // ---------- rejection matrix ----------

    #[test]
    fn chat_batch_rejects_malformed_unknown_duplicate_missing_and_empty() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_load.clone()],
            ))
            .unwrap();
        export(&mgr, &batch.batch_id, snap.session_epoch);

        // Malformed: a prose line WITHOUT the pair shape the parser
        // forbids forgiving (a colon-terminated prose line reports the
        // more helpful `unknown key` instead).
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!(
                "Sure, here are the translations\n{k_save}: Сохранить игру\n{k_load}: Загрузить игру"
            ),
        )
        .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);
        assert!(err.message.contains("malformed line 1"), "{err}");

        // A colon-terminated prose line is an unknown-key refusal.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!(
                "Sure! Here are the translations:\n{k_save}: Сохранить игру\n{k_load}: Загрузить игру"
            ),
        )
        .unwrap_err();
        assert!(err.message.contains("unknown key"), "{err}");

        // Unknown/extra key.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: Сохранить игру\n{k_load}: Загрузить игру\nkeyed·Nope: лишний"),
        )
        .unwrap_err();
        assert!(err.message.contains("unknown key"), "{err}");

        // Duplicate key.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: Сохранить игру\n{k_save}: Ещё раз\n{k_load}: Загрузить игру"),
        )
        .unwrap_err();
        assert!(err.message.contains("duplicate"), "{err}");

        // Missing key (partial response).
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: Сохранить игру"),
        )
        .unwrap_err();
        assert!(err.message.contains("missing"), "{err}");

        // Empty translation.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: \n{k_load}: Загрузить игру"),
        )
        .unwrap_err();
        assert!(err.message.contains("empty translation"), "{err}");

        // The batch stays `exported` after refusals — the user re-pastes.
        let after = mgr.load_batch(&batch.batch_id).unwrap();
        assert_eq!(after.status, ChatBatchStatus::Exported);
        assert!(after.preview.is_none());
    }

    #[test]
    fn chat_batch_rejects_stale_revision_and_bad_selection_and_import_before_export() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        export(&mgr, &batch.batch_id, snap.session_epoch);

        // The project moves under the batch (a manual TM write acks a new
        // revision).
        mgr.tm_upsert(&crate::contract::TmUpsertRequest {
            project_id: snap.project_id.clone(),
            session_epoch: snap.session_epoch,
            source_text: "Save game".into(),
            target_text: "Сохранить игру".into(),
            target_locale: "Russian".into(),
            status: None,
        })
        .unwrap();

        // The status surface reports drift WITHOUT a refused op.
        let live = status(&mgr, &batch.batch_id);
        assert!(live.stale);
        assert_eq!(live.revision_now, snap.revision + 1);

        // The import refuses AND persists the stale status.
        let err = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: Сохранить игру"),
        )
        .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::StaleRevision);
        let staled = mgr.load_batch(&batch.batch_id).unwrap();
        assert_eq!(staled.status, ChatBatchStatus::Stale);
        assert!(status(&mgr, &batch.batch_id).stale);

        // Re-export is the recovery path (it re-records the gates).
        let re = export(&mgr, &batch.batch_id, snap.session_epoch);
        assert_eq!(re.batch.status, ChatBatchStatus::Exported);
        assert_eq!(re.batch.revision_at_export, snap.revision + 1);
        let imported = import(
            &mgr,
            &batch.batch_id,
            snap.session_epoch,
            &format!("{k_save}: Сохранить игру"),
        )
        .unwrap();
        assert_eq!(imported.batch.status, ChatBatchStatus::Imported);

        // Apply with the STALE revision the caller cached → refused by the
        // session guard, batch flips stale (persisted).
        let err = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::StaleRevision);
        assert_eq!(
            mgr.load_batch(&batch.batch_id).unwrap().status,
            ChatBatchStatus::Stale
        );

        // ---- selection-side strictness: unknown, duplicate, empty.
        let err = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &["keyed·Нет".into()],
            ))
            .unwrap_err();
        assert!(err.message.contains("not found in the inventory"), "{err}");
        let err = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_save.clone()],
            ))
            .unwrap_err();
        assert!(err.message.contains("duplicate entry key"), "{err}");
        let err = mgr
            .chat_batch_create(&create_req(&snap.project_id, snap.session_epoch, &[]))
            .unwrap_err();
        assert!(err.message.contains("at least one"), "{err}");

        // Import before export.
        let fresh = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        let err = import(
            &mgr,
            &fresh.batch_id,
            snap.session_epoch,
            &format!("{k_save}: x"),
        )
        .unwrap_err();
        assert!(err.message.contains("never exported"), "{err}");

        // List surface: two batches for this project.
        assert_eq!(mgr.chat_batch_list(&snap.project_id).unwrap().len(), 2);
    }

    /// The batch is scoped to its owning session: a foreign manager does
    /// not see the project (wrong-project rejection at the seam), and ids
    /// are form-checked before any filesystem access.
    #[test]
    fn chat_batch_wrong_project_and_malformed_ids_refuse() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();

        // A fresh manager over ANOTHER root knows no project.
        let dir2 = tempfile::tempdir().unwrap();
        let mgr2 = ProjectSessionManager::new(dir2.path().join("managed")).unwrap();
        let err = mgr2
            .chat_batch_status(&ChatBatchStatusRequest {
                batch_id: batch.batch_id.clone(),
            })
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ProjectNotFound);

        // Malformed ids never touch the filesystem.
        for bad in ["../escape", "proj-xyz", "batch_underscore", ""] {
            let err = mgr
                .chat_batch_status(&ChatBatchStatusRequest {
                    batch_id: bad.into(),
                })
                .unwrap_err();
            assert_eq!(
                err.code,
                ContractErrorCode::ContractViolation,
                "{bad}: {err}"
            );
        }
    }

    /// Persistence across manager restarts: a NEW manager over the SAME
    /// managed root (after project_open) sees the batch and completes the
    /// cycle on the restarted session.
    #[test]
    fn chat_batch_state_persists_across_restart() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        export(&mgr, &batch.batch_id, snap.session_epoch);

        let root = mgr.managed_root().to_path_buf();
        let mgr2 = ProjectSessionManager::new(&root).unwrap();
        let snap2 = mgr2.open(&snap.project_id).unwrap();
        let status = status(&mgr2, &batch.batch_id);
        assert_eq!(status.batch.status, ChatBatchStatus::Exported);
        assert!(!status.stale);

        let imported = import(
            &mgr2,
            &batch.batch_id,
            snap2.session_epoch,
            &format!("{k_save}: Сохранить игру"),
        )
        .unwrap();
        assert_eq!(imported.batch.status, ChatBatchStatus::Imported);
        let applied = mgr2
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap2.revision,
                session_epoch: snap2.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 1);
        assert_eq!(applied.batch.status, ChatBatchStatus::Done);
    }

    /// The exported prompt's pair lines round-trip through the exact line
    /// shape the prompt prescribes (a compliant answer parses clean).
    #[test]
    fn chat_prompt_format_round_trips_through_the_parser() {
        let (_dir, mgr, snap) = setup();
        let keys: Vec<String> = vec![keyed_key("SaveGame"), keyed_key("LoadGame")];
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &keys.clone(),
            ))
            .unwrap();
        let exported = export(&mgr, &batch.batch_id, snap.session_epoch);
        // Simulate a compliant answer built from the prompt's own pair
        // lines (a lazy model echoing the prompt parses clean).
        let pairs: Vec<&str> = exported
            .prompt
            .lines()
            .filter(|l| {
                keys.iter()
                    .any(|k| l.starts_with(k.as_str()) && l.contains(PAIR_DELIM))
            })
            .collect();
        assert_eq!(pairs.len(), 2);
        let imported =
            import(&mgr, &batch.batch_id, snap.session_epoch, &pairs.join("\n")).unwrap();
        assert_eq!(imported.preview.items.len(), 2);
        assert!(imported.preview.items.iter().all(|p| !p.text.is_empty()));
        // And a BOM-prefixed paste (transport noise) parses identically.
        let bom_response = format!("\u{feff}{}", pairs.join("\n"));
        let again = import(&mgr, &batch.batch_id, snap.session_epoch, &bom_response).unwrap();
        assert_eq!(again.preview.items, imported.preview.items);
    }

    /// The source-hash gate catches a same-revision source drift that
    /// slips past the revision check (defense in depth, mandate's
    /// "source hash check").
    #[test]
    fn chat_batch_source_hash_gate_catches_inventory_drift() {
        let items = vec![ChatBatchItem {
            key: "keyed·A".into(),
            source: "A".into(),
            entry: SourceEntryId {
                kind: EntryKind::Keyed,
                key: "A".into(),
                def_type: None,
            },
            translation_at_select: None,
            origin_at_select: None,
        }];
        let h1 = chat_batch_source_hash(&items);
        let mut drifted = items.clone();
        drifted[0].source = "A2".into();
        assert_ne!(h1, chat_batch_source_hash(&drifted));
    }

    // ---------- structured response format v1 ----------

    /// Build a compliant v1 structured response for the given (key, text)
    /// pairs — mirrors what a well-behaved model sends back.
    fn structured_response(batch_id: &str, locale: &str, rows: &[(&str, &str)]) -> String {
        let mut s = String::new();
        s.push_str(&format!("{CHAT_RESPONSE_VERSION}\n"));
        s.push_str(&format!("{DIR_BATCH} {batch_id}\n"));
        s.push_str(&format!("{DIR_LOCALE} {locale}\n"));
        for (k, v) in rows {
            s.push_str(&format!(
                "{DIR_BEGIN}\n{DIR_KEY} {k}\n{DIR_REVIEW} no\n{DIR_TRANSLATION}\n{v}\n{DIR_END}\n"
            ));
        }
        s.push_str(&format!("{DIR_END_RESPONSE}\n"));
        s
    }

    #[test]
    fn chat_batch_structured_format_round_trips_multiline_and_review_flags() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_load.clone()],
            ))
            .unwrap();
        let exported = export(&mgr, &batch.batch_id, snap.session_epoch);
        // The prompt teaches the structured format and states the languages.
        assert!(exported.prompt.contains(CHAT_RESPONSE_VERSION));
        assert!(exported
            .prompt
            .contains("Target language (locale folder): Russian"));
        assert!(exported
            .prompt
            .contains("Source language (locale folder): en"));

        // Multi-line translation + a needs_review row with a note.
        let multiline = "Сохранить игру\n(вторая строка подсказки)";
        let response = format!(
            "{CHAT_RESPONSE_VERSION}\n{DIR_BATCH} {}\n{DIR_LOCALE} Russian\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_REVIEW} no\n{DIR_TRANSLATION}\n{multiline}\n{DIR_END}\n{DIR_BEGIN}\n{DIR_KEY} {k_load}\n{DIR_REVIEW} yes\n{DIR_NOTE} неоднозначный термин\n{DIR_TRANSLATION}\nЗагрузить игру\n{DIR_END}\n{DIR_END_RESPONSE}\n",
            batch.batch_id
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        assert_eq!(imported.preview.items.len(), 2);
        // Batch order, multi-line text preserved verbatim.
        assert_eq!(imported.preview.items[0].key, k_save);
        assert_eq!(imported.preview.items[0].text, multiline);
        assert!(!imported.preview.items[0].needs_review);
        assert_eq!(imported.preview.items[1].key, k_load);
        assert_eq!(imported.preview.items[1].text, "Загрузить игру");
        assert!(imported.preview.items[1].needs_review);
        assert_eq!(
            imported.preview.items[1].note.as_deref(),
            Some("неоднозначный термин")
        );

        // needs_review rows are HELD BACK by default: 1 applicable row of 2.
        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 1);
        assert_eq!(applied.held_back, 1);
        // The held-back row never landed.
        let env = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        let load_tr = env
            .project
            .translations
            .iter()
            .find(|t| t.locale == "Russian" && t.source_id.key == "LoadGame");
        assert!(
            load_tr.is_none(),
            "needs_review row must not apply silently"
        );

        // An explicit override applies it (the human decided).
        let batch2 = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_load),
            ))
            .unwrap();
        export(&mgr, &batch2.batch_id, snap.session_epoch);
        let response2 = structured_response(
            &batch2.batch_id,
            "Russian",
            &[(k_load.as_str(), "Загрузить игру")],
        );
        import(&mgr, &batch2.batch_id, snap.session_epoch, &response2).unwrap();
        let applied2 = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch2.batch_id.clone(),
                expected_revision: load_project_with_meta(
                    &mgr.managed_path(&snap.project_id).unwrap(),
                )
                .unwrap()
                .meta
                .revision
                .unwrap(),
                session_epoch: snap.session_epoch,
                override_keys: vec![k_load.clone()],
            })
            .unwrap();
        assert_eq!(applied2.applied, 1);
        assert_eq!(applied2.held_back, 0);
    }

    #[test]
    fn chat_batch_structured_format_rejects_every_violation_class() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_load.clone()],
            ))
            .unwrap();
        let bid = batch.batch_id.clone();
        export(&mgr, &bid, snap.session_epoch);
        let import_err = |text: String| import(&mgr, &bid, snap.session_epoch, &text).unwrap_err();

        // Foreign version of the family → named refusal, never a guess.
        let err = import_err(format!(
            "RIMLOC-CHAT-RESPONSE v999\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_TRANSLATION}\nx\n{DIR_END}\n"
        ));
        assert!(
            err.message.contains("unsupported response format version"),
            "{err}"
        );

        // Wrong batch id.
        let err = import_err(structured_response(
            "batch-someone-elses",
            "Russian",
            &[
                (k_save.as_str(), "Сохранить игру"),
                (k_load.as_str(), "Загрузить игру"),
            ],
        ));
        assert!(err.message.contains("wrong batch id"), "{err}");

        // Wrong target locale (mandate §11).
        let err = import_err(structured_response(
            &bid,
            "Polish",
            &[
                (k_save.as_str(), "Сохранить игру"),
                (k_load.as_str(), "Загрузить игру"),
            ],
        ));
        assert!(err.message.contains("wrong target locale"), "{err}");

        // Unknown key inside a block.
        let err = import_err(structured_response(
            &bid,
            "Russian",
            &[
                (k_save.as_str(), "Сохранить игру"),
                ("keyed·Nope", "лишний"),
            ],
        ));
        assert!(err.message.contains("unknown key"), "{err}");

        // Duplicate block.
        let err = import_err(structured_response(
            &bid,
            "Russian",
            &[
                (k_save.as_str(), "Сохранить игру"),
                (k_save.as_str(), "Ещё раз"),
            ],
        ));
        assert!(err.message.contains("duplicate"), "{err}");

        // Missing key.
        let err = import_err(structured_response(
            &bid,
            "Russian",
            &[(k_save.as_str(), "Сохранить игру")],
        ));
        assert!(err.message.contains("missing"), "{err}");

        // Empty translation (block closed with no text).
        let err = import_err(format!(
            "{CHAT_RESPONSE_VERSION}\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_TRANSLATION}\n{DIR_END}\n{DIR_BEGIN}\n{DIR_KEY} {k_load}\n{DIR_TRANSLATION}\nЗагрузить игру\n{DIR_END}\n"
        ));
        assert!(err.message.contains("empty translation"), "{err}");

        // Unclosed final block.
        let err = import_err(format!(
            "{CHAT_RESPONSE_VERSION}\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_TRANSLATION}\nСохранить игру\n{DIR_END}\n{DIR_BEGIN}\n{DIR_KEY} {k_load}\n{DIR_TRANSLATION}\nЗагрузить игру\n"
        ));
        assert!(err.message.contains("never closed"), "{err}");

        // Content after END_RESPONSE.
        let err = import_err(format!(
            "{}\nпосле конца — мусор\n",
            structured_response(
                &bid,
                "Russian",
                &[
                    (k_save.as_str(), "Сохранить игру"),
                    (k_load.as_str(), "Загрузить игру")
                ]
            )
            .trim_end()
        ));
        assert!(err.message.contains("content after"), "{err}");

        // Commentary outside a block.
        let err = import_err(format!(
            "{CHAT_RESPONSE_VERSION}\nSure, here you go!\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_TRANSLATION}\nСохранить игру\n{DIR_END}\n{DIR_BEGIN}\n{DIR_KEY} {k_load}\n{DIR_TRANSLATION}\nЗагрузить игру\n{DIR_END}\n"
        ));
        assert!(err.message.contains("malformed"), "{err}");

        // The batch stays `exported` after refusals — re-paste is legal.
        assert_eq!(
            mgr.load_batch(&bid).unwrap().status,
            ChatBatchStatus::Exported
        );
    }

    /// Mandate §30 corpus: reordered entries and a BOM prefix parse clean in
    /// the structured format too.
    #[test]
    fn chat_batch_structured_accepts_reorder_and_bom() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_load.clone()],
            ))
            .unwrap();
        export(&mgr, &batch.batch_id, snap.session_epoch);
        let response = format!(
            "\u{feff}{CHAT_RESPONSE_VERSION}\n{DIR_BEGIN}\n{DIR_KEY} {k_load}\n{DIR_TRANSLATION}\nЗагрузить игру\n{DIR_END}\n{DIR_BEGIN}\n{DIR_KEY} {k_save}\n{DIR_TRANSLATION}\nСохранить игру\n{DIR_END}\n{DIR_END_RESPONSE}\n"
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        assert_eq!(imported.preview.items[0].key, k_save);
        assert_eq!(imported.preview.items[1].key, k_load);
    }

    // ---------- large selections, XML values, protected tokens ----------

    /// A 220-string selection round-trips create → export → structured
    /// import → apply at full fidelity (mandate: large projects, no loss).
    #[test]
    fn chat_batch_220_string_selection_round_trips_and_applies() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        // Keyed XML with 220 entries (some carrying XML-heavy values).
        let mut keyed = String::from("<RimWorld-Strings>");
        for i in 0..220 {
            keyed.push_str(&format!(
                "<Entry{i}>Source string {i} with {{0}} and %s</Entry{i}>"
            ));
        }
        keyed.push_str("</RimWorld-Strings>");
        write(&mod_root.join("Languages/English/Keyed/Big.xml"), &keyed);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let keys: Vec<String> = (0..220).map(|i| keyed_key(&format!("Entry{i}"))).collect();
        let batch = mgr
            .chat_batch_create(&create_req(&snap.project_id, snap.session_epoch, &keys))
            .unwrap();
        assert_eq!(batch.items.len(), 220);
        let exported = export(&mgr, &batch.batch_id, snap.session_epoch);
        // Protected tokens surfaced as an explicit list (mandate §6).
        assert!(exported.prompt.contains("Protected tokens"));
        assert!(exported.prompt.contains("%s"));

        // Compliant structured response for ALL 220 (shuffled order,
        // XML-rich values, multi-line on a few).
        let mut rows = String::new();
        rows.push_str(&format!("{CHAT_RESPONSE_VERSION}\n"));
        for i in (0..220).rev() {
            let text = if i % 30 == 0 {
                format!("Строка {i}\nвторая строка <i>тег</i> {{0}} %s")
            } else {
                format!("Строка {i} <i>тег</i> {{0}} %s")
            };
            rows.push_str(&format!(
                "{DIR_BEGIN}\n{DIR_KEY} {}\n{DIR_REVIEW} no\n{DIR_TRANSLATION}\n{text}\n{DIR_END}\n",
                keyed_key(&format!("Entry{i}"))
            ));
        }
        rows.push_str(&format!("{DIR_END_RESPONSE}\n"));
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &rows).unwrap();
        assert_eq!(imported.preview.items.len(), 220);
        assert!(imported.preview.items.iter().all(|p| p.conflict.is_none()));
        let sample = imported
            .preview
            .items
            .iter()
            .find(|p| p.text.contains('\n'))
            .expect("multi-line rows survive");
        assert!(sample.text.contains("<i>тег</i> {0} %s"));

        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 220);
        assert_eq!(applied.held_back, 0);
        let env = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        let count = env
            .project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian" && t.origin == Origin::Imported)
            .count();
        assert_eq!(count, 220);
    }

    /// XML constructs and grammar tokens in VALUES survive the structured
    /// round-trip; the prompt's protected-token list carries them.
    #[test]
    fn chat_batch_xml_values_and_tokens_survive_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        write(
            &mod_root.join("Languages/English/Keyed/Tech.xml"),
            r#"<RimWorld-Strings><Cost>Cost: {0} [gram] %s &amp; <i>rich</i></Cost></RimWorld-Strings>"#,
        );
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let k_cost = keyed_key("Cost");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                std::slice::from_ref(&k_cost),
            ))
            .unwrap();
        assert_eq!(batch.items[0].source, "Cost: {0} [gram] %s & <i>rich</i>");
        let exported = export(&mgr, &batch.batch_id, snap.session_epoch);
        for token in ["{0}", "%s", "[gram]"] {
            assert!(
                exported.prompt.contains(&format!("- {token}\n")),
                "protected token {token} must be listed in the prompt"
            );
        }
        let response = structured_response(
            &batch.batch_id,
            "Russian",
            &[(k_cost.as_str(), "Цена: {0} [gram] %s & <i>богато</i>")],
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        assert_eq!(
            imported.preview.items[0].text,
            "Цена: {0} [gram] %s & <i>богато</i>"
        );
        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 1);
    }

    /// Glossary lookup: only terms RELEVANT to the batch's sources are
    /// baked into the prompt (mandate §6 — relevant terminology only).
    #[test]
    fn chat_batch_prompt_carries_relevant_glossary_only() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        // Two terms: one relevant ("save"), one not.
        mgr.glossary_upsert(&crate::contract::ProjectGlossaryUpsertRequest {
            project_id: snap.project_id.clone(),
            session_epoch: snap.session_epoch,
            term: "save".into(),
            translation: "сохранение".into(),
            note: None,
        })
        .unwrap();
        mgr.glossary_upsert(&crate::contract::ProjectGlossaryUpsertRequest {
            project_id: snap.project_id.clone(),
            session_epoch: snap.session_epoch,
            term: "unrelated".into(),
            translation: "никогда".into(),
            note: None,
        })
        .unwrap();
        // The upserts acked revisions — re-open the fresh state for create.
        let snap2 = mgr.snapshot(&snap.project_id).unwrap();
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap2.project_id,
                snap2.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        let exported = export(&mgr, &batch.batch_id, snap2.session_epoch);
        assert!(exported.prompt.contains("save => сохранение"));
        assert!(!exported.prompt.contains("unrelated"));
        // Persisted for the history surface.
        assert_eq!(exported.batch.glossary_used.len(), 1);
    }

    // ---------- conflict gate (human work is never silently overwritten) ----------

    /// A human fix that lands BETWEEN create and export (the re-export
    /// laundering hole) makes the row `changed_after_select`: held back by
    /// default, applied only on the explicit override.
    #[test]
    fn chat_batch_conflict_gate_holds_back_human_fix_after_selection() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        let k_load = keyed_key("LoadGame");
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_save.clone(), k_load.clone()],
            ))
            .unwrap();

        // A HUMAN translation lands on SaveGame after the selection (acked
        // write through the session seam — the ordinary editor path).
        mgr.apply(&crate::contract::ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: snap.revision,
            session_epoch: snap.session_epoch,
            intents: vec![TranslationIntent {
                entry: batch.items[0].entry.clone(),
                locale: "Russian".into(),
                action: IntentAction::SetTranslation,
                text: Some("Моё ручное сохранение".into()),
            }],
            origin: None, // absence = human
        })
        .unwrap();

        // Export re-records the gates at the new revision (sources intact —
        // only the translation moved), so the import passes; the conflict
        // gate is what protects the human fix now.
        export(&mgr, &batch.batch_id, snap.session_epoch);
        let response = structured_response(
            &batch.batch_id,
            "Russian",
            &[
                (k_save.as_str(), "ИИ-перевод сохранения"),
                (k_load.as_str(), "Загрузить игру"),
            ],
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        let conflict = imported
            .preview
            .items
            .iter()
            .find(|p| p.key == k_save)
            .unwrap()
            .conflict
            .as_ref()
            .expect("human fix must conflict");
        assert_eq!(conflict.reason, "changed_after_select");
        assert_eq!(conflict.current_text, "Моё ручное сохранение");
        assert_eq!(conflict.current_origin, "human");
        assert!(imported.preview.items[1].conflict.is_none());

        // Default apply: the conflict row is HELD BACK (1 of 2), the clean
        // row applies; the human text stays.
        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision + 1,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 1);
        assert_eq!(applied.held_back, 1);
        let env = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        let saved = env
            .project
            .translations
            .iter()
            .find(|t| t.locale == "Russian" && t.source_id.key == "SaveGame")
            .unwrap();
        assert_eq!(saved.text.as_deref(), Some("Моё ручное сохранение"));
        assert_eq!(saved.origin, Origin::Human);

        // A fresh batch over the same row snapshots the LIVE state — still
        // the HUMAN text (the held-back AI suggestion never landed), and an
        // identical-to-live response never conflicts: nothing would change.
        let fresh = mgr.open(&snap.project_id).unwrap();
        let batch3 = mgr
            .chat_batch_create(&create_req(
                &fresh.project_id,
                fresh.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        assert_eq!(
            batch3.items[0].translation_at_select.as_deref(),
            Some("Моё ручное сохранение")
        );
        export(&mgr, &batch3.batch_id, fresh.session_epoch);
        let response3 = structured_response(
            &batch3.batch_id,
            "Russian",
            &[(k_save.as_str(), "Моё ручное сохранение")],
        );
        let imported3 = import(&mgr, &batch3.batch_id, fresh.session_epoch, &response3).unwrap();
        assert!(imported3.preview.items[0].conflict.is_none());
    }

    /// Mandate §14/§30: altered placeholders make a row a PROBLEM row —
    /// held back from apply with the named mismatch, applied only on the
    /// explicit override; clean rows in the same response apply normally.
    #[test]
    fn chat_batch_placeholder_mismatch_is_a_problem_row_never_silent() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        write(
            &mod_root.join("Languages/English/Keyed/Tokens.xml"),
            r#"<RimWorld-Strings><Progress>Progress: {0} of %s</Progress><Plain>Plain text</Plain></RimWorld-Strings>"#,
        );
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let k_progress = keyed_key("Progress");
        let k_plain = keyed_key("Plain");

        let batch = mgr
            .chat_batch_create(&create_req(
                &snap.project_id,
                snap.session_epoch,
                &[k_progress.clone(), k_plain.clone()],
            ))
            .unwrap();
        export(&mgr, &batch.batch_id, snap.session_epoch);

        // The token row LOST {0} and renamed %s → structural problem; the
        // plain row is clean.
        let response = structured_response(
            &batch.batch_id,
            "Russian",
            &[
                (k_progress.as_str(), "Прогресс: {1} из %d"),
                (k_plain.as_str(), "Обычный текст"),
            ],
        );
        let imported = import(&mgr, &batch.batch_id, snap.session_epoch, &response).unwrap();
        let problem = imported
            .preview
            .items
            .iter()
            .find(|p| p.key == k_progress)
            .unwrap();
        let detail = problem.structural_problem.as_ref().expect("mismatch named");
        assert!(detail.contains("{0}"), "{detail}");
        // Identical sets stay clean.
        assert!(imported
            .preview
            .items
            .iter()
            .find(|p| p.key == k_plain)
            .unwrap()
            .structural_problem
            .is_none());

        // Default apply: only the clean row lands; the problem row is held.
        let applied = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap.revision,
                session_epoch: snap.session_epoch,
                override_keys: vec![],
            })
            .unwrap();
        assert_eq!(applied.applied, 1);
        assert_eq!(applied.held_back, 1);
        let env = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        assert!(env
            .project
            .translations
            .iter()
            .all(|t| t.source_id.key != "Progress"));

        // The explicit override applies the (human-decided) problem row.
        let fresh = mgr.open(&snap.project_id).unwrap();
        let response2 = structured_response(
            &batch.batch_id,
            "Russian",
            &[
                (k_progress.as_str(), "Прогресс: {1} из %d"),
                (k_plain.as_str(), "Обычный текст"),
            ],
        );
        let _ = response2;
        let batch2 = mgr
            .chat_batch_create(&create_req(
                &fresh.project_id,
                fresh.session_epoch,
                std::slice::from_ref(&k_progress),
            ))
            .unwrap();
        export(&mgr, &batch2.batch_id, fresh.session_epoch);
        let imported2 = import(
            &mgr,
            &batch2.batch_id,
            fresh.session_epoch,
            &structured_response(
                &batch2.batch_id,
                "Russian",
                &[(k_progress.as_str(), "Прогресс: {1} из %d")],
            ),
        )
        .unwrap();
        assert!(imported2.preview.items[0].structural_problem.is_some());
        let applied2 = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch2.batch_id.clone(),
                expected_revision: fresh.revision,
                session_epoch: fresh.session_epoch,
                override_keys: vec![k_progress.clone()],
            })
            .unwrap();
        assert_eq!(applied2.applied, 1);
        assert_eq!(applied2.held_back, 0);
    }

    /// A row that carried a HUMAN translation at selection time conflicts
    /// when the response proposes a different text (`human_translation`).
    #[test]
    fn chat_batch_conflict_gate_flags_preexisting_human_translation() {
        let (_dir, mgr, snap) = setup();
        let k_save = keyed_key("SaveGame");
        // Human translation BEFORE the selection.
        mgr.apply(&crate::contract::ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: snap.revision,
            session_epoch: snap.session_epoch,
            intents: vec![TranslationIntent {
                entry: SourceEntryId {
                    kind: EntryKind::Keyed,
                    key: "SaveGame".into(),
                    def_type: None,
                },
                locale: "Russian".into(),
                action: IntentAction::SetTranslation,
                text: Some("Ручной перевод".into()),
            }],
            origin: None,
        })
        .unwrap();
        let snap2 = mgr.snapshot(&snap.project_id).unwrap();
        let batch = mgr
            .chat_batch_create(&create_req(
                &snap2.project_id,
                snap2.session_epoch,
                std::slice::from_ref(&k_save),
            ))
            .unwrap();
        assert_eq!(batch.items[0].origin_at_select.as_deref(), Some("human"));
        export(&mgr, &batch.batch_id, snap2.session_epoch);
        let response = structured_response(
            &batch.batch_id,
            "Russian",
            &[(k_save.as_str(), "ИИ-вариант")],
        );
        let imported = import(&mgr, &batch.batch_id, snap2.session_epoch, &response).unwrap();
        let conflict = imported.preview.items[0].conflict.as_ref().unwrap();
        assert_eq!(conflict.reason, "human_translation");
        assert_eq!(conflict.current_text, "Ручной перевод");

        // Held back by default; the identical-to-live text never conflicts.
        let err = mgr
            .chat_batch_apply(&ChatBatchApplyRequest {
                batch_id: batch.batch_id.clone(),
                expected_revision: snap2.revision,
                session_epoch: snap2.session_epoch,
                override_keys: vec![],
            })
            .unwrap_err();
        assert!(err.message.contains("no applicable rows"), "{err}");
    }
}
