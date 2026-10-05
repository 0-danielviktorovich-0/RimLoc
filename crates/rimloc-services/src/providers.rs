//! Provider-instance settings (provider/settings parity slice): the store
//! and the secret seam behind the `provider_instance_*` contract operations.
//!
//! Separation of concerns — the security property of this module:
//! - the API key lives ONLY in the OS keychain, through the
//!   [`ProviderSecretSink`] seam (feature `keychain` ⇒
//!   [`KeychainSecretSink`] over `rimloc-llm::secrets`; tests inject an
//!   in-memory sink);
//! - the settings file (`<managed root>/settings/providers.json`) holds
//!   METADATA ONLY — ids, presets, models, base URLs, presence markers.
//!   A secret never reaches this file, any log line, or any response DTO
//!   ([`crate::contract::ProviderInstanceSummary`] has no secret field);
//! - form validation (`invalid_config`) refuses bad URLs, empty models and
//!   keyless cloud instances BEFORE any state change.

use crate::contract::{ContractError, ContractErrorCode, ProviderInstanceSummary};
use rimloc_llm::provider::builtin_presets;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Where provider API keys live. Production: the OS keychain. Tests: an
/// in-memory sink. The sink hands the secret back ONLY to callers that
/// already know it exists — the contract layer probes presence (`has`),
/// it never reads values.
pub trait ProviderSecretSink: Send + Sync {
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    /// Returns the stored value — called only by engine-facing paths, never
    /// by a contract operation.
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    /// `Ok(true)` = existed and is gone.
    fn delete(&self, account: &str) -> Result<bool, String>;
    /// Presence probe — never returns the value.
    fn has(&self, account: &str) -> Result<bool, String>;
}

/// Production sink: the OS keychain via `rimloc-llm::secrets` (keyring v3;
/// macOS Keychain / Secret Service / Windows Credential Manager). Service
/// naming follows the `rimloc-llm` convention; a test/sandbox run
/// redirects the namespace with `RIMLOC_KEYCHAIN_SERVICE` or an explicit
/// service name.
#[cfg(feature = "keychain")]
pub struct KeychainSecretSink {
    service: String,
}

#[cfg(feature = "keychain")]
impl KeychainSecretSink {
    /// The convention service (`rimloc-llm`), honouring the
    /// `RIMLOC_KEYCHAIN_SERVICE` override for sandboxed runs.
    pub fn with_default_service() -> Self {
        Self {
            service: rimloc_llm::secrets::effective_service(),
        }
    }

    /// Explicit service name (automated tests use the
    /// `rimloc-llm-selftest` namespace).
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

#[cfg(feature = "keychain")]
impl ProviderSecretSink for KeychainSecretSink {
    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        rimloc_llm::secrets::set_secret_in(&self.service, account, secret)
            .map_err(|e| e.to_string())
    }
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        rimloc_llm::secrets::get_secret_in(&self.service, account).map_err(|e| e.to_string())
    }
    fn delete(&self, account: &str) -> Result<bool, String> {
        rimloc_llm::secrets::delete_secret_in(&self.service, account).map_err(|e| e.to_string())
    }
    fn has(&self, account: &str) -> Result<bool, String> {
        rimloc_llm::secrets::has_secret_in(&self.service, account).map_err(|e| e.to_string())
    }
}

/// In-memory sink for tests and feature-off runs that never store secrets:
/// instances created without a keychain build simply cannot carry one
/// (upsert-with-secret is refused as `unsupported_capability`), so this
/// sink exists only behind `cfg(test)`.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct MemorySecretSink {
    entries: std::sync::Mutex<std::collections::BTreeMap<String, String>>,
}

#[cfg(test)]
impl ProviderSecretSink for MemorySecretSink {
    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        self.entries
            .lock()
            .expect("memory sink poisoned")
            .insert(account.to_string(), secret.to_string());
        Ok(())
    }
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        Ok(self
            .entries
            .lock()
            .expect("memory sink poisoned")
            .get(account)
            .cloned())
    }
    fn delete(&self, account: &str) -> Result<bool, String> {
        Ok(self
            .entries
            .lock()
            .expect("memory sink poisoned")
            .remove(account)
            .is_some())
    }
    fn has(&self, account: &str) -> Result<bool, String> {
        Ok(self
            .entries
            .lock()
            .expect("memory sink poisoned")
            .contains_key(account))
    }
}

// ---------------------------------------------------------------------------
// Settings file (metadata ONLY — never a secret)
// ---------------------------------------------------------------------------

