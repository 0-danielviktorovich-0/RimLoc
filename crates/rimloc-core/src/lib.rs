use color_eyre::eyre::eyre;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Workspace-wide result alias.
pub type Result<T> = color_eyre::eyre::Result<T>;

/// Schema version for RimLoc data outputs (JSON/PO headers).
pub const RIMLOC_SCHEMA_VERSION: u32 = 1;

pub mod placeholders {
    /// Return true if a percent placeholder looks suspicious (e.g., single '%' not matching printf pattern).
    pub fn is_bad_percent(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                // literal '%%' is ok
                if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                    i += 2;
                    continue;
                }
                // Accept printf-like tokens: %d, %s, %i, %f with optional position/zero/width
                // This is a light check consistent with the validator logic.
                let mut j = i + 1;
                // optional positional like 1$
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'$' {
                    j += 1;
                }
                // optional zero or width digits
                while j < bytes.len() && (bytes[j] == b'0' || bytes[j].is_ascii_digit()) {
                    j += 1;
                }
                if j < bytes.len() && matches!(bytes[j] as char, 'd' | 's' | 'i' | 'f') {
                    i = j + 1;
                    continue;
                }
                return true;
            }
            i += 1;
        }
        false
    }
}

/// Minimal unit used across crates to represent a single translation entry
/// scanned from RimWorld XML (Keyed/DefInjected) or produced by tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransUnit {
    pub key: String,
    /// Source string (may be missing for keys detected without text)
    pub source: Option<String>,
    /// Absolute or relative path to the file where this unit comes from.
    /// For Defs-derived units this is the canonical DefInjected OUTPUT path
    /// (export grouping); the real source location rides in [`TransUnit::src`].
    pub path: PathBuf,
    /// 1-based line number if available
    pub line: Option<usize>,
    /// TKey provenance for units extracted from TKey-attributed XML nodes.
    /// `key` stays the LOGICAL identity `<defName>.<TKey>`; the proven
    /// serialization detail lives here (typed metadata, not key-shape).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tkey: Option<TKeyMeta>,
    /// Real source location when `path` is a canonical/virtual OUTPUT path.
    /// Defs-derived units are bridged to a DefInjected output path for export
    /// grouping; this keeps the effective source file (and the
    /// parser-guaranteed line) for the Source Inspector. Never fabricated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<SourceRef>,
    /// Why this occurrence is the effective winner — winner-reason
    /// provenance from [`winner_reason`], set by the scan pipeline at the
    /// decision point (Gate H semantics, Source Inspector mandate §7).
    /// `None` = no precedence decision involved this unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_by: Option<String>,
    /// The unit comes from an `IfModActive` (game-state dependent)
    /// LoadFolders dir: included in the documented offline superset but
    /// unresolved offline, so the effective view stays POTENTIAL.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conditional: bool,
}

impl Default for TransUnit {
    fn default() -> Self {
        Self {
            key: String::new(),
            source: None,
            path: PathBuf::new(),
            line: None,
            tkey: None,
            src: None,
            selected_by: None,
            conditional: false,
        }
    }
}

/// Real source location (file + parser-guaranteed line) behind a canonical
/// unit. No fabricated line numbers: `line` is `None` unless the parser
/// computed it (Source Inspector mandate §1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SourceRef {
    pub file: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// Winner-reason vocabulary for per-entry source provenance. The scan
