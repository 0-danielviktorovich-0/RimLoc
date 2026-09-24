//! Eligibility engine (Gate J implementation) — the single runtime that turns
//! the [`rimloc_domain::eligibility`] contract into verdicts over canonical
//! [`SourceEntry`] values.
//!
//! Responsibilities (TRANSLATION_ELIGIBILITY mandate):
//! - a DECLARATIVE builtin seed pack derived from the existing extraction
//!   defaults (allowlist dictionary + the NoTranslate technical-field family);
//! - user/project rule packs evaluated BEFORE the builtin fallback, while the
//!   precedence ladder keeps Deterministic/Verified NON_TRANSLATABLE final;
//! - unmatched entries go to REVIEW for human/AI adjudication (never silently
//!   Translatable);
//! - an explain JSON artifact for the future CLI/GUI surfaces.
//!
//! The engine is a struct, not a trait: there is exactly ONE decision
//! component; scan/GUI/LLM/compare/import must call it instead of deciding
//! themselves.

use rimloc_core::TransUnit;
use rimloc_domain::canonical::{EntryKind, PatchStage, SourceEntry};
use rimloc_domain::eligibility::{
    authority_of, resolve, Authority, Conflict, Decision, Evidence, EvidenceSource, Rule, Verdict,
    PRECEDENCE,
};
use serde::Deserialize;
use std::path::Path;

/// Current rule-pack schema version accepted by [`load_rule_pack`].
pub const RULE_PACK_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Builtin seed pack
// ---------------------------------------------------------------------------

/// Technical/identifier fields that must never be translated. Translating
/// them produces "unnecessary DefInjected" entries and breaks Def loading
/// (the game's `[NoTranslate]`/`[Unsaved]` family — GAME_SOURCE_FINDINGS §6,
/// RIMWORLD_REFERENCE_AUDIT §C). RimLoc extraction is allowlist-driven, so
/// these never enter the inventory from Defs — but DefInjected sidecar files
/// in the wild DO carry them, and the engine must reject such entries.
const NOTRANSLATE_FIELDS: &[&str] = &[
    "defName",       // def identity
    "packageId",     // mod identity (About.xml)
    "texPath",       // texture asset path
    "workerClass",   // C# class reference
    "defaultDamage", // numeric/stat reference
];

/// Universal human-facing field leaves (def-type agnostic fallback). Derived
/// from the extraction allowlist leaves (rimloc-parsers-xml
/// `assets/defs_fields.json`) minus context-dependent leaves ("name", "li",
/// "customSummary") that are only safe under a scoped path and therefore stay
/// in the def-type-scoped dictionary rules below.
const TRANSLATABLE_LEAVES: &[&str] = &[
    "label",
    "labelShort",
    "labelPlural",
    "labelMale",
    "labelFemale",
    "labelNoun",
    "title",
    "titleShort",
    "titleFemale",
    "titleShortFemale",
    "description",
    "baseDesc",
    "helpText",
    "reportString",
    "gerundLabel",
    "jobString",
    "letterLabel",
    "letterText",
    "deathMessage",
    "pawnSingular",
    "leaderTitle",
    "ingestCommandString",
];

