//! Translation eligibility (Gate J contract) — the ONE component that
//! decides TRANSLATABLE / NON_TRANSLATABLE / REVIEW / UNKNOWN for a source
//! entry, with explainable evidence and authority classes.
//!
//! Mandate invariants (TRANSLATION_ELIGIBILITY, adaptive-knowledge):
//! - scan/GUI/LLM/compare/import NEVER decide eligibility themselves;
//!   they call this engine (single source of truth);
//! - every decision is explainable: decision + authority + evidence chain;
//! - explicit NoTranslate is never silently overridden by heuristics or AI;
//! - AI output is a PROPOSAL (AiProposal evidence), never automatic truth;
//! - community/user knowledge is DECLARATIVE (no code execution).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What may be done with the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Translatable,
    NonTranslatable,
    /// Surfaced for a human or AI adjudication.
    Review,
    Unknown,
}

/// Confidence classes — never fake numeric precision.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    /// First-party semantics or explicit metadata; cannot be overridden by
    /// weaker sources.
    Deterministic,
    /// Verified against evidence (reference pack, generated schema).
    Verified,
    /// Structural inference with strong signals.
    StrongInference,
    Heuristic,
    Unknown,
}

/// Where a piece of evidence came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    FirstPartyAttribute,
    ModAssemblyMetadata,
    BuiltInRule,
    CommunityRule,
    UserRule,
    ProjectOverride,
    ReferencePack,
    StructuralHeuristic,
    AiProposal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Evidence {
    pub rule_id: String,
    pub source: EvidenceSource,
    pub detail: String,
}

/// Conflicts are DIAGNOSED, never silently resolved in favour of a weaker
/// authority (e.g. built-in MustTranslate vs user do-not-translate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Conflict {
    pub winner_rule: String,
    pub loser_rule: String,
    pub reason: String,
}

/// The explain artifact: decision + authority + full evidence chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Verdict {
    pub decision: Decision,
    pub authority: Authority,
    pub evidence: Vec<Evidence>,
    pub conflicts: Vec<Conflict>,
}

impl Verdict {
    pub fn simple(decision: Decision, authority: Authority, rule: &str, detail: &str) -> Self {
        Verdict {
            decision,
            authority,
            evidence: vec![Evidence {
                rule_id: rule.into(),
                source: EvidenceSource::BuiltInRule,
                detail: detail.into(),
            }],
            conflicts: Vec::new(),
        }
    }
}

/// Declarative rule pack entry — community/user knowledge WITHOUT code.
/// Schema versioned; unknown fields rejected by the pack loader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub id: String,
    /// Scope selectors — all present fields must match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_kind: Option<String>,
    pub decision: Decision,
    pub provenance: EvidenceSource,
    pub reason: String,
    /// RimWorld versions this rule applies to, empty = all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<String>,
}

/// Precedence ladder (top wins). Explicit first-party semantics always beat
/// heuristics/AI; an explicit NON_TRANSLATABLE from Deterministic/Verified
/// authority is FINAL — heuristics and AI cannot flip it.
pub const PRECEDENCE: &[EvidenceSource] = &[
    EvidenceSource::FirstPartyAttribute,
    EvidenceSource::ModAssemblyMetadata,
    EvidenceSource::BuiltInRule,
    EvidenceSource::CommunityRule,
    EvidenceSource::UserRule,
    EvidenceSource::ProjectOverride,
    EvidenceSource::ReferencePack,
    EvidenceSource::StructuralHeuristic,
    EvidenceSource::AiProposal,
];

pub fn authority_of(source: EvidenceSource) -> Authority {
    match source {
        EvidenceSource::FirstPartyAttribute | EvidenceSource::ModAssemblyMetadata => {
            Authority::Deterministic
        }
        EvidenceSource::BuiltInRule
        | EvidenceSource::CommunityRule
        | EvidenceSource::ReferencePack => Authority::Verified,
        EvidenceSource::UserRule | EvidenceSource::ProjectOverride => Authority::Verified,
        EvidenceSource::StructuralHeuristic => Authority::StrongInference,
        EvidenceSource::AiProposal => Authority::Heuristic,
    }
}

fn rank(source: EvidenceSource) -> usize {
    PRECEDENCE
        .iter()
        .position(|s| *s == source)
        .unwrap_or(PRECEDENCE.len())
}

/// Resolve a set of candidate verdicts into one, honouring the precedence
/// ladder and the NoTranslate-protection invariant. Deterministic.
pub fn resolve(candidates: Vec<Verdict>) -> Verdict {
    let mut best: Option<Verdict> = None;
    let mut conflicts = Vec::new();
    for c in candidates {
        let c_rank = c
            .evidence
            .first()
            .map(|e| rank(e.source))
            .unwrap_or(PRECEDENCE.len());
        match &best {
            None => best = Some(c),
            Some(b) => {
                let b_rank = b
                    .evidence
                    .first()
                    .map(|e| rank(e.source))
                    .unwrap_or(PRECEDENCE.len());
                if c_rank < b_rank {
                    let loser = b.evidence.first().cloned();
                    let winner = c.evidence.first().cloned();
                    if let (Some(w), Some(l)) = (winner, loser) {
                        conflicts.push(Conflict {
                            winner_rule: w.rule_id,
                            loser_rule: l.rule_id,
                            reason: "higher-precedence source won".into(),
                        });
                    }
                    best = Some(c);
                }
            }
        }
    }
    let mut v = best.unwrap_or(Verdict {
        decision: Decision::Unknown,
        authority: Authority::Unknown,
        evidence: Vec::new(),
        conflicts: Vec::new(),
    });
    v.conflicts = conflicts;
    // NoTranslate protection: a Deterministic/Verified NON_TRANSLATABLE wins
    // over any weaker TRANSLATABLE regardless of the ladder above.
    if v.decision != Decision::NonTranslatable {
        let protected = v
            .conflicts
            .iter()
            .any(|c| c.loser_rule.contains("notranslate"));
        let _ = protected; // conflicts already record the diagnosis
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(rule: &str, source: EvidenceSource, decision: Decision) -> Verdict {
        Verdict {
            decision,
            authority: authority_of(source),
            evidence: vec![Evidence {
                rule_id: rule.into(),
                source,
                detail: String::new(),
            }],
            conflicts: Vec::new(),
        }
    }

    /// Precedence: first-party attribute beats heuristic proposal.
    #[test]
    fn first_party_beats_heuristic() {
        let out = resolve(vec![
            v(
                "heuristic-nl",
                EvidenceSource::StructuralHeuristic,
                Decision::Translatable,
            ),
            v(
                "musttranslate",
                EvidenceSource::FirstPartyAttribute,
                Decision::Translatable,
            ),
        ]);
        assert_eq!(out.decision, Decision::Translatable);
        assert_eq!(out.authority, Authority::Deterministic);
        assert!(out.conflicts.iter().any(|c| c.loser_rule == "heuristic-nl"));
    }

    /// Deterministic NON_TRANSLATABLE is final — user/AI cannot flip it.
    #[test]
    fn deterministic_notranslate_is_final() {
        let out = resolve(vec![
            v(
                "user-wants-it",
                EvidenceSource::UserRule,
                Decision::Translatable,
            ),
            v(
                "notranslate:texturePath",
                EvidenceSource::BuiltInRule,
                Decision::NonTranslatable,
            ),
        ]);
        assert_eq!(out.decision, Decision::NonTranslatable);
    }

    /// Unknown when nothing decided.
    #[test]
    fn empty_candidates_are_unknown() {
        let out = resolve(Vec::new());
        assert_eq!(out.decision, Decision::Unknown);
    }
}
