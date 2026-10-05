//! OS-keychain access for provider API keys (feature `keychain`) — the ONE
//! secret-persistence seam of the LLM subsystem.
//!
//! Discipline (mirrors `KeySource::Auto`):
//! - service = [`KEYCHAIN_SERVICE`] (`rimloc-llm`), account = provider
//!   instance id — so a key stored here for an instance is resolvable by the
//!   engine through the existing `Auto` path with no extra wiring;
//! - the secret value NEVER appears in an error message, a log line or a
//!   `Debug` output — errors carry only the account (a non-secret id);
//! - automated tests use [`TEST_SERVICE`], never the user's real entries;
//!   `RIMLOC_KEYCHAIN_SERVICE` overrides the production service name for
//!   sandboxed/automation runs that must not share the user's namespace.

use crate::LlmError;

/// Keychain service convention for the LLM subsystem. Same service the
/// `KeySource::Auto` resolution reads (`crates/rimloc-llm/src/lib.rs`).
pub const KEYCHAIN_SERVICE: &str = "rimloc-llm";

/// Reserved namespace for AUTOMATED TESTS only — production flows never
/// touch it, so CI runs stay out of the user's real `rimloc-llm` entries.
pub const TEST_SERVICE: &str = "rimloc-llm-selftest";

/// The effective production service name: `RIMLOC_KEYCHAIN_SERVICE` overrides
/// [`KEYCHAIN_SERVICE`] (sandboxed/automation isolation), otherwise the
/// convention applies.
pub fn effective_service() -> String {
    match std::env::var("RIMLOC_KEYCHAIN_SERVICE") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => KEYCHAIN_SERVICE.to_string(),
    }
}

fn entry(service: &str, account: &str) -> Result<keyring::Entry, LlmError> {
    keyring::Entry::new(service, account)
        .map_err(|e| LlmError::Keychain(format!("keychain entry `{service}/{account}`: {e}")))
}

/// Store (or overwrite) a secret under `service/account`. Empty secrets are
/// refused BEFORE any keychain write (an empty credential is a config bug,
/// not a stored state).
pub fn set_secret_in(service: &str, account: &str, secret: &str) -> Result<(), LlmError> {
    if secret.is_empty() {
        return Err(LlmError::Keychain(
            "refusing to store an empty secret".to_string(),
        ));
    }
    entry(service, account)?
        .set_password(secret)
        .map_err(|e| LlmError::Keychain(format!("keychain set for `{account}`: {e}")))
}

/// Read a secret; `Ok(None)` = no entry (a typed absence, not an error).
pub fn get_secret_in(service: &str, account: &str) -> Result<Option<String>, LlmError> {
    match entry(service, account)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(LlmError::Keychain(format!(
            "keychain read for `{account}`: {e}"
        ))),
    }
}

/// Delete a secret; `Ok(true)` = it existed and is gone, `Ok(false)` =
/// there was nothing to delete.
pub fn delete_secret_in(service: &str, account: &str) -> Result<bool, LlmError> {
    match entry(service, account)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(LlmError::Keychain(format!(
            "keychain delete for `{account}`: {e}"
        ))),
    }
}

/// Presence probe WITHOUT returning the value (the contract layer only ever
/// learns whether a key exists, never what it is).
pub fn has_secret_in(service: &str, account: &str) -> Result<bool, LlmError> {
    match entry(service, account)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(LlmError::Keychain(format!(
            "keychain probe for `{account}`: {e}"
        ))),
    }
}

// Convenience wrappers over the CONVENTION service (`rimloc-llm`).

pub fn set_secret(account: &str, secret: &str) -> Result<(), LlmError> {
    set_secret_in(&effective_service(), account, secret)
}

pub fn get_secret(account: &str) -> Result<Option<String>, LlmError> {
    get_secret_in(&effective_service(), account)
}

pub fn delete_secret(account: &str) -> Result<bool, LlmError> {
    delete_secret_in(&effective_service(), account)
}

pub fn has_secret(account: &str) -> Result<bool, LlmError> {
    has_secret_in(&effective_service(), account)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-run account so repeated test runs never collide inside the
    /// TEST namespace.
    fn unique_account(tag: &str) -> String {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        format!(
            "lane-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }

    /// Drops the keychain entry even when an assertion fails — the test
    /// namespace stays clean no matter how the body exits.
    struct Cleanup<'a> {
        service: &'a str,
        account: &'a str,
    }
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = delete_secret_in(self.service, self.account);
        }
    }

    /// PROOF (keychain roundtrip): set → get returns the exact value →
    /// has = true → delete → get = None → has = false. Real OS keychain,
    /// TEST namespace only. На linux CI нет Secret Service/dbus — keyring
    /// остаётся моком (решение по linux-упаковке отдельное, см. blockers
    /// keychain-лейна), поэтому доказательство живёт на macOS/windows.
    #[test]
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn keychain_roundtrip_write_read_delete() {
        let account = unique_account("roundtrip");
        let _guard = Cleanup {
            service: TEST_SERVICE,
            account: &account,
        };
        assert!(!has_secret_in(TEST_SERVICE, &account).unwrap());
        set_secret_in(TEST_SERVICE, &account, "sk-test-secret-0123456789").unwrap();
        assert!(has_secret_in(TEST_SERVICE, &account).unwrap());
        let value = get_secret_in(TEST_SERVICE, &account).unwrap();
        assert_eq!(value.as_deref(), Some("sk-test-secret-0123456789"));
        assert!(delete_secret_in(TEST_SERVICE, &account).unwrap());
        assert_eq!(get_secret_in(TEST_SERVICE, &account).unwrap(), None);
        assert!(!has_secret_in(TEST_SERVICE, &account).unwrap());
    }

    /// Deleting an absent entry is an honest `Ok(false)`, never an error —
    /// delete of an already-removed provider must not look like a failure.
    #[test]
    fn keychain_delete_of_absent_entry_is_false_not_error() {
        let account = unique_account("absent");
        assert!(!delete_secret_in(TEST_SERVICE, &account).unwrap());
    }

    /// Empty secrets are refused before any keychain access.
    #[test]
    fn keychain_refuses_empty_secret() {
        let account = unique_account("empty");
        let err = set_secret_in(TEST_SERVICE, &account, "").unwrap_err();
        assert!(err.to_string().contains("empty secret"), "{err}");
        assert!(!has_secret_in(TEST_SERVICE, &account).unwrap());
    }

    // The cross-PROCESS restart proof lives in
    // `crates/rimloc-llm/tests/keychain_restart.rs` (two separate
    // cargo-test processes, TEST namespace only).
}