/// The builtin seed pack: declarative [`Rule`] values compiled from the
/// existing extraction defaults. Deterministic order (kind rules, NoTranslate,
/// then the allowlist dictionary in sorted order, then universal leaves).
pub fn builtin_seed_rules() -> Vec<Rule> {
    let mut rules = Vec::new();

    // First-party localization mechanisms: TKey nodes and Keyed/LanguageData
    // entries exist specifically to be translated.
    rules.push(Rule {
        id: "builtin:translatable:kind:tkey".into(),
        def_type: None,
        field_path: None,
        package_id: None,
        entry_kind: Some("tkey".into()),
        decision: Decision::Translatable,
        provenance: EvidenceSource::BuiltInRule,
        reason: "first-party TKey mechanism exposes the node for translation (RimWorld 1.1+)"
            .into(),
        versions: Vec::new(),
    });
    rules.push(Rule {
        id: "builtin:translatable:kind:keyed".into(),
        def_type: None,
        field_path: None,
        package_id: None,
        entry_kind: Some("keyed".into()),
        decision: Decision::Translatable,
        provenance: EvidenceSource::BuiltInRule,
        reason: "Keyed/LanguageData nodes exist to be translated (first-party mechanism)".into(),
        versions: Vec::new(),
    });

    for field in NOTRANSLATE_FIELDS {
        rules.push(Rule {
            id: format!("builtin:notranslate:{field}"),
            def_type: None,
            field_path: Some((*field).into()),
            package_id: None,
            entry_kind: None,
            decision: Decision::NonTranslatable,
            provenance: EvidenceSource::BuiltInRule,
            reason: "technical/identifier field; translating it breaks Def loading \
                     ([NoTranslate]/[Unsaved] family, GAME_SOURCE_FINDINGS §6)"
                .into(),
            versions: Vec::new(),
        });
    }

    // Def-type-scoped rules from the extraction allowlist dictionary. The
    // dictionary is the same data the scanner uses (allowlist defaults), so
    // engine and extractor can never disagree about what was extracted.
    let dict = rimloc_parsers_xml::load_embedded_defs_dict();
    let mut pairs: Vec<(&String, &String)> = dict
        .0
        .iter()
        .flat_map(|(dt, fields)| fields.iter().map(move |f| (dt, f)))
        .collect();
    pairs.sort();
    pairs.dedup();
    for (def_type, field) in pairs {
        rules.push(Rule {
            id: format!("builtin:translatable:{def_type}:{field}"),
            def_type: Some(def_type.clone()),
            field_path: Some(field.clone()),
            package_id: None,
            entry_kind: None,
            decision: Decision::Translatable,
            provenance: EvidenceSource::BuiltInRule,
            reason: "extraction allowlist default (rimloc-parsers-xml assets/defs_fields.json)"
                .into(),
            versions: Vec::new(),
        });
    }

    for leaf in TRANSLATABLE_LEAVES {
        rules.push(Rule {
            id: format!("builtin:translatable:leaf:{leaf}"),
            def_type: None,
            field_path: Some((*leaf).into()),
            package_id: None,
            entry_kind: None,
            decision: Decision::Translatable,
            provenance: EvidenceSource::BuiltInRule,
            reason: "universal human-facing field pattern (leaf of the extraction allowlist)"
                .into(),
            versions: Vec::new(),
        });
    }

    rules
}

// ---------------------------------------------------------------------------
// Rule pack loading
// ---------------------------------------------------------------------------

/// Mirror of [`rimloc_domain::eligibility::Rule`] with
/// `deny_unknown_fields`: the contract type stays permissive (it is also a
/// serialization schema), while pack FILES are strict — a typo'd field must
/// fail loudly instead of silently narrowing a rule.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackRule {
    id: String,
    #[serde(default)]
    def_type: Option<String>,
    #[serde(default)]
    field_path: Option<String>,
    #[serde(default)]
    package_id: Option<String>,
    #[serde(default)]
    entry_kind: Option<String>,
    decision: Decision,
    provenance: EvidenceSource,
    reason: String,
    #[serde(default)]
    versions: Vec<String>,
}

