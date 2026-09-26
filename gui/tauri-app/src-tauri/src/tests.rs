// Enforcement + smoke tests for the Tauri shell (binding wave 2).
// Wired back into the build in this wave: the file had been dead since
// 7fc196f (moved here without a `mod tests;` declaration) and its legacy
// smoke bodies rotted against removed api_* functions — they are gone.
#[cfg(test)]
// Named `shell` (not `tests`) to avoid a module named like its containing
// file-module (clippy module_inception).
mod shell {
    use crate::{LEGACY_PRIVILEGED_COMMANDS, LIVE_COMMANDS};

    /// The ORIGINAL registration (pre-binding-wave main.rs) — the partition
    /// base for the wave-2 deregistration enforcement (lead decision 033 #4).
    const ORIGINAL_REGISTERED: &[&str] = &[
        "get_app_info",
        "scan_mod",
        "learn_defs",
        "export_po",
        "validate_mod",
        "xml_health",
        "import_po",
        "build_mod",
        "diff_xml_cmd",
        "lang_update_cmd",
        "annotate_cmd",
        "init_lang_cmd",
        "get_log_info",
        "pick_directory",
        "save_text_via_dialog",
        "log_message",
        "open_path",
        "set_debug_options",
        "get_diagnostics",
        "collect_diagnostics_via_dialog",
        "simulate_error",
        "simulate_panic",
        "morph_cmd",
        "learn_keyed_cmd",
        "dump_schemas",
        "get_profile",
        "validate_po_gui",
        "learn_patches_cmd",
        "get_cli_i18n",
        "apply_translation",
        "load_tm",
        "scan_strings_gui",
        "load_plugin_cmd",
        "list_plugins_cmd",
        "coverage_gui",
        "export_xliff_gui",
        "import_xliff_gui",
        "merge_keyed_gui",
    ];

    #[test]
    fn live_entry_registers_every_contract_command() {
        for name in rimloc_gui_lib::contract_adapter::CONTRACT_COMMANDS {
            assert!(
                LIVE_COMMANDS.contains(name),
                "contract command `{name}` missing from the live registration"
            );
        }
    }

    #[test]
    fn live_entry_never_registers_privileged_legacy_commands() {
        for name in LEGACY_PRIVILEGED_COMMANDS {
            assert!(
                !LIVE_COMMANDS.contains(name),
                "privileged legacy command `{name}` must not be in the live registration"
            );
        }
    }

    #[test]
    fn live_and_legacy_sets_partition_the_original_registration() {
        for name in ORIGINAL_REGISTERED {
            let in_live = LIVE_COMMANDS.contains(name);
            let in_legacy = LEGACY_PRIVILEGED_COMMANDS.contains(name);
            assert!(
        in_live ^ in_legacy,
        "command `{name}` must live in exactly one registration set (live={in_live}, legacy={in_legacy})"
      );
        }
        assert_eq!(
            LIVE_COMMANDS.len() + LEGACY_PRIVILEGED_COMMANDS.len(),
            ORIGINAL_REGISTERED.len() + rimloc_gui_lib::contract_adapter::CONTRACT_COMMANDS.len()
        );
    }

    #[test]
    fn supplementary_regex_no_privileged_pattern_in_live() {
        let live = LIVE_COMMANDS.join("\n");
        for pattern in [
            "apply_translation",
            "save_text",
            "open_path",
            "plugin",
            "simulate_",
            "morph_cmd",
            "dump_schemas",
            "get_profile",
            "load_tm",
        ] {
            assert!(
                !live.contains(pattern),
                "live registration leaks `{pattern}`"
            );
        }
    }

    /// P2-1/P2-2 (J cross-review): dump_schemas writes 5 schema files into a
    /// caller-chosen directory; get_profile returns raw profile.jsonl records
    /// — both are privileged legacy now, never in the live entry.
    #[test]
    fn dump_schemas_and_get_profile_are_privileged_legacy() {
        for name in ["dump_schemas", "get_profile"] {
            assert!(!LIVE_COMMANDS.contains(&name), "`{name}` must not be live");
            assert!(
                LEGACY_PRIVILEGED_COMMANDS.contains(&name),
                "`{name}` must be in the privileged legacy set"
            );
        }
    }

    /// Night audit P1-2: load_tm reads ANY caller-chosen baseline PO and
    /// walkdir-walks ANY caller-chosen tm_roots with no root containment or
    /// symlink checks — arbitrary read, same exposure class as open_path.
    /// The live entry (and therefore frontend-v2 via the one transport seam)
    /// must never reach it; it returns with root guards via the contract TM
    /// slice.
    #[test]
    fn load_tm_is_privileged_legacy_arbitrary_read() {
        assert!(
            !LIVE_COMMANDS.contains(&"load_tm"),
            "load_tm must not be live: unrestricted caller-chosen paths"
        );
        assert!(
            LEGACY_PRIVILEGED_COMMANDS.contains(&"load_tm"),
            "load_tm must be in the privileged legacy set"
        );
    }
}