const SETTINGS_DIR: &str = "settings";
const SETTINGS_FILE: &str = "providers.json";
const SETTINGS_KIND: &str = "rimloc-provider-settings";
const SETTINGS_VERSION: u32 = 1;

/// One persisted instance. `has_key_cache` is a PRESENCE marker (bool), not
/// the key; it heals from the live sink probe whenever a sink is attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProviderInstanceRecord {
    pub id: String,
    pub preset: String,
    pub label: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub local: bool,
    pub has_key_cache: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProviderSettingsFile {
    kind: String,
    version: u32,
    revision: u64,
    instances: Vec<ProviderInstanceRecord>,
}

impl Default for ProviderSettingsFile {
    fn default() -> Self {
        Self {
            kind: SETTINGS_KIND.to_string(),
            version: SETTINGS_VERSION,
            revision: 0,
            instances: Vec::new(),
        }
    }
}

/// In-memory mirror of the settings file. `unreadable` carries the load
/// error when the file exists but cannot be parsed — corruption is surfaced
/// as a typed failure (M2 lesson: corruption must never look like "the
/// settings are gone"), and NO write ever happens over an unreadable file
/// (nothing is silently overwritten or deleted). `loaded_hash` is the
/// sha256 of the bytes as last seen/written by THIS session — persist
/// refuses (`project_changed_on_disk`) when the file changed outside the
/// session, exactly the project-file discipline: an external write is never
/// clobbered by a stale in-memory state.
pub(crate) struct ProviderSettingsState {
    path: PathBuf,
    file: ProviderSettingsFile,
    unreadable: Option<String>,
    loaded_hash: Option<String>,
}

impl ProviderSettingsState {
    /// Load (or initialise) the settings file under `<managed_root>/settings/`.
    pub fn load(managed_root: &Path) -> std::io::Result<Self> {
        let path = managed_root.join(SETTINGS_DIR).join(SETTINGS_FILE);
        let mut state = Self {
            path,
            file: ProviderSettingsFile::default(),
            unreadable: None,
            loaded_hash: None,
        };
        match std::fs::read(&state.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            Ok(bytes) => {
                state.loaded_hash = Some(crate::observability::sha256_hex(&bytes));
                match serde_json::from_slice::<ProviderSettingsFile>(&bytes) {
                    Ok(file) if file.kind == SETTINGS_KIND && file.version == SETTINGS_VERSION => {
                        state.file = file;
                    }
                    Ok(file) => {
                        state.unreadable = Some(format!(
                            "unsupported provider settings container: kind `{}`, version `{}`",
                            file.kind, file.version
                        ));
                    }
                    Err(e) => {
                        state.unreadable = Some(format!("provider settings unreadable: {e}"));
                    }
                }
            }
        }
        Ok(state)
    }

    /// External-change guard: the file on disk must be EXACTLY what this
    /// session last read or wrote.
    fn check_disk_unchanged(&self) -> Result<(), ContractError> {
        let current = std::fs::read(&self.path)
            .ok()
            .map(|b| crate::observability::sha256_hex(&b));
        if current != self.loaded_hash {
            return Err(ContractError::new(
                ContractErrorCode::ProjectChangedOnDisk,
                "provider settings changed outside this session; reload the app to adopt \
                 the on-disk state (refusing to overwrite)",
            ));
        }
        Ok(())
    }