impl From<PackRule> for Rule {
    fn from(p: PackRule) -> Self {
        Rule {
            id: p.id,
            def_type: p.def_type,
            field_path: p.field_path,
            package_id: p.package_id,
            entry_kind: p.entry_kind,
            decision: p.decision,
            provenance: p.provenance,
            reason: p.reason,
            versions: p.versions,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulePackFile {
    schema_version: u32,
    rules: Vec<PackRule>,
}

/// Load a versioned declarative rule pack:
/// `{"schema_version": 1, "rules": [...]}`. Unknown fields are rejected on
/// both the wrapper and every rule; decision/provenance values are validated
/// by serde against the contract enums; ids and reasons must be non-empty.
pub fn load_rule_pack(path: &Path) -> crate::Result<Vec<Rule>> {
    let file = std::fs::File::open(path)?;
    let pack: RulePackFile = serde_json::from_reader(file)?;
    if pack.schema_version != RULE_PACK_SCHEMA_VERSION {
        return Err(color_eyre::eyre::eyre!(
            "rule pack {}: unsupported schema_version {} (expected {})",
            path.display(),
            pack.schema_version,
            RULE_PACK_SCHEMA_VERSION
        ));
    }
    let mut out = Vec::with_capacity(pack.rules.len());
    for r in pack.rules {
        if r.id.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "rule pack {}: rule with empty id",
                path.display()
            ));
        }
        if r.reason.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "rule pack {}: rule '{}' has empty reason",
                path.display(),
                r.id
            ));
        }
        out.push(Rule::from(r));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The one eligibility decision component.
pub struct EligibilityEngine {
    builtin: Vec<Rule>,
    external: Vec<Rule>,
}

impl Default for EligibilityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EligibilityEngine {
    /// Engine with the builtin seed pack only (no user/project rules).
    pub fn new() -> Self {
        Self {
            builtin: builtin_seed_rules(),
            external: Vec::new(),
        }
    }

    /// Append a loaded user/project rule pack. External rules are evaluated
    /// BEFORE the builtin fallback; the precedence ladder (and the
    /// NoTranslate finality guard) still governs the outcome.
    pub fn with_rule_pack(mut self, rules: Vec<Rule>) -> Self {
        self.external.extend(rules);
        self
    }

    /// Decide the eligibility of one canonical source entry.
    pub fn evaluate(&self, entry: &SourceEntry, package_id: Option<&str>) -> Verdict {
        let mut candidates: Vec<Verdict> = Vec::new();
        // 1) User/project packs first — the ladder ranks them by provenance.
        for r in &self.external {
            if rule_matches(r, entry, package_id) {
                candidates.push(verdict_from_rule(r, entry, package_id));
            }
        }
        // 2) Builtin fallback. Order inside the pack matters only for
        //    equal-rank ties (same BuiltInRule precedence): kind rules beat
        //    NoTranslate (TKey/Keyed are explicit first-party mechanisms), and
        //    NoTranslate beats translatable patterns (safety default).
        for r in &self.builtin {
            if rule_matches(r, entry, package_id) {
                candidates.push(verdict_from_rule(r, entry, package_id));
            }
        }

        if candidates.is_empty() {
            return review_verdict(entry);
        }

        let mut verdict = resolve(candidates.clone());
        enforce_notranslate_finality(&mut verdict, &candidates);
        verdict
    }

    /// The explain artifact as JSON for future CLI/GUI consumers:
    /// decision / authority / evidence chain / diagnosed conflicts.
    pub fn explain_json(&self, entry: &SourceEntry, package_id: Option<&str>) -> serde_json::Value {
        let verdict = self.evaluate(entry, package_id);
        serde_json::to_value(&verdict).unwrap_or(serde_json::Value::Null)
    }
}

// ---------------------------------------------------------------------------
// Matching and resolution helpers
// ---------------------------------------------------------------------------

/// Known def type of an entry: TKey metadata wins, else the first context.
fn entry_def_type(entry: &SourceEntry) -> Option<&str> {
    entry
        .tkey
        .as_ref()
        .map(|m| m.def_type.as_str())
        .or_else(|| entry.contexts.first().and_then(|c| c.def_type.as_deref()))
}

/// Field part of a logical key: everything after the defName segment
/// ("Widget.tools.li.label" -> "tools.li.label"; a Keyed key is its own
/// field part and is normally decided by the kind rule anyway).
fn field_part(key: &str) -> &str {
    match key.split_once('.') {
        Some((_, rest)) => rest,
        None => key,
    }
}