/// pipeline is the single producer; the canonical
/// `SourceProvenance.selected_by` mirrors these exact strings, and the
/// view-selection facts ("version-selected"/"loadfolders") stay in
/// `version_selected`/`conditional_branch` of the same provenance record.
pub mod winner_reason {
    /// Defs/TKey duplicate identity: the FIRST registration wins
    /// (GetDefSilentFail semantics).
    pub const DEFS_FIRST_FILE: &str = "first-file-wins";
    /// Keyed duplicate across files: LAST loaded file wins (SetOrAdd).
    pub const KEYED_LAST_FILE: &str = "keyed-last-wins";
    /// Keyed duplicate within one file: the game errors and takes the FIRST.
    pub const KEYED_FIRST_IN_FILE: &str = "keyed-first-in-file";
    /// TKey identity shared by several nodes of one file: the LAST field
    /// assignment in document order wins (contexts > 1).
    pub const TKEY_LAST_ASSIGNMENT: &str = "tkey-last-assignment";
    /// DefInjected duplicate: SetOrAdd overwrite, last occurrence wins.
    pub const DEFINJECTED_SETORADD: &str = "definjected-setoradd";
    /// A patch operation produced/replaced this value.
    pub const PATCH_APPLIED: &str = "patch-applied";
    /// The LoadFolders.xml view resolution chose the content roots.
    pub const LOADFOLDERS: &str = "loadfolders";
    /// A version directory/tag selection chose this content root.
    pub const VERSION_SELECTED: &str = "version-selected";
}

/// Typed TKey metadata (general form — never a growing special-case field):
/// - `strategy` is the proven serialization class observed for the identity
///   ("bare" for TipSetDef `li`, "slate_ref" for direct SlateRef fields,
///   "parms_value_slate_ref" for `<parms>` descendants of QuestNode_SubScript);
/// - `suffix` is the DefInjected path suffix implied by that strategy
///   ("" / ".slateRef" / ".value.slateRef") — the primary proven path is
///   `key + suffix`;
/// - `contexts` counts source nodes sharing this identity (RimWorld
///   field-assignment semantics: the LAST node in document order wins;
///   a value above one means the pack shares one identity —
///   see DLC-TKEY-ADJUDICATION §5).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
pub struct TKeyMeta {
    pub strategy: String,
    pub suffix: String,
    /// Owning Def type (e.g. "QuestScriptDef") — provenance needed to
    /// reconstruct the DefInjected output location on round-trip.
    pub def_type: String,
    #[serde(default)]
    pub contexts: u32,
    /// Real per-node source locations of the shared identity, in document
    /// order (Source Inspector mandate §14: Primary location + Other
    /// usages). The LAST entry is the effective one (last field assignment
    /// wins) and mirrors `TransUnit.src`; earlier entries are other usages
    /// whose text was overwritten. Only same-file, accepted (non-duplicate)
    /// nodes are recorded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<SourceRef>,
}

/// Simple PO entry used by import/export utilities and tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoEntry {
    pub key: String,
    pub value: String,
    /// Optional reference like
    /// "…/Languages/English/Keyed/Some.xml:42" used to reconstruct paths.
    pub reference: Option<String>,
}

/// Keep a lightweight error type for crates that still import it.
#[derive(Debug, Error)]
pub enum RimLocError {
    #[error("{0}")]
    Other(String),
}

/// Parse a minimal subset of PO syntax used across the workspace.
/// Supports single-line `msgid`/`msgstr` pairs and optional reference lines (`#: ...`).
pub fn parse_simple_po(input: &str) -> Result<Vec<PoEntry>> {
    let mut entries = Vec::new();
    let mut cur_ref: Option<String> = None;
    let mut cur_id: Option<String> = None;

    fn unquote(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#:") {
            cur_ref = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("msgid") {
            let eq = rest
                .trim_start()
                .strip_prefix(' ')
                .unwrap_or(rest)
                .trim_start_matches('=');
            cur_id = Some(unquote(eq.trim()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("msgstr") {
            let eq = rest
                .trim_start()
                .strip_prefix(' ')
                .unwrap_or(rest)
                .trim_start_matches('=');
            let val = unquote(eq.trim());
            if let Some(id) = cur_id.take() {
                entries.push(PoEntry {
                    key: id,
                    value: val,
                    reference: cur_ref.take(),
                });
            } else {
                return Err(eyre!("Malformed PO entry: msgstr without msgid"));
            }
        }
    }

    Ok(entries)
}