    fn persist(&mut self) -> Result<u64, ContractError> {
        if self.unreadable.is_some() {
            let msg = self
                .unreadable
                .as_deref()
                .unwrap_or("provider settings unreadable");
            return Err(ContractError::new(
                ContractErrorCode::SaveFailed,
                format!("{msg}; refusing to write over an unreadable settings file"),
            ));
        }
        self.check_disk_unchanged()?;
        let bytes = serde_json::to_vec_pretty(&self.file)
            .map_err(|e| ContractError::new(ContractErrorCode::SaveFailed, e.to_string()))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ContractError::new(ContractErrorCode::SaveFailed, format!("settings dir: {e}"))
            })?;
        }
        crate::write_atomic(&self.path, &bytes).map_err(|e| {
            ContractError::new(
                ContractErrorCode::SaveFailed,
                format!("settings persist: {e}"),
            )
        })?;
        self.loaded_hash = Some(crate::observability::sha256_hex(&bytes));
        Ok(self.file.revision)
    }

    fn check_unreadable(&self) -> Result<(), ContractError> {
        match &self.unreadable {
            Some(msg) => Err(ContractError::new(
                ContractErrorCode::SaveFailed,
                msg.clone(),
            )),
            None => Ok(()),
        }
    }

    fn summary(&self, record: &ProviderInstanceRecord, has_key: bool) -> ProviderInstanceSummary {
        ProviderInstanceSummary {
            id: record.id.clone(),
            preset: record.preset.clone(),
            label: record.label.clone(),
            model: record.model.clone(),
            base_url: record.base_url.clone(),
            local: record.local,
            has_key,
            created_at_ms: record.created_at_ms,
            updated_at_ms: record.updated_at_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Form validation (the typed `invalid_config` surface)
// ---------------------------------------------------------------------------

/// A syntactically valid absolute http(s) URL with a host. Anything else is
/// a typed `invalid_config` refusal BEFORE any state change.
pub(crate) fn validate_base_url(url: &str) -> Result<(), ContractError> {
    let parsed = reqwest::Url::parse(url).map_err(|e| {
        ContractError::new(
            ContractErrorCode::InvalidConfig,
            format!("base_url `{url}` is not a valid URL: {e}"),
        )
    })?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(ContractError::new(
            ContractErrorCode::InvalidConfig,
            format!("base_url `{url}` must use http(s), got `{scheme}://`"),
        ));
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        return Err(ContractError::new(
            ContractErrorCode::InvalidConfig,
            format!("base_url `{url}` has no host"),
        ));
    }
    Ok(())
}

/// The preset registry for the contract surface: the builtin presets plus
/// the `custom` OpenAI-compatible template (base_url + model are REQUIRED
/// there — that is the whole point of the template).
pub(crate) fn known_preset(preset: &str) -> bool {
    preset == "custom" || builtin_presets().contains_key(preset)
}

pub(crate) fn preset_default_base_url(preset: &str) -> Option<String> {
    builtin_presets()
        .get(preset)
        .and_then(|p| p.base_url.clone())
}

pub(crate) fn preset_is_local(preset: &str) -> bool {
    preset == "ollama"
}

/// Context object uniting the manager's secret sink with the settings
/// state — the four `provider_instance_*` session operations delegate here.
pub(crate) struct ProviderOps<'a> {
    pub state: &'a mut ProviderSettingsState,
    pub sink: Option<&'a Arc<dyn ProviderSecretSink>>,
}