/// Normalize one dictionary-style path to a comparable form: strip `{h}`-like
/// markers, `[...]` predicates and `a|b` aliases, and lowercase — the scanner
/// matches field names case-insensitively, so the engine must agree.
fn normalize_field_path(path: &str) -> String {
    path.split('.')
        .map(|seg| {
            let seg = seg.split('{').next().unwrap_or(seg);
            let seg = seg.split('[').next().unwrap_or(seg);
            seg.split('|').next().unwrap_or(seg).trim()
        })
        .filter(|seg| !seg.is_empty())
        .map(|seg| seg.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(".")
}

/// True when the rule's field path matches the entry's field part
/// (exact or suffix, so "label" matches "tools.li.label").
fn field_matches(rule_field: &str, key_field: &str) -> bool {
    let rule_fp = normalize_field_path(rule_field);
    let key_fp = normalize_field_path(key_field);
    if rule_fp.is_empty() || key_fp.is_empty() {
        return false;
    }
    key_fp == rule_fp || key_fp.ends_with(&format!(".{rule_fp}"))
}

/// Case/underscore-insensitive token compare for kind names, so packs may
/// write "tkey", "t_key" or "TKey" for [`EntryKind::TKey`].
fn kind_token_matches(rule_kind: &str, kind: EntryKind) -> bool {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| *c != '_')
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>()
    };
    let serde_name = serde_json::to_string(&kind).unwrap_or_default();
    let serde_name = serde_name.trim_matches('"');
    squash(rule_kind) == squash(serde_name)
}

/// All present selectors of the rule must match the entry.
fn rule_matches(rule: &Rule, entry: &SourceEntry, package_id: Option<&str>) -> bool {
    if let Some(p) = &rule.package_id {
        match package_id {
            Some(actual) if actual.eq_ignore_ascii_case(p) => {}
            _ => return false,
        }
    }
    if !rule.versions.is_empty() {
        match entry.provenance.version_selected.as_deref() {
            Some(v) if rule.versions.iter().any(|rv| rv == v) => {}
            _ => return false,
        }
    }
    if let Some(k) = &rule.entry_kind {
        if !kind_token_matches(k, entry.id.kind) {
            return false;
        }
    }
    if let Some(dt) = &rule.def_type {
        match entry_def_type(entry) {
            // Known def type must agree with the rule's scope.
            Some(edt) if edt.eq_ignore_ascii_case(dt) => {}
            // Def type unknown (ordinary DefInjected entries): a field-scoped
            // rule can still apply through its field path alone; an unscoped
            // one cannot be verified.
            None if rule.field_path.is_some() => {}
            None => return false,
            Some(_) => return false,
        }
    }
    if let Some(fp) = &rule.field_path {
        if !field_matches(fp, field_part(&entry.id.key)) {
            return false;
        }
    }
    true
}

fn verdict_from_rule(rule: &Rule, entry: &SourceEntry, package_id: Option<&str>) -> Verdict {
    Verdict {
        decision: rule.decision,
        authority: authority_of(rule.provenance),
        evidence: vec![Evidence {
            rule_id: rule.id.clone(),
            source: rule.provenance,
            detail: format!(
                "{} [key={}, kind={}, package={}]",
                rule.reason,
                entry.id.key,
                serde_json::to_string(&entry.id.kind).unwrap_or_default(),
                package_id.unwrap_or("-"),
            ),
        }],
        conflicts: Vec::new(),
    }
}

/// Nothing matched: the entry goes to adjudication, honestly UNKNOWN in
/// authority (never a fake heuristic "translatable").
fn review_verdict(entry: &SourceEntry) -> Verdict {
    Verdict {
        decision: Decision::Review,
        authority: Authority::Unknown,
        evidence: vec![Evidence {
            rule_id: "builtin:review:unmatched".into(),
            source: EvidenceSource::StructuralHeuristic,
            detail: format!(
                "no builtin or external rule matched; adjudication required [key={}, kind={}]",
                entry.id.key,
                serde_json::to_string(&entry.id.kind).unwrap_or_default(),
            ),
        }],
        conflicts: Vec::new(),
    }
}

