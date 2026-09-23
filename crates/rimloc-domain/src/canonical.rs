//! Canonical project model (Gate I2) — the ONE typed domain model.
//!
//! Design authority: docs/development/CANONICAL_PROJECT_MODEL.md and the
//! canonical model mandates. Key invariants:
//! - A source entry identity is TARGET-LOCALE INDEPENDENT: the same entry can
//!   carry an EN→RU and an EN→JA translation without becoming two entries.
//! - One logical identity may have MULTIPLE source contexts (RimWorld
//!   duplicate semantics); exactly one is the effective winner, the rest are
//!   preserved as diagnostics.
//! - Every entry can explain WHERE its content came from (provenance):
//!   raw source, version-selected root, conditional LoadFolders branch,
//!   patch-transformed, overridden context.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable identity of a source entry. `kind` + `key` must be unique inside a
/// project for one source version; locale is deliberately absent.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct SourceEntryId {
    pub kind: EntryKind,
    /// Logical key: Keyed key, `<defName>.<field>`, or `<defName>.<TKey>`.
    pub key: String,
}

/// First-party localization mechanism families (extensible; variants only
/// where BEHAVIOUR differs, not for cosmetic categorisation).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Keyed,
    DefInjected,
    /// TKey-attributed def field (system exists since RimWorld 1.1).
    TKey,
    /// Strings/*.txt and RulePack text families.
    Strings,
    Backstories,
    /// Content produced by patch operations rather than raw Defs XML.
    PatchDerived,
}

/// One source occurrence of a logical identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceContext {
    pub file: String,
    pub line: Option<usize>,
    /// Owning def type when known (e.g. "QuestScriptDef").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub def_type: Option<String>,
    pub role: ContextRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextRole {
    /// The value RimWorld actually uses (effective precedence winner).
    Effective,
    /// An overridden/losing occurrence — diagnostic evidence only.
    Overridden,
}

/// Provenance of the source state an entry represents (Gate H/I refinement).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceProvenance {
    /// Content root selected via version/LoadFolders resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_selected: Option<String>,
    /// Reached through a conditional LoadFolders branch (IfModActive etc.).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conditional_branch: bool,
    /// Patch stage coverage that produced this content.
    #[serde(default)]
    pub patch_stage: PatchStage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatchStage {
    #[default]
    None,
    /// Fully evaluated by the supported subset.
    Applied,
    /// Some operations unsupported — content may be stale (POTENTIAL view).
    Partial,
}

/// The canonical source unit. `id` is target-independent; translations live
/// in [`Translation`] keyed by locale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceEntry {
    pub id: SourceEntryId,
    /// Source text of the EFFECTIVE context.
    pub text: String,
    /// Source locale (usually "en" for mods; never a target locale).
    pub source_locale: String,
    pub contexts: Vec<SourceContext>,
    pub provenance: SourceProvenance,
    /// TKey serialization metadata when kind == EntryKind::TKey.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tkey: Option<rimloc_core::TKeyMeta>,
}

/// Translated state for one (source entry, target locale) pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Translation {
    pub source_id: SourceEntryId,
    pub locale: String,
    pub text: Option<String>,
    /// Dimensions instead of one giant enum (mandate 2 §7): completeness ×
    /// review × validation × lifecycle compose without impossible states.
    pub completeness: Completeness,
    pub review: Review,
    pub validation: ValidationState,
    pub lifecycle: Lifecycle,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Set when the source text changed after this translation was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_changed: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Untranslated,
    Translated,
    /// Literal TODO placeholder — RimWorld parity: counts as missing.
    Todo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    None,
    Pending,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    Unknown,
    Ok,
    Issues(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Active,
    /// Source no longer exists — entry kept as diagnostic, not exported.
    Obsolete,
    /// More than one plausible source identity (registry ambiguity).
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Unknown,
    Human,
    Tm,
    Llm,
    Imported,
}

/// Where the scan context came from (Gate H refinement: exact vs potential).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InventoryContext {
    pub target_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_dlc: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_mods: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub load_order: Vec<String>,
    /// EXACT only when active-mod context is known and patch coverage full;
    /// otherwise the inventory is an honest POTENTIAL/CONDITIONAL superset.
    pub view: ViewLabel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewLabel {
    Exact,
    #[default]
    Potential,
}

