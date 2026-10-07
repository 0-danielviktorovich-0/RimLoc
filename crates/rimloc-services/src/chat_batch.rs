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
    ChatBatchImportRequest, ChatBatchImportResponse, ChatBatchItem, ChatBatchPreview,
    ChatBatchPreviewItem, ChatBatchStatus, ChatBatchStatusRequest, ChatBatchStatusResponse,
    ContractError, ContractErrorCode, IntentAction, ProjectId, TranslationIntent,
    CHAT_BATCH_FILE_VERSION,
};
use crate::observability::{generate_operation_id, sha256_hex};
use crate::session::ProjectSessionManager;
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

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
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
        // live trusted state (epoch guard included).
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
                items.push(ChatBatchItem {
                    key: key.clone(),
                    source: entry.text.clone(),
                    entry: entry.id.clone(),
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
            match chat_batch_inventory_hash(project, &batch.items) {
                Some(h) if h == chat_batch_source_hash(&batch.items) => Ok((h, revision)),
                Some(_) => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    "batch basis drifted: a source text changed since the selection; create a new batch"
                        .to_string(),
                )),
                None => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    "batch basis drifted: an entry is missing from the inventory; rescan and create a new batch"
                        .to_string(),
                )),
            }
        })?;
        let (hash, revision) = live;

        batch.revision_at_export = revision;
        batch.source_hash = hash;
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
                Some(h) if h == batch.source_hash => Ok(()),
                _ => Err(ContractError::new(
                    ContractErrorCode::ContractViolation,
                    "source hash mismatch: the batch lines no longer match the inventory; re-export the batch"
                        .to_string(),
                )),
            }
        });
        if let Err(e) = checked {
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

        let parsed = parse_chat_response(&req.response_text, &batch).map_err(|e| {
            ContractError::new(ContractErrorCode::ContractViolation, e.message)
                .with_details(e.details)
        })?;

        let preview = ChatBatchPreview {
            items: parsed,
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

        // Map preview rows back onto the pinned identities, in batch order.
        // A preview row whose key is not pinned is impossible by
        // construction (the parser only accepts batch keys); if a future
        // edit breaks that, refuse honestly instead of guessing.
        let by_key: HashMap<&str, &ChatBatchItem> =
            batch.items.iter().map(|i| (i.key.as_str(), i)).collect();
        let mut intents = Vec::with_capacity(preview.items.len());
        for p in &preview.items {
            let item = by_key.get(p.key.as_str()).ok_or_else(|| {
                ContractError::new(
                    ContractErrorCode::Internal,
                    format!("preview key `{}` is not pinned in the batch", p.key),
                )
            })?;
            intents.push(TranslationIntent {
                entry: item.entry.clone(),
                locale: batch.locale.clone(),
                action: IntentAction::SetTranslation,
                text: Some(p.text.clone()),
            });
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
        });
    }
    Some(chat_batch_source_hash(&live))
}

/// The structured prompt the user copies into the LLM chat. The response
/// contract is EXACTLY one `<key>: <translation>` line per batch item, in
/// the given order, nothing else — the parser refuses anything looser.
pub fn build_chat_prompt(batch: &ChatBatch) -> String {
    let mut p = String::new();
    p.push_str("You are a game localization translator. Translate each source string into the target language.\n\n");
    p.push_str(&format!(
        "Target language (locale folder): {}\n",
        batch.locale
    ));
    p.push_str(&format!("Strings: {}\n\n", batch.items.len()));
    p.push_str("Source strings follow. The key before the \": \" is an identifier — copy it EXACTLY, unchanged, into your answer.\n\n");
    for i in &batch.items {
        p.push_str(&format!("{}{PAIR_DELIM}{}\n", i.key, i.source));
    }
    p.push_str(
        "\nRespond with ONLY one line per string, in the same order, in the exact format:\n",
    );
    p.push_str(&format!("<key>{PAIR_DELIM}<translation>\n\n"));
    p.push_str("Rules:\n");
    p.push_str("- Copy each key exactly; never invent, reorder, merge or drop keys.\n");
    p.push_str("- Keep placeholders like {0}, %s, $name and XML tags intact.\n");
    p.push_str("- One line per string; keep the translation on a single line.\n");
    p.push_str("- No commentary, no code fences, no extra lines.\n");
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

/// The STRICT parser (mandate): one `<key>: <translation>` line per batch
/// item. Blank lines are ignored; EVERY other line must parse against a
/// known batch key. Refusals (whole import, never partial):
/// - malformed — a non-blank line without the `<known-key>: <text>` shape,
///   or an empty translation;
/// - unknown/extra — a well-shaped line whose key is not in the batch;
/// - duplicate — one key answered twice;
/// - missing — a batch key the response never answered.
fn parse_chat_response(
    text: &str,
    batch: &ChatBatch,
) -> Result<Vec<ChatBatchPreviewItem>, ChatParseRejection> {
    let keys: BTreeSet<&str> = batch.items.iter().map(|i| i.key.as_str()).collect();
    // Responses pasted from terminals can carry a leading UTF-8 BOM; it is
    // transport noise, not content — strip it once instead of failing the
    // first line for an invisible character.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
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
        }];
        let h1 = chat_batch_source_hash(&items);
        let mut drifted = items.clone();
        drifted[0].source = "A2".into();
        assert_ne!(h1, chat_batch_source_hash(&drifted));
    }
}