impl ProviderOps<'_> {
    fn sink(&self) -> Result<&Arc<dyn ProviderSecretSink>, ContractError> {
        self.sink.ok_or_else(|| {
            ContractError::new(
                ContractErrorCode::UnsupportedCapability,
                "this build has no keychain sink: provider API keys cannot be stored \
                 (build rimloc-services with feature `keychain`)",
            )
        })
    }

    fn has_key(&self, account: &str, fallback: bool) -> Result<bool, ContractError> {
        match self.sink {
            Some(sink) => sink
                .has(account)
                .map_err(|e| ContractError::new(ContractErrorCode::SaveFailed, e)),
            None => Ok(fallback),
        }
    }

    /// Read-only redacted list. Corruption of the settings file is a typed
    /// failure, never an empty list.
    pub fn list(
        &self,
        job_id: String,
    ) -> Result<crate::contract::ProviderInstanceListResponse, ContractError> {
        self.state.check_unreadable()?;
        let mut instances = Vec::with_capacity(self.state.file.instances.len());
        for record in &self.state.file.instances {
            let has_key = self.has_key(&record.id, record.has_key_cache)?;
            instances.push(self.state.summary(record, has_key));
        }
        let total = instances.len();
        Ok(crate::contract::ProviderInstanceListResponse {
            job_id,
            revision: self.state.file.revision,
            total,
            instances,
        })
    }

    /// Create or edit one instance. Order of operations (crash-consistency
    /// and no-orphan discipline):
    /// 1. form validation (`invalid_config` / `contract_violation`) —
    ///    nothing has been written yet;
    /// 2. key availability check — a keyless cloud instance is refused
    ///    BEFORE any write;
    /// 3. keychain write (only when a new secret was supplied);
    /// 4. metadata persist (atomic tmp+rename, persist-before-ack).
    pub fn upsert(
        &mut self,
        req: &crate::contract::ProviderInstanceUpsertRequest,
        mint_id: impl FnOnce() -> String,
        job_id: String,
    ) -> Result<crate::contract::ProviderInstanceUpsertResponse, ContractError> {
        self.state.check_unreadable()?;

        // --- lost-update guard -------------------------------------------------
        if let Some(expected) = req.expected_revision {
            if expected != self.state.file.revision {
                return Err(ContractError::stale_revision(
                    expected,
                    self.state.file.revision,
                ));
            }
        }

        // --- form: preset / model / base_url -----------------------------------
        let preset = req.preset.trim();
        if preset.is_empty() {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                "provider preset must not be empty",
            ));
        }
        if !known_preset(preset) {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "unknown provider preset `{preset}` (known: anthropic, openai, zai, ollama, custom)"
                ),
            ));
        }
        let is_custom = preset == "custom";
        let local = req.local.unwrap_or(preset_is_local(preset));
        let model = req.model.trim();
        if model.is_empty() {
            return Err(ContractError::new(
                ContractErrorCode::InvalidConfig,
                "model must not be empty",
            ));
        }
        let explicit_base_url = req
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let base_url = match explicit_base_url {
            Some(url) => Some(url.to_string()),
            None if is_custom => {
                return Err(ContractError::new(
                    ContractErrorCode::InvalidConfig,
                    "custom provider requires a base_url (OpenAI-compatible endpoint)",
                ));
            }
            None => preset_default_base_url(preset),
        };
        if let Some(url) = &base_url {
            validate_base_url(url)?;
        }

        // --- identity -----------------------------------------------------------
        let mut existing = match req.instance_id.as_deref() {
            None => None,
            Some(id) => match self.state.file.instances.iter().find(|r| r.id == id) {
                Some(record) => Some(record.clone()),
                None => {
                    return Err(ContractError::new(
                        ContractErrorCode::ContractViolation,
                        format!("unknown provider instance id: {id}"),
                    ));
                }
            },
        };
        let id = match &existing {
            Some(record) => record.id.clone(),
            None => mint_id(),
        };
        // --- key state (BEFORE any write) ---------------------------------------
        let new_secret = req
            .secret
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let has_key = if let Some(secret) = new_secret {
            self.sink()?.set(&id, secret).map_err(|e| {
                ContractError::new(
                    ContractErrorCode::SaveFailed,
                    format!("keychain write failed: {e}"),
                )
            })?;
            true
        } else {
            let fallback = existing.as_ref().is_some_and(|r| r.has_key_cache);
            self.has_key(&id, fallback)?
        };
        if !local && !has_key {
            return Err(ContractError::new(
                ContractErrorCode::InvalidConfig,
                "no API key stored for this cloud provider; paste the key to save the instance",
            ));
        }

        // --- persist metadata (persist-before-ack) ------------------------------
        let now = now_ms();
        let label = req.label.trim();
        let record = match &mut existing {
            Some(_) => {
                let slot = self
                    .state
                    .file
                    .instances
                    .iter_mut()
                    .find(|r| r.id == id)
                    .expect("instance existed a few lines above");
                slot.preset = preset.to_string();
                if !label.is_empty() {
                    slot.label = label.to_string();
                }
                slot.model = model.to_string();
                slot.base_url = base_url;
                slot.local = local;
                slot.has_key_cache = has_key;
                slot.updated_at_ms = now;
                slot.clone()
            }
            None => {
                let record = ProviderInstanceRecord {
                    id: id.clone(),
                    preset: preset.to_string(),
                    label: if label.is_empty() {
                        preset.to_string()
                    } else {
                        label.to_string()
                    },
                    model: model.to_string(),
                    base_url,
                    local,
                    has_key_cache: has_key,
                    created_at_ms: now,
                    updated_at_ms: now,
                };
                self.state.file.instances.push(record.clone());
                record
            }
        };
        self.state.file.revision += 1;
        let revision = self.state.persist()?;

        Ok(crate::contract::ProviderInstanceUpsertResponse {
            job_id,
            revision,
            instance: self.state.summary(&record, has_key),
        })
    }

    /// Remove one instance. The KEYCHAIN key is deleted FIRST: a failed
    /// keychain delete refuses the whole operation and the metadata stays,
    /// so no orphaned secret can remain.
    pub fn delete(
        &mut self,
        req: &crate::contract::ProviderInstanceDeleteRequest,
        job_id: String,
    ) -> Result<crate::contract::ProviderInstanceDeleteResponse, ContractError> {
        self.state.check_unreadable()?;
        if let Some(expected) = req.expected_revision {
            if expected != self.state.file.revision {
                return Err(ContractError::stale_revision(
                    expected,
                    self.state.file.revision,
                ));
            }
        }
        let record = self
            .state
            .file
            .instances
            .iter()
            .find(|r| r.id == req.instance_id)
            .cloned()
            .ok_or_else(|| {
                ContractError::new(
                    ContractErrorCode::ContractViolation,
                    format!("unknown provider instance id: {}", req.instance_id),
                )
            })?;

        let key_removed = if let Some(sink) = self.sink {
            sink.delete(&record.id).map_err(|e| {
                ContractError::new(
                    ContractErrorCode::SaveFailed,
                    format!("keychain delete failed: {e}"),
                )
            })?
        } else {
            false
        };
        self.state.file.instances.retain(|r| r.id != record.id);
        self.state.file.revision += 1;
        let revision = self.state.persist()?;
        Ok(crate::contract::ProviderInstanceDeleteResponse {
            job_id,
            revision,
            removed_id: record.id,
            key_removed,
        })
    }

    /// Form validation WITHOUT network and WITHOUT keychain access. This is
    /// a CONFIGURATION check — reachability/auth probes are not in this
    /// slice (and never make paid calls from it).
    pub fn validate(
        &self,
        req: &crate::contract::ProviderInstanceValidateRequest,
        job_id: String,
    ) -> Result<crate::contract::ProviderInstanceValidateResponse, ContractError> {
        let mut problems = Vec::new();
        let preset = req.preset.trim();
        if preset.is_empty() {
            problems.push("preset must not be empty".to_string());
        } else if !known_preset(preset) {
            problems.push(format!(
                "unknown provider preset `{preset}` (known: anthropic, openai, zai, ollama, custom)"
            ));
        }
        let is_custom = preset == "custom";
        let local = req.local.unwrap_or(preset_is_local(preset));
        if req.model.trim().is_empty() {
            problems.push("model must not be empty".to_string());
        }
        let explicit = req
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let base_url = match explicit {
            Some(url) => Some(url.to_string()),
            None if is_custom => None,
            None => preset_default_base_url(preset),
        };
        match base_url {
            Some(url) => {
                if let Err(e) = validate_base_url(&url) {
                    problems.push(e.message);
                }
            }
            None if is_custom => {
                problems.push(
                    "custom provider requires a base_url (OpenAI-compatible endpoint)".to_string(),
                );
            }
            None => {}
        }
        if !local && !req.has_key {
            problems.push(
                "no API key stored for this cloud provider; paste the key to save the instance"
                    .to_string(),
            );
        }
        let ok = problems.is_empty();
        Ok(crate::contract::ProviderInstanceValidateResponse {
            job_id,
            ok,
            problems,
        })
    }
}