fn rank_of(verdict: &Verdict) -> usize {
    verdict
        .evidence
        .first()
        .and_then(|e| PRECEDENCE.iter().position(|s| *s == e.source))
        .unwrap_or(PRECEDENCE.len())
}

/// NoTranslate protection (contract invariant): a Deterministic/Verified
/// NON_TRANSLATABLE is FINAL — a weaker or equal-rank TRANSLATABLE winner is
/// flipped back with a recorded conflict. A strictly higher-precedence
/// TRANSLATABLE (first-party MustTranslate semantics) stays the winner and
/// the clash is diagnosed instead of silently resolved.
fn enforce_notranslate_finality(verdict: &mut Verdict, candidates: &[Verdict]) {
    if verdict.decision == Decision::NonTranslatable {
        return;
    }
    let Some(nt) = candidates.iter().find(|c| {
        c.decision == Decision::NonTranslatable
            && matches!(c.authority, Authority::Deterministic | Authority::Verified)
    }) else {
        return;
    };
    let diagnose = |v: &mut Verdict, winner: String, loser: String, reason: &str| {
        // One diagnosis per clashing pair — resolve() may already have
        // recorded the same pair from the ladder pass.
        if v.conflicts
            .iter()
            .any(|c| c.winner_rule == winner && c.loser_rule == loser)
        {
            return;
        }
        v.conflicts.push(Conflict {
            winner_rule: winner,
            loser_rule: loser,
            reason: reason.into(),
        });
    };
    let nt_rank = rank_of(nt);
    let winner_rank = rank_of(verdict);
    if nt_rank <= winner_rank {
        if let Some(l) = verdict.evidence.first() {
            let winner = nt
                .evidence
                .first()
                .map(|e| e.rule_id.clone())
                .unwrap_or_default();
            diagnose(
                verdict,
                winner,
                l.rule_id.clone(),
                "deterministic/verified NON_TRANSLATABLE is final (NoTranslate protection)",
            );
        }
        verdict.decision = nt.decision;
        verdict.authority = nt.authority;
        verdict.evidence = nt.evidence.clone();
    } else if let (Some(w), Some(l)) = (
        verdict.evidence.first().map(|e| e.rule_id.clone()),
        nt.evidence.first().map(|e| e.rule_id.clone()),
    ) {
        diagnose(
            verdict,
            w,
            l,
            "higher-precedence TRANSLATABLE outranks NON_TRANSLATABLE (diagnosed)",
        );
    }
}

// ---------------------------------------------------------------------------
// Inventory wiring helper
// ---------------------------------------------------------------------------

