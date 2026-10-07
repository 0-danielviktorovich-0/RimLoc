//! `rimloc provider-test` — connectivity/auth probe for a provider.
//!
//! Wires the previously dead [`rimloc_llm::provider::Provider::test_connection`]
//! to the CLI: one tiny "reply with ok" round-trip that validates endpoint
//! reachability, auth and model availability without touching any mod.
//! Contract: exit 0 on success, exit 1 on failure — usable from scripts and
//! CI. The probe makes a REAL request (mock stays offline and is free);
//! output reports id/model/endpoint only, never key material.

pub fn run_provider_test(
    provider: String,
    model: Option<String>,
    base_url: Option<String>,
    key_env: Option<String>,
) -> color_eyre::Result<()> {
    let p = crate::commands::translate::build_provider(&provider, model, base_url, key_env)?;
    ui_info!("provider-test-probing", provider = provider.as_str());
    match p.test_connection() {
        Ok(info) => {
            ui_ok!(
                "provider-test-ok",
                provider = info.id.as_str(),
                model = info.model.as_str(),
                endpoint = info.endpoint.as_str()
            );
            Ok(())
        }
        Err(e) => {
            ui_err!(
                "provider-test-failed",
                provider = provider.as_str(),
                error = e.to_string()
            );
            std::process::exit(1);
        }
    }
}