pub(crate) fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{
        ProviderInstanceDeleteRequest, ProviderInstanceUpsertRequest,
        ProviderInstanceValidateRequest,
    };
    use crate::session::ProjectSessionManager;

    const SECRET: &str = "sk-lane-secret-abc123-nevershown";

    struct Fixture {
        _dir: tempfile::TempDir,
        sink: Arc<MemorySecretSink>,
        manager: ProjectSessionManager,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(MemorySecretSink::default());
        let manager =
            ProjectSessionManager::new_with_secret_sink(dir.path(), sink.clone()).unwrap();
        Fixture {
            _dir: dir,
            sink,
            manager,
        }
    }

    fn upsert_req(
        preset: &str,
        model: &str,
        base_url: Option<&str>,
    ) -> ProviderInstanceUpsertRequest {
        ProviderInstanceUpsertRequest {
            instance_id: None,
            preset: preset.into(),
            label: preset.to_string(),
            model: model.into(),
            base_url: base_url.map(str::to_string),
            secret: None,
            local: None,
            expected_revision: None,
        }
    }

    fn settings_path(manager: &ProjectSessionManager) -> PathBuf {
        manager
            .managed_root()
            .join("settings")
            .join("providers.json")
    }

    /// PROOF (live-contract CRUD): create → list shows the redacted summary
    /// → edit updates in place → delete removes it; the key follows the
    /// instance in the sink, not in any file.
    #[test]
    fn provider_instance_crud_over_live_contract() {
        let fx = fixture();

        // CREATE (zai preset, key supplied) — persist-before-ack.
        let mut req = upsert_req("zai", "glm-4.7", None);
        req.secret = Some(SECRET.into());
        let created = fx.manager.provider_instance_upsert(&req).unwrap();
        assert!(created.instance.id.starts_with("prov-"));
        assert_eq!(created.instance.preset, "zai");
        assert!(created.instance.has_key);
        // The preset default base_url is resolved, not left empty.
        assert_eq!(
            created.instance.base_url.as_deref(),
            Some("https://api.z.ai/api/paas/v4")
        );
        let id = created.instance.id.clone();
        assert!(fx.sink.has(&id).unwrap());
        assert_eq!(fx.sink.get(&id).unwrap().as_deref(), Some(SECRET));

        // LIST — redacted summary; the secret appears NOWHERE in the wire.
        let list = fx.manager.provider_instance_list().unwrap();
        assert_eq!(list.total, 1);
        let wire = serde_json::to_string(&list).unwrap();
        assert!(!wire.contains(SECRET), "list wire form leaked the secret");
        assert!(wire.contains("\"has_key\":true"));

        // EDIT — same id, new model; secret untouched (None = keep).
        let mut edit = upsert_req("zai", "glm-4.6", None);
        edit.instance_id = Some(id.clone());
        let edited = fx.manager.provider_instance_upsert(&edit).unwrap();
        assert_eq!(edited.instance.id, id);
        assert_eq!(edited.instance.model, "glm-4.6");
        assert!(edited.instance.has_key);
        assert_eq!(
            fx.sink.get(&id).unwrap().as_deref(),
            Some(SECRET),
            "edit without a secret must keep the stored key"
        );

        // Unknown instance id is a typed violation, never a silent create.
        let mut ghost = upsert_req("zai", "glm-4.6", None);
        ghost.instance_id = Some("prov-does-not-exist".into());
        let err = fx.manager.provider_instance_upsert(&ghost).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);

        // DELETE — keychain key removed first, then metadata.
        let del = fx
            .manager
            .provider_instance_delete(&ProviderInstanceDeleteRequest {
                instance_id: id.clone(),
                expected_revision: None,
            })
            .unwrap();
        assert_eq!(del.removed_id, id);
        assert!(del.key_removed);
        assert!(!fx.sink.has(&id).unwrap());
        let list = fx.manager.provider_instance_list().unwrap();
        assert_eq!(list.total, 0);
        // Deleting again is a typed refusal, never silent success.
        let err = fx
            .manager
            .provider_instance_delete(&ProviderInstanceDeleteRequest {
                instance_id: id,
                expected_revision: None,
            })
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);
    }

    /// PROOF (no plaintext): after upserts, the settings file (and the whole
    /// managed tree) contains the instance metadata but NOT the secret —
    /// byte-level check over every file in the managed root.
    #[test]
    fn provider_secret_never_lands_in_persist_files() {
        let fx = fixture();
        let mut req = upsert_req("openai", "gpt-5", None);
        req.secret = Some(SECRET.into());
        let created = fx.manager.provider_instance_upsert(&req).unwrap();
        let id = created.instance.id.clone();

        let root = fx.manager.managed_root();
        let mut checked = 0usize;
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let bytes = std::fs::read(&path).unwrap();
                    assert!(
                        !windows_contains(&bytes, SECRET.as_bytes()),
                        "plaintext secret found in {}",
                        path.display()
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 1, "settings file must exist after an upsert");
        let settings = std::fs::read_to_string(settings_path(&fx.manager)).unwrap();
        assert!(settings.contains(&id), "metadata is in the settings file");
        assert!(settings.contains("\"has_key_cache\": true"));
        assert!(!settings.contains(SECRET));
    }

    /// PROOF (invalid-config, typed and panic-free): bad URL, non-http
    /// scheme, hostless URL, empty model, unknown preset, keyless cloud,
    /// custom without base_url — every refusal is a typed contract error
    /// BEFORE any state change (revision untouched, no file written).
    #[test]
    fn provider_invalid_config_is_typed_and_writes_nothing() {
        let fx = fixture();

        let bad_cases: Vec<(ProviderInstanceUpsertRequest, ContractErrorCode, &str)> = vec![
            (
                {
                    let mut r = upsert_req("openai", "gpt-5", Some("notaurl"));
                    r.secret = Some(SECRET.into());
                    r
                },
                ContractErrorCode::InvalidConfig,
                "not a valid URL",
            ),
            (
                {
                    let mut r = upsert_req("openai", "gpt-5", Some("ftp://mirror.example/v1"));
                    r.secret = Some(SECRET.into());
                    r
                },
                ContractErrorCode::InvalidConfig,
                "must use http",
            ),
            (
                {
                    let mut r = upsert_req("openai", "gpt-5", Some("https://"));
                    r.secret = Some(SECRET.into());
                    r
                },
                ContractErrorCode::InvalidConfig,
                "no host|not a valid URL",
            ),
            (
                {
                    let mut r = upsert_req("openai", "   ", None);
                    r.secret = Some(SECRET.into());
                    r
                },
                ContractErrorCode::InvalidConfig,
                "model must not be empty",
            ),
            (
                {
                    let mut r = upsert_req("skynet", "gpt-5", None);
                    r.secret = Some(SECRET.into());
                    r
                },
                ContractErrorCode::ContractViolation,
                "unknown provider preset",
            ),
            (
                // Cloud preset, NO secret anywhere → invalid_config, nothing written.
                upsert_req("openai", "gpt-5", None),
                ContractErrorCode::InvalidConfig,
                "no API key",
            ),
            (
                upsert_req("custom", "my-model", None),
                ContractErrorCode::InvalidConfig,
                "requires a base_url",
            ),
        ];
        for (req, expected_code, expected_msg_part) in bad_cases {
            let err = fx.manager.provider_instance_upsert(&req).unwrap_err();
            assert_eq!(err.code, expected_code, "case: {err}");
            let re = regex::Regex::new(expected_msg_part).unwrap();
            assert!(
                re.is_match(&err.message),
                "message `{}` must match `{expected_msg_part}`",
                err.message
            );
        }

        // The revision never moved and the settings file never appeared:
        // refusals are pre-state, not post-state cleanup.
        assert_eq!(fx.manager.provider_instance_list().unwrap().revision, 0);
        assert!(!settings_path(&fx.manager).exists());
    }

    /// PROOF (validate op): the same form rules WITHOUT network and without
    /// a secret on the wire; the problems list names each defect.
    #[test]
    fn provider_validate_reports_problems_without_network() {
        let fx = fixture();
        let ok = fx
            .manager
            .provider_instance_validate(&ProviderInstanceValidateRequest {
                preset: "custom".into(),
                model: "my-model".into(),
                base_url: Some("https://llm.example.internal/v1".into()),
                local: None,
                has_key: true,
            })
            .unwrap();
        assert!(ok.ok);
        assert!(ok.problems.is_empty());

        let bad = fx
            .manager
            .provider_instance_validate(&ProviderInstanceValidateRequest {
                preset: "custom".into(),
                model: String::new(),
                base_url: None,
                local: None,
                has_key: false,
            })
            .unwrap();
        assert!(!bad.ok);
        assert_eq!(bad.problems.len(), 3, "{:?}", bad.problems);
    }

    /// PROOF (lost-update guard): a stale expected_revision refuses the
    /// whole settings mutation with stale_revision.
    #[test]
    fn provider_upsert_refuses_stale_revision() {
        let fx = fixture();
        let mut req = upsert_req("ollama", "qwen3", None);
        req.local = Some(true);
        fx.manager.provider_instance_upsert(&req).unwrap();
        let mut second = upsert_req("ollama", "qwen3", None);
        second.expected_revision = Some(0);
        let err = fx.manager.provider_instance_upsert(&second).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::StaleRevision);
    }

    /// PROOF (restart persistence of the CONFIG + key presence): a NEW
    /// session manager over the same managed root sees the instances and
    /// their live key presence — metadata from the settings file, key
    /// presence from the (Arc-shared) sink that outlives the manager.
    #[test]
    fn provider_instances_persist_across_session_restart() {
        let fx = fixture();
        let mut req = upsert_req("anthropic", "claude-sonnet-4-5", None);
        req.secret = Some(SECRET.into());
        let created = fx.manager.provider_instance_upsert(&req).unwrap();
        let id = created.instance.id.clone();
        drop(fx.manager);

        // Fresh manager, SAME root and sink — the "restart" (in-process
        // mirror; the OS-level restart proof lives in the keychain lane).
        let manager =
            ProjectSessionManager::new_with_secret_sink(fx._dir.path(), fx.sink.clone()).unwrap();
        let list = manager.provider_instance_list().unwrap();
        assert_eq!(list.total, 1);
        let instance = &list.instances[0];
        assert_eq!(instance.id, id);
        assert!(instance.has_key, "key presence survives the restart");
        assert_eq!(instance.model, "claude-sonnet-4-5");
        let wire = serde_json::to_string(&list).unwrap();
        assert!(!wire.contains(SECRET));
    }

    /// PROOF (no-sink honesty): without a keychain sink, an upsert WITH a
    /// secret is a typed unsupported_capability — never a silent in-memory
    /// or on-disk secret.
    #[test]
    fn provider_upsert_without_keychain_sink_is_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let manager = ProjectSessionManager::new(dir.path()).unwrap();
        let mut req = upsert_req("openai", "gpt-5", None);
        req.secret = Some(SECRET.into());
        let err = manager.provider_instance_upsert(&req).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::UnsupportedCapability);
        // Nothing leaked anywhere.
        assert!(!serde_json::to_string(&err).unwrap().contains(SECRET));
        assert!(!dir.path().join("settings").join("providers.json").exists());
    }

    /// PROOF (corrupt settings honesty): a settings file corrupted BEFORE a
    /// session loads it is a typed failure on EVERY provider op, and no
    /// write ever happens over it (nothing is silently destroyed).
    #[test]
    fn provider_corrupt_settings_file_is_typed_failure_and_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings").join("providers.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // A file unreadable at LOAD time: the fresh session refuses every op.
        std::fs::write(&path, b"{ this is not json").unwrap();
        let sink = Arc::new(MemorySecretSink::default());
        let manager = ProjectSessionManager::new_with_secret_sink(dir.path(), sink).unwrap();

        let err = manager.provider_instance_list().unwrap_err();
        assert_eq!(err.code, ContractErrorCode::SaveFailed);
        assert!(err.message.contains("unreadable"));
        let mut req = upsert_req("ollama", "qwen3", None);
        req.local = Some(true);
        let err = manager.provider_instance_upsert(&req).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::SaveFailed);
        assert!(err.message.contains("unreadable"));

        // The corrupt bytes are still exactly there — nothing deleted,
        // nothing "healed" behind the user's back.
        assert_eq!(std::fs::read(&path).unwrap(), b"{ this is not json");
    }

    /// PROOF (external-change guard): a settings file changed OUTSIDE the
    /// session (here: a hand edit between load and save) refuses the write
    /// with `project_changed_on_disk` — a stale in-memory state never
    /// clobbers the durable file, the same discipline as managed projects.
    #[test]
    fn provider_upsert_refuses_externally_changed_settings_file() {
        let fx = fixture();
        let mut req = upsert_req("ollama", "qwen3", None);
        req.local = Some(true);
        let _ = fx.manager.provider_instance_upsert(&req).unwrap();
        let path = settings_path(&fx.manager);

        // External write BETWEEN load and save (valid container, different
        // content — as another tool or a hand edit would produce).
        let external: ProviderSettingsFile = ProviderSettingsFile {
            revision: 42,
            ..serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
        };
        std::fs::write(&path, serde_json::to_vec_pretty(&external).unwrap()).unwrap();

        let mut second = upsert_req("ollama", "qwen2.5:7b", None);
        second.local = Some(true);
        let err = fx.manager.provider_instance_upsert(&second).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ProjectChangedOnDisk);

        // The external content is still exactly on disk.
        let reread: ProviderSettingsFile =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(reread.revision, 42);
    }

    /// The keychain-backed sink over the TEST namespace obeys the same
    /// trait contract end-to-end (feature-gated; skipped without the
    /// `keychain` feature).
    #[cfg(feature = "keychain")]
    #[test]
    fn provider_keychain_sink_roundtrip_through_contract() {
        let dir = tempfile::tempdir().unwrap();
        let service = rimloc_llm::secrets::TEST_SERVICE;
        let sink = Arc::new(KeychainSecretSink::with_service(service));
        let manager = ProjectSessionManager::new_with_secret_sink(dir.path(), sink).unwrap();

        let mut req = upsert_req("zai", "glm-4.7", None);
        req.secret = Some(SECRET.into());
        let created = manager.provider_instance_upsert(&req).unwrap();
        let id = created.instance.id.clone();

        // The REAL OS keychain (test namespace) now holds the key under the
        // convention service/account pair.
        assert_eq!(
            rimloc_llm::secrets::get_secret_in(service, &id)
                .unwrap()
                .as_deref(),
            Some(SECRET)
        );

        // Restart: a fresh manager + fresh sink over the same root sees the
        // key presence from the OS keychain.
        let manager2 = ProjectSessionManager::new_with_secret_sink(
            dir.path(),
            Arc::new(KeychainSecretSink::with_service(service)),
        )
        .unwrap();
        let list = manager2.provider_instance_list().unwrap();
        assert!(list.instances[0].has_key);
        let wire = serde_json::to_string(&list).unwrap();
        assert!(!wire.contains(SECRET));

        // Delete removes BOTH the metadata and the OS keychain entry.
        let del = manager2
            .provider_instance_delete(&ProviderInstanceDeleteRequest {
                instance_id: id.clone(),
                expected_revision: None,
            })
            .unwrap();
        assert!(del.key_removed);
        assert_eq!(
            rimloc_llm::secrets::get_secret_in(service, &id).unwrap(),
            None
        );
    }

    // -- helpers ------------------------------------------------------------

    fn windows_contains(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len().max(1)).any(|w| w == needle)
    }
}