/// The canonical project: one source inventory + per-locale translations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    pub context: InventoryContext,
    pub entries: Vec<SourceEntry>,
    pub translations: Vec<Translation>,
}

impl Project {
    pub fn translation(&self, id: &SourceEntryId, locale: &str) -> Option<&Translation> {
        self.translations
            .iter()
            .find(|t| t.source_id == *id && t.locale == locale)
    }

    /// The single write path the GUI must use (no PO, no files).
    pub fn update_translation(
        &mut self,
        id: SourceEntryId,
        locale: &str,
        text: Option<String>,
        origin: Origin,
    ) -> &mut Translation {
        let completeness = match text.as_deref() {
            None => Completeness::Untranslated,
            Some(t) if t.trim().eq_ignore_ascii_case("TODO") => Completeness::Todo,
            Some(_) => Completeness::Translated,
        };
        if let Some(pos) = self
            .translations
            .iter()
            .position(|t| t.source_id == id && t.locale == locale)
        {
            let t = &mut self.translations[pos];
            t.text = text;
            t.completeness = completeness;
            t.origin = origin;
            return &mut self.translations[pos];
        }
        self.translations.push(Translation {
            source_id: id,
            locale: locale.to_string(),
            text,
            completeness,
            review: Review::None,
            validation: ValidationState::Unknown,
            lifecycle: Lifecycle::Active,
            origin,
            notes: String::new(),
            source_changed: None,
        });
        self.translations.last_mut().expect("just pushed")
    }

    /// Mark translations whose source text drifted (sourceChanged pipeline).
    pub fn mark_source_changed(&mut self, locale: &str) -> usize {
        let mut n = 0;
        for t in &mut self.translations {
            if t.locale != locale || t.text.is_none() {
                continue;
            }
            let Some(entry) = self.entries.iter().find(|e| e.id == t.source_id) else {
                continue;
            };
            if entry.text != t.text.as_deref().unwrap_or_default() {
                // Source drift means the entry text changed relative to what
                // was last translated; flag for review.
                if t.source_changed.is_none() {
                    t.source_changed = Some("source text changed".into());
                    t.review = Review::Pending;
                    n += 1;
                }
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str) -> SourceEntry {
        SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::TKey,
                key: key.into(),
            },
            text: "Hello".into(),
            source_locale: "en".into(),
            contexts: vec![SourceContext {
                file: "Defs/X.xml".into(),
                line: Some(3),
                def_type: Some("QuestScriptDef".into()),
                role: ContextRole::Effective,
            }],
            provenance: SourceProvenance {
                version_selected: Some("1.6".into()),
                conditional_branch: false,
                patch_stage: PatchStage::Applied,
            },
            tkey: None,
        }
    }

    /// Identity is target-independent: same id serves any locale.
    #[test]
    fn translation_is_scoped_by_locale_under_one_identity() {
        let e = entry("Q.Key");
        let mut p = Project {
            entries: vec![e],
            ..Default::default()
        };
        let id = SourceEntryId {
            kind: EntryKind::TKey,
            key: "Q.Key".into(),
        };
        p.update_translation(id.clone(), "ru", Some("Привет".into()), Origin::Human);
        p.update_translation(id.clone(), "ja", Some("こんにちは".into()), Origin::Human);
        assert_eq!(p.translations.len(), 2);
        assert!(p.translation(&id, "ru").is_some());
        assert!(p.translation(&id, "ja").is_some());
        // Still ONE source entry — not duplicated per locale.
        assert_eq!(p.entries.len(), 1);
    }

    #[test]
    fn update_translation_maps_todo_and_overwrites() {
        let e = entry("Q.Key");
        let mut p = Project {
            entries: vec![e],
            ..Default::default()
        };
        let id = SourceEntryId {
            kind: EntryKind::TKey,
            key: "Q.Key".into(),
        };
        p.update_translation(id.clone(), "ru", Some("TODO".into()), Origin::Tm);
        assert_eq!(
            p.translation(&id, "ru").unwrap().completeness,
            Completeness::Todo
        );
        let t = p.update_translation(id.clone(), "ru", Some("Готово".into()), Origin::Human);
        assert_eq!(t.completeness, Completeness::Translated);
        assert_eq!(p.translations.len(), 1, "no duplicate rows");
    }
}