/// Decide a whole scan-world inventory (Gate J wiring for validators that
/// speak [`TransUnit`], not canonical entries). Units are bridged to canonical
/// form by the ONE conversion point ([`crate::canonical_bridge`]), so the
/// engine sees exactly the identities (kind + collapsed duplicate semantics)
/// the rest of the pipeline sees.
///
/// This convenience constructs a builtin-only engine per call — fine for
/// batch diagnostics; full-featured consumers (GUI explain, project
/// workflows) should build [`SourceEntry`] values once via the bridge and
/// reuse one [`EligibilityEngine`] that also carries user/project rule packs.
pub fn evaluate_units(units: &[TransUnit]) -> Vec<(String, Verdict)> {
    let entries = crate::canonical_bridge::source_entries(units, PatchStage::None, None, None);
    let engine = EligibilityEngine::new();
    entries
        .iter()
        .map(|e| (e.id.key.clone(), engine.evaluate(e, None)))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rimloc_core::TKeyMeta;
    use rimloc_domain::canonical::{ContextRole, SourceContext, SourceEntryId, SourceProvenance};

    fn plain_entry(kind: EntryKind, key: &str) -> SourceEntry {
        SourceEntry {
            id: SourceEntryId {
                kind,
                key: key.into(),
            },
            text: "Hello".into(),
            source_locale: "en".into(),
            contexts: vec![SourceContext {
                file: "Defs/X.xml".into(),
                line: Some(3),
                def_type: None,
                role: ContextRole::Effective,
            }],
            provenance: SourceProvenance::default(),
            tkey: None,
        }
    }

    fn definj(key: &str) -> SourceEntry {
        plain_entry(EntryKind::DefInjected, key)
    }

    fn rule(id: &str, provenance: EvidenceSource, decision: Decision, field: &str) -> Rule {
        Rule {
            id: id.into(),
            def_type: None,
            field_path: Some(field.into()),
            package_id: None,
            entry_kind: None,
            decision,
            provenance,
            reason: "test rule".into(),
            versions: Vec::new(),
        }
    }

    /// MustTranslate-like first-party rule beats a user do-not-translate rule.
    /// Pack order mirrors the contract test: the weaker candidate first, so
    /// the ladder replacement diagnoses the clash.
    #[test]
    fn first_party_beats_user_rule() {
        let engine = EligibilityEngine::new().with_rule_pack(vec![
            rule(
                "user:do-not-translate",
                EvidenceSource::UserRule,
                Decision::NonTranslatable,
                "label",
            ),
            rule(
                "musttranslate:label",
                EvidenceSource::FirstPartyAttribute,
                Decision::Translatable,
                "label",
            ),
        ]);
        let v = engine.evaluate(&definj("Widget.label"), None);
        assert_eq!(v.decision, Decision::Translatable);
        assert_eq!(v.authority, Authority::Deterministic);
        assert!(
            v.conflicts
                .iter()
                .any(|c| c.loser_rule == "user:do-not-translate"),
            "{v:?}"
        );
    }

    /// Explicit NoTranslate is final: a user Translatable wish cannot flip a
    /// builtin NonTranslatable, and neither can a pack rule claiming builtin
    /// provenance (equal rank — resolved by the finality guard).
    #[test]
    fn notranslate_is_final() {
        let weaker = EligibilityEngine::new().with_rule_pack(vec![rule(
            "user:wants-texpath",
            EvidenceSource::UserRule,
            Decision::Translatable,
            "texPath",
        )]);
        let v = weaker.evaluate(&definj("Widget.texPath"), None);
        assert_eq!(v.decision, Decision::NonTranslatable);
        assert!(
            v.conflicts
                .iter()
                .any(|c| c.loser_rule == "user:wants-texpath"),
            "{v:?}"
        );

        let equal_rank = EligibilityEngine::new().with_rule_pack(vec![rule(
            "pack:claims-builtin",
            EvidenceSource::BuiltInRule,
            Decision::Translatable,
            "defName",
        )]);
        let v = equal_rank.evaluate(&definj("Widget.defName"), None);
        assert_eq!(v.decision, Decision::NonTranslatable);
        assert!(v
            .conflicts
            .iter()
            .any(|c| c.loser_rule == "pack:claims-builtin"),);
    }

    /// An unknown field matches nothing: REVIEW with unknown authority, never
    /// a silent Translatable.
    #[test]
    fn unknown_field_is_review() {
        let engine = EligibilityEngine::new();
        let v = engine.evaluate(&definj("Widget.someUnknownField"), None);
        assert_eq!(v.decision, Decision::Review);
        assert_eq!(v.authority, Authority::Unknown);
        assert_eq!(v.evidence[0].rule_id, "builtin:review:unmatched");

        // A known human-facing field on the same shape is translatable.
        let v = engine.evaluate(&definj("Widget.label"), None);
        assert_eq!(v.decision, Decision::Translatable);
    }

    /// TKey and Keyed entries are translatable by first-party mechanism.
    #[test]
    fn tkey_and_keyed_are_translatable() {
        let engine = EligibilityEngine::new();
        let mut tkey_entry = plain_entry(EntryKind::TKey, "SampleQuest.LetterTextSample");
        tkey_entry.tkey = Some(TKeyMeta {
            strategy: "slate_ref".into(),
            suffix: ".slateRef".into(),
            def_type: "QuestScriptDef".into(),
            contexts: 1,
        });
        let v = engine.evaluate(&tkey_entry, None);
        assert_eq!(v.decision, Decision::Translatable);
        assert_eq!(v.evidence[0].rule_id, "builtin:translatable:kind:tkey");

        let v = engine.evaluate(&plain_entry(EntryKind::Keyed, "Greeting"), None);
        assert_eq!(v.decision, Decision::Translatable);
        assert_eq!(v.evidence[0].rule_id, "builtin:translatable:kind:keyed");
    }

    /// Rule packs: valid pack loads; unknown fields, bad schema version and
    /// empty ids are rejected.
    #[test]
    fn rule_pack_loading_and_validation() -> crate::Result<()> {
        let dir = tempfile::tempdir()?;
        let good = dir.path().join("good.pack.json");
        std::fs::write(
            &good,
            r#"{
                "schema_version": 1,
                "rules": [
                    {
                        "id": "community:hugulugs:name",
                        "def_type": "HediffDef",
                        "field_path": "name",
                        "entry_kind": "def_injected",
                        "decision": "translatable",
                        "provenance": "community_rule",
                        "reason": "community knowledge",
                        "versions": ["1.6"]
                    }
                ]
            }"#,
        )?;
        let rules = load_rule_pack(&good)?;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "community:hugulugs:name");
        assert_eq!(rules[0].decision, Decision::Translatable);
        assert_eq!(rules[0].versions, vec!["1.6".to_string()]);

        // Unknown field inside a rule must be rejected.
        let unknown_field = dir.path().join("unknown_field.pack.json");
        std::fs::write(
            &unknown_field,
            r#"{
                "schema_version": 1,
                "rules": [
                    {
                        "id": "x",
                        "field": "label",
                        "decision": "translatable",
                        "provenance": "user_rule",
                        "reason": "r"
                    }
                ]
            }"#,
        )?;
        assert!(load_rule_pack(&unknown_field).is_err());

        // Unknown wrapper field must be rejected.
        let unknown_wrapper = dir.path().join("unknown_wrapper.pack.json");
        std::fs::write(
            &unknown_wrapper,
            r#"{ "schema_version": 1, "rules": [], "extra": true }"#,
        )?;
        assert!(load_rule_pack(&unknown_wrapper).is_err());

        // Wrong schema version is rejected.
        let bad_version = dir.path().join("bad_version.pack.json");
        std::fs::write(&bad_version, r#"{ "schema_version": 99, "rules": [] }"#)?;
        assert!(load_rule_pack(&bad_version).is_err());

        // Empty id is rejected.
        let empty_id = dir.path().join("empty_id.pack.json");
        std::fs::write(
            &empty_id,
            r#"{
                "schema_version": 1,
                "rules": [
                    { "id": "  ", "decision": "review", "provenance": "user_rule", "reason": "r" }
                ]
            }"#,
        )?;
        assert!(load_rule_pack(&empty_id).is_err());
        Ok(())
    }

    /// The pack-rule mirror must cover every domain Rule field: serializing a
    /// domain Rule with all fields set must survive a pack reload.
    #[test]
    fn pack_rule_mirror_covers_domain_rule_fields() {
        let full = Rule {
            id: "project:x".into(),
            def_type: Some("ThingDef".into()),
            field_path: Some("label".into()),
            package_id: Some("somemod.main".into()),
            entry_kind: Some("def_injected".into()),
            decision: Decision::NonTranslatable,
            provenance: EvidenceSource::ProjectOverride,
            reason: "keep identifiers".into(),
            versions: vec!["1.5".into(), "1.6".into()],
        };
        let json = serde_json::to_value(&full).unwrap();
        let back: PackRule = serde_json::from_value(json).expect("mirror drifted from domain Rule");
        assert_eq!(back.id, full.id);
        assert_eq!(back.versions.len(), 2);
    }

    /// Package scoping, user rules as fallback deciders, and explain JSON.
    #[test]
    fn package_scoping_user_fallback_and_explain_json() {
        // A project override can decide when NO builtin rule matched.
        let scoped = EligibilityEngine::new().with_rule_pack(vec![clone_with_package(
            &rule(
                "project:hidden",
                EvidenceSource::ProjectOverride,
                Decision::NonTranslatable,
                "someUnknownField",
            ),
            "somemod.main",
        )]);
        // Different package: the scoped rule cannot match, nothing else does.
        let v = scoped.evaluate(&definj("Widget.someUnknownField"), Some("othermod.main"));
        assert_eq!(v.decision, Decision::Review);
        // Matching package: the project override decides.
        let v = scoped.evaluate(&definj("Widget.someUnknownField"), Some("somemod.main"));
        assert_eq!(v.decision, Decision::NonTranslatable);
        assert_eq!(v.authority, Authority::Verified);

        // Against a builtin decision the ladder wins: BuiltInRule outranks
        // ProjectOverride, so the builtin Translatable stays and the clash is
        // diagnosed instead of silently resolved.
        let mixed = EligibilityEngine::new().with_rule_pack(vec![clone_with_package(
            &rule(
                "project:hide-label",
                EvidenceSource::ProjectOverride,
                Decision::NonTranslatable,
                "label",
            ),
            "somemod.main",
        )]);
        let v = mixed.evaluate(&definj("Widget.label"), Some("othermod.main"));
        assert_eq!(v.decision, Decision::Translatable);
        let v = mixed.evaluate(&definj("Widget.label"), Some("somemod.main"));
        assert_eq!(v.decision, Decision::Translatable);
        assert!(
            v.conflicts
                .iter()
                .any(|c| c.loser_rule == "project:hide-label"),
            "{v:?}"
        );

        let explain = mixed.explain_json(&definj("Widget.label"), Some("somemod.main"));
        assert_eq!(explain["decision"], "translatable");
        assert!(explain["evidence"].as_array().map(|e| !e.is_empty()) == Some(true));
        assert!(explain["conflicts"].is_array());
    }

    fn clone_with_package(r: &Rule, package_id: &str) -> Rule {
        let mut out = r.clone();
        out.package_id = Some(package_id.into());
        out
    }

    /// Seed-pack shape contract: the composition the eligibility doc states.
    /// Any change to the builtin vocabulary must consciously update the doc
    /// (docs/development/TRANSLATION_ELIGIBILITY.md).
    #[test]
    fn seed_pack_shape_matches_documented_composition() {
        let rules = builtin_seed_rules();
        let kind = rules.iter().filter(|r| r.entry_kind.is_some()).count();
        let notranslate = rules
            .iter()
            .filter(|r| r.id.starts_with("builtin:notranslate:"))
            .count();
        let dict_pairs = {
            let dict = rimloc_parsers_xml::load_embedded_defs_dict();
            let mut pairs: Vec<(&String, &String)> = dict
                .0
                .iter()
                .flat_map(|(dt, fields)| fields.iter().map(move |f| (dt, f)))
                .collect();
            pairs.sort();
            pairs.dedup();
            pairs.len()
        };
        let leaves = TRANSLATABLE_LEAVES.len();
        assert_eq!(kind, 2, "two first-party kind rules (TKey, Keyed)");
        assert_eq!(notranslate, 5, "the [NoTranslate]-family field rules");
        assert_eq!(
            rules.len(),
            kind + notranslate + dict_pairs + leaves,
            "seed pack is exactly kind + NoTranslate + def-type dict + leaf rules"
        );
        // Rule ids are unique — rule packs are addressed by id in conflicts
        // and explain output.
        let mut ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate seed rule ids");
        // The documented total (155): 2 kind + 5 NoTranslate + 126 def-type
        // dict pairs + 22 universal leaf patterns.
        assert_eq!(
            rules.len(),
            155,
            "seed pack drifted from the documented 155 rules; update \
             docs/development/TRANSLATION_ELIGIBILITY.md consciously"
        );
    }
}
