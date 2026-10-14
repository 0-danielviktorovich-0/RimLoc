//! Full-project source search (Source Discovery, Stage A) — READ-ONLY.
//!
//! The mandate: a user-facing search across the project's relevant SOURCE
//! files (Defs, source-language Languages, Patches; optional advanced scope
//! over any readable text file) that REUSES the scanner pipeline — the
//! effective-view resolver, the canonical identity and the eligibility
//! engine — and classifies every text match against the canonical source
//! inventory. This is deliberately NOT a parallel inventory implementation:
//! the one effective scan (`scan_units_effective_full_for_source`) is the
//! only source of structural identity here.
//!
//! Classification per match (the last user operation on a key decides its
//! state; the last SCANNER decision decides a match's class):
//! - `AlreadyIndexed` — the match falls inside a source region the scanner
//!   indexed (identity = the canonical `SourceEntryId`);
//! - `DeliberatelyExcluded` — the eligibility engine rules the underlying
//!   entry NonTranslatable (deterministic NoTranslate, never a guess);
//! - `UnknownEligibility` — the engine surfaces it for human review
//!   (Decision::Review / Unknown);
//! - `SuspectedExtractionOmission` — eligible by the engine but absent from
//!   the effective inventory (a CANDIDATE defect: raw text match alone is
//!   never labeled a confirmed scanner defect);
//! - `OutsideEffectiveView` — the match lives outside the resolved view
//!   (inactive version directory, unsatisfied conditional branch);
//! - `ParseFailure` — the file does not parse as XML;
//! - `UnsupportedFormat` — advanced-scope match in a file family RimLoc
//!   does not model (honest refusal to pretend it is a translation unit).
//!
//! Bounded by construction: per-file and total size caps, result limit and
//! offset pagination, and a cancellation probe checked between files.

use crate::scan::{
    autodiscover_defs_context, scan_units_effective_full_for_source, DEFAULT_SOURCE_LANG_DIR,
};
use crate::EligibilityEngine;
use rimloc_domain::canonical::SourceEntryId;
use rimloc_domain::eligibility::Decision;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Match classification (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    AlreadyIndexed,
    DeliberatelyExcluded,
    OutsideEffectiveView,
    UnknownEligibility,
    SuspectedExtractionOmission,
    UnsupportedFormat,
    ParseFailure,
}

/// Scope of the search. `Standard` covers the file families the scanner
/// models (Defs, source-language Languages, Patches); `AllText` adds every
/// other readable text file under the mod root as an ADVANCED scope whose
/// non-standard matches are classified `UnsupportedFormat` when the file
/// family is not part of the translatable model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    #[default]
    Standard,
    AllText,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceSearchRequest {
    pub mod_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_lang_dir: Option<String>,
    pub query: String,
    #[serde(default)]
    pub scope: SearchScope,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub max_results: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceSearchMatch {
    /// Project-root-relative file path (forward slashes).
    pub file: String,
    /// 1-based line of the first match on this line.
    pub line: usize,
    /// The trimmed text of the matched line.
    pub line_text: String,
    /// Enclosing XML element path at the match (xml files only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xml_context: Option<String>,
    pub status: MatchStatus,
    /// Canonical identity when the scanner indexed this region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<SourceEntryId>,
    /// Human-readable explanation of the classification (why).
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceSearchReport {
    pub query: String,
    /// Total matches found across the scanned tree (before pagination).
    pub total_matches: usize,
    pub offset: usize,
    /// Whether `total_matches` exceeds offset + returned results.
    pub truncated: bool,
    pub matches: Vec<SourceSearchMatch>,
    pub files_scanned: usize,
    pub files_skipped_binary: usize,
    /// Files that failed XML parsing (file → error), matches inside them
    /// classified `ParseFailure`.
    pub parse_failures: Vec<(String, String)>,
    /// Source fingerprint of the scanned context (provenance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
    /// Effective view summary (version + relative content roots).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_view: Option<EffectiveViewInfo>,
}

/// Small honest summary of the resolved view for the wire.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EffectiveViewInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub content_dirs: Vec<String>,
    pub conditional_dirs: usize,
}

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_MAX_RESULTS: usize = 200;
const HARD_MAX_RESULTS: usize = 1000;
const MIN_QUERY_LEN: usize = 2;

/// Read-only full-project source search. Reuses the ONE effective scan
/// pipeline for identity; text matching itself is line-based, Unicode-aware
/// and case-insensitive (plain `to_lowercase` contains — deterministic and
/// allocation-light).
pub fn search_sources(
    req: &SourceSearchRequest,
    cancelled: &impl Fn() -> bool,
) -> Result<SourceSearchReport, crate::contract::ContractError> {
    use crate::contract::{ContractError, ContractErrorCode};
    let query_lc = req.query.to_lowercase();
    if req.query.trim().chars().count() < MIN_QUERY_LEN {
        return Err(ContractError::new(
            ContractErrorCode::ValidationFailed,
            format!("query too short (min {MIN_QUERY_LEN} chars)"),
        ));
    }
    let max_results = if req.max_results == 0 {
        DEFAULT_MAX_RESULTS
    } else {
        req.max_results.min(HARD_MAX_RESULTS)
    };

    // ONE effective scan — the same pipeline the inventory uses.
    let root = req.mod_root.clone();
    let auto = autodiscover_defs_context(&req.mod_root).map_err(|e| {
        crate::contract::ContractError::new(
            crate::contract::ContractErrorCode::Internal,
            format!("source discovery: {e}"),
        )
    })?;
    let scan = scan_units_effective_full_for_source(
        &req.mod_root,
        req.target_version.as_deref(),
        &auto.dict,
        &auto.extra_fields,
        None,
        req.source_lang_dir.as_deref(),
    )
    .map_err(|e| {
        crate::contract::ContractError::new(
            crate::contract::ContractErrorCode::Internal,
            format!("source discovery scan: {e}"),
        )
    })?;
    let fingerprint =
        crate::scan::source_fingerprint(&req.mod_root, req.target_version.as_deref()).ok();

    // Coverage index: relative file → sorted unit start lines → entry id.
    // Identities come from the canonical bridge (the ONE identity source).
    // Honest patch stage: any unsupported op means content may be stale
    // (PatchStage::Partial), a fully supported patch pass is Applied.
    let patch_stage = if scan.patch.unsupported_count > 0 {
        rimloc_domain::canonical::PatchStage::Partial
    } else if scan.patch.ops_total > 0 {
        rimloc_domain::canonical::PatchStage::Applied
    } else {
        rimloc_domain::canonical::PatchStage::None
    };
    let entries = crate::canonical_bridge::source_entries_with_locale(
        &scan.units,
        patch_stage,
        scan.view.as_ref().and_then(|v| v.version.as_deref()),
        None,
        req.source_lang_dir
            .as_deref()
            .unwrap_or(DEFAULT_SOURCE_LANG_DIR),
    );
    let mut coverage: BTreeMap<String, Vec<(usize, SourceEntryId)>> = BTreeMap::new();
    let mut coverage_alt: BTreeMap<String, String> = BTreeMap::new();
    let mut entry_by_id: BTreeMap<SourceEntryId, rimloc_domain::canonical::SourceEntry> =
        BTreeMap::new();
    for e in &entries {
        entry_by_id.insert(e.id.clone(), e.clone());
        for c in &e.contexts {
            let raw = normalize_rel(&c.file);
            let rel = rel_to(&root, Path::new(&c.file));
            coverage_alt.insert(raw.clone(), rel.clone());
            // Scan contexts can carry canonical VIRTUAL paths (Defs-derived
            // DefInjected output paths) whose file name differs from the
            // real source file. Also index the BARE FILE NAME as an alt key
            // so the real-file lookup still finds the identity.
            let v = coverage.entry(rel).or_default();
            if let Some(line) = c.line {
                v.push((line, e.id.clone()));
            }
        }
    }
    // File-name alias index: scan contexts sometimes carry VIRTUAL paths
    // (Defs-derived DefInjected output paths) whose directory differs from
    // the real source file; the real file still resolves by name. A name
    // shared by two real files keeps both entries (the line partition
    // below disambiguates).
    let mut coverage_by_name: BTreeMap<String, Vec<(usize, SourceEntryId)>> = BTreeMap::new();
    for (rel_key, v) in &coverage {
        if let Some(fname) = Path::new(rel_key).file_name().and_then(|f| f.to_str()) {
            let e = coverage_by_name.entry(fname.to_string()).or_default();
            e.extend(v.iter().cloned());
        }
    }
    for v in coverage.values_mut() {
        v.sort_unstable();
        v.dedup_by(|a, b| a.0 == b.0);
    }
    for v in coverage_by_name.values_mut() {
        v.sort_unstable();
        v.dedup_by(|a, b| a.0 == b.0);
    }

    // Eligibility engine — the SOLE authority for translatability.
    let engine = EligibilityEngine::new();

    // Walk the mod tree.
    let mut files: Vec<PathBuf> = Vec::new();
    collect_text_files(&root, 0, &mut files, &cancelled).map_err(|source| {
        crate::contract::ContractError::new(
            crate::contract::ContractErrorCode::Internal,
            format!("source discovery walk: {source}"),
        )
    })?;
    files.sort();

    let view = scan.view.clone();
    let mut report = SourceSearchReport {
        query: req.query.clone(),
        total_matches: 0,
        offset: req.offset,
        truncated: false,
        matches: Vec::new(),
        files_scanned: 0,
        files_skipped_binary: 0,
        parse_failures: Vec::new(),
        source_fingerprint: fingerprint,
        effective_view: view.as_ref().map(|v| EffectiveViewInfo {
            version: v.version.clone(),
            content_dirs: v
                .content_dirs
                .iter()
                .map(|d| normalize_rel(&d.display().to_string()))
                .collect(),
            conditional_dirs: v.conditional_dirs.len(),
        }),
    };

    for path in &files {
        if cancelled() {
            break;
        }
        let rel = rel_to(&root, path);
        let effective_here = is_effective_file(&view, path, &root);
        if !standard_scope_file(path) && !req.advanced_all_text() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.len() as u64 > MAX_FILE_BYTES {
            continue;
        }
        if bytes.iter().take(8192).any(|b| *b == 0) {
            report.files_skipped_binary += 1;
            continue;
        }
        report.files_scanned += 1;
        let content = String::from_utf8_lossy(&bytes);
        let is_xml = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"));
        let xml_ok = if is_xml { validate_xml(&content) } else { true };
        if is_xml && !xml_ok {
            report
                .parse_failures
                .push((rel.clone(), "malformed XML".to_string()));
        }
        let rel_fname = Path::new(&rel)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|f| f.to_string());
        let covered = coverage
            .get(&rel)
            .or_else(|| coverage_alt.get(&rel).and_then(|k| coverage.get(k)))
            .or_else(|| rel_fname.as_deref().and_then(|f| coverage_by_name.get(f)));
        let lang_dir = format!(
            "Languages/{}",
            req.source_lang_dir.as_deref().unwrap_or("English")
        );

        for (idx, raw_line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let lc = raw_line.to_lowercase();
            let Some(col) = lc.find(&query_lc) else {
                continue;
            };
            report.total_matches += 1;
            let take =
                report.total_matches > req.offset && (report.matches.len() as usize) < max_results;
            if !take {
                continue;
            }
            // Covered by the inventory? (unit start-line partition per file)
            let hit = covered.and_then(|v| {
                v.iter()
                    .rev()
                    .find(|(start, _)| *start <= line_no)
                    .map(|(_, id)| id.clone())
            });
            let line_text = raw_line.trim().to_string();
            let xml_context = if is_xml {
                Some(xml_context_of(&content, line_no))
            } else {
                None
            };
            let (status, identity, detail) = if !xml_ok && is_xml {
                (
                    MatchStatus::ParseFailure,
                    None,
                    "file failed XML parsing — the scanner cannot index it".to_string(),
                )
            } else if let Some(id) = hit {
                classify_indexed(&engine, &entry_by_id, &id, &lang_dir)
            } else if !effective_here {
                (
                    MatchStatus::OutsideEffectiveView,
                    None,
                    "file is outside the resolved effective view (inactive version \
                     directory or unsatisfied conditional branch)"
                        .to_string(),
                )
            } else {
                classify_candidate(&engine, &rel, &content, line_no, is_xml, &lang_dir)
            };
            let _ = col;
            report.matches.push(SourceSearchMatch {
                file: rel.clone(),
                line: line_no,
                line_text,
                xml_context,
                status,
                identity,
                detail,
            });
        }
    }
    report.truncated = report.total_matches > report.offset + report.matches.len();
    Ok(report)
}

impl SourceSearchRequest {
    fn advanced_all_text(&self) -> bool {
        self.scope == SearchScope::AllText
    }
}

/// An indexed match: engine verdict decides indexed-vs-excluded.
fn classify_indexed(
    engine: &EligibilityEngine,
    entry_by_id: &BTreeMap<SourceEntryId, rimloc_domain::canonical::SourceEntry>,
    id: &SourceEntryId,
    _lang_dir: &str,
) -> (MatchStatus, Option<SourceEntryId>, String) {
    let Some(entry) = entry_by_id.get(id) else {
        return (
            MatchStatus::AlreadyIndexed,
            Some(id.clone()),
            "indexed by the scanner".to_string(),
        );
    };
    let verdict = engine.evaluate(entry, None);
    match verdict.decision {
        Decision::NonTranslatable => (
            MatchStatus::DeliberatelyExcluded,
            Some(id.clone()),
            format!(
                "the eligibility engine rules this entry NonTranslatable ({})",
                verdict
                    .evidence
                    .first()
                    .map(|e| e.rule_id.clone())
                    .unwrap_or_else(|| "rule".to_string())
            ),
        ),
        Decision::Translatable => (
            MatchStatus::AlreadyIndexed,
            Some(id.clone()),
            "indexed by the scanner (translatable)".to_string(),
        ),
        _ => (
            MatchStatus::UnknownEligibility,
            Some(id.clone()),
            "indexed by the scanner; eligibility needs human review".to_string(),
        ),
    }
}

/// An uncovered candidate: build a synthetic entry from the raw XML context
/// and let the eligibility engine classify it. NEVER labels a raw text
/// match a confirmed scanner defect on its own — that is the audit's job.
#[allow(clippy::too_many_arguments)]
fn classify_candidate(
    engine: &EligibilityEngine,
    rel: &str,
    content: &str,
    line_no: usize,
    is_xml: bool,
    lang_dir: &str,
) -> (MatchStatus, Option<SourceEntryId>, String) {
    let ctx = CandidateContext::probe(content, line_no);
    let text = ctx.text_at(content, line_no).unwrap_or_default();
    let synthetic = synthetic_entry(ctx, rel, line_no, text, is_xml, lang_dir);
    let verdict = engine.evaluate(&synthetic, None);
    let decision = verdict.decision;
    let reason = verdict
        .evidence
        .first()
        .map(|e| e.rule_id.clone())
        .unwrap_or_else(|| "no matching rule".to_string());
    match decision {
        Decision::NonTranslatable => (
            MatchStatus::DeliberatelyExcluded,
            None,
            format!("deterministic NoTranslate ({reason})"),
        ),
        Decision::Translatable => (
            MatchStatus::SuspectedExtractionOmission,
            Some(synthetic.id),
            format!(
                "translatable by the eligibility engine but absent from the \
                 effective inventory ({reason}) — candidate extraction omission"
            ),
        ),
        _ => (
            MatchStatus::UnknownEligibility,
            Some(synthetic.id),
            format!("unknown field/rule — needs human review ({reason})"),
        ),
    }
}

/// Minimal raw-context probe: element stack, owning def type, defName,
/// innermost field — derived from the file text (NOT from the scanner).
struct CandidateContext {
    def_type: Option<String>,
    def_name: Option<String>,
    field: Option<String>,
}

impl CandidateContext {
    fn probe(content: &str, line_no: usize) -> Self {
        let mut stack: Vec<String> = Vec::new();
        let mut def_type = None;
        let mut def_name = None;
        let mut field = None;
        for (idx, raw) in content.lines().enumerate() {
            if idx + 1 > line_no {
                break;
            }
            let line = raw.trim();
            for (i, ch) in line.char_indices() {
                if ch != '<' {
                    continue;
                }
                let rest = &line[i + 1..];
                if rest.starts_with('!') || rest.starts_with('?') {
                    continue;
                }
                if let Some(close) = rest.strip_prefix('/') {
                    let name: String = close
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if stack.last().map(|t: &String| t == &name).unwrap_or(false) {
                        stack.pop();
                        if def_name.is_none() {
                            def_name = None;
                        }
                    }
                } else {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if name.is_empty() {
                        continue;
                    }
                    if stack.len() == 1 && def_type.is_none() && name != "Defs" {
                        def_type = Some(name.clone());
                    }
                    if name == "defName" {
                        def_name = text_between(line, i, &name);
                    } else if stack.len() >= 2 {
                        field = Some(name.clone());
                    }
                    let self_closing = line[i..].contains("/>");
                    if !self_closing {
                        stack.push(name.clone());
                    }
                }
            }
        }
        Self {
            def_type,
            def_name,
            field,
        }
    }

    fn text_at(&self, content: &str, line_no: usize) -> Option<String> {
        content
            .lines()
            .nth(line_no.saturating_sub(1))
            .map(|l| l.trim().to_string())
    }
}

fn text_between(line: &str, open_idx: usize, tag: &str) -> Option<String> {
    let close = format!("</{tag}>");
    let start = open_idx + tag.len() + 2;
    let rest = line.get(start..)?;
    let end = rest.find(&close)?;
    let v = rest.get(..end)?.trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Synthetic canonical entry for eligibility evaluation. Identity is
/// structural: Defs XML → DefInjected-style `<defName>.<field>`; source
/// Languages Keyed → the element name; unsupported families keep a raw key.
fn synthetic_entry(
    ctx: CandidateContext,
    rel: &str,
    line_no: usize,
    text: String,
    is_xml: bool,
    lang_dir: &str,
) -> rimloc_domain::canonical::SourceEntry {
    use rimloc_domain::canonical::{
        ContextRole, EntryKind, SourceContext, SourceEntry, SourceEntryId, SourceProvenance,
    };
    let (kind, key, def_type) = if !is_xml {
        (EntryKind::Strings, rel.to_string(), None)
    } else if rel.starts_with(&format!("Languages/{lang_dir}")) || rel.contains("/Languages/") {
        (
            EntryKind::Keyed,
            ctx.field.clone().unwrap_or_else(|| rel.to_string()),
            None,
        )
    } else {
        let field = ctx.field.clone().unwrap_or_else(|| "label".to_string());
        let base = ctx.def_name.clone().unwrap_or_else(|| "?".to_string());
        (
            EntryKind::DefInjected,
            format!("{base}.{field}"),
            ctx.def_type.clone(),
        )
    };
    SourceEntry {
        id: SourceEntryId {
            kind,
            key,
            def_type,
        },
        text,
        source_locale: "en".to_string(),
        contexts: vec![SourceContext {
            file: rel.to_string(),
            line: Some(line_no),
            def_type: ctx.def_type.clone(),
            role: ContextRole::Effective,
        }],
        provenance: SourceProvenance::default(),
        tkey: None,
        source_ref: None,
    }
}

fn validate_xml(content: &str) -> bool {
    let mut reader = quick_xml::Reader::from_str(content);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => return true,
            Ok(_) => buf.clear(),
            Err(_) => return false,
        }
    }
}

fn xml_context_of(content: &str, line_no: usize) -> String {
    let mut stack: Vec<String> = Vec::new();
    for (idx, raw) in content.lines().enumerate() {
        if idx + 1 > line_no {
            break;
        }
        let line = raw.trim();
        for (i, ch) in line.char_indices() {
            if ch != '<' {
                continue;
            }
            let rest = &line[i + 1..];
            if rest.starts_with('!') || rest.starts_with('?') {
                continue;
            }
            if let Some(close) = rest.strip_prefix('/') {
                let name: String = close
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if stack.last().map(|t: &String| t == &name).unwrap_or(false) {
                    stack.pop();
                }
            } else {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if name.is_empty() {
                    continue;
                }
                if !line[i..].contains("/>") {
                    stack.push(name);
                }
            }
        }
    }
    stack.join(" > ")
}

fn normalize_rel(p: &str) -> String {
    p.replace('\\', "/")
}

/// Project-relative, forward-slash path (the coverage map's key space).
/// Windows-safe: separators normalized BOTH ways before the prefix strip,
/// and the coverage map additionally indexes the RAW path form so a
/// case/separator mismatch can never silently disconnect a match from its
/// inventory identity (the Windows CI failure class).
fn rel_to(root: &Path, p: &Path) -> String {
    let norm = |x: &Path| x.to_string_lossy().replace('\\', "/");
    // The walk yields canonical paths; canonicalize the root so the prefix
    // strip matches (macos tempdir /var -> /private/var).
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_s = norm(&root_c);
    let p_s = norm(p);
    if let Some(stripped) = p_s.strip_prefix(&format!("{root_s}/")) {
        return stripped.to_string();
    }
    if p_s == root_s {
        return String::new();
    }
    p_s
}

fn is_effective_file(
    view: &Option<crate::modview::EffectiveModView>,
    path: &Path,
    root: &Path,
) -> bool {
    let Some(view) = view else { return true };
    if view.content_dirs.is_empty() {
        return true;
    }
    // A LoadFolders layout resolves specific content dirs; the bare root
    // then only hosts the shared folders (About/Languages/Patches). When
    // deeper dirs exist, the bare root must not mark EVERYTHING effective
    // (a 1.5 leftover must stay outside the 1.6 view).
    let dirs: Vec<&PathBuf> = if view.content_dirs.len() > 1 {
        view.content_dirs
            .iter()
            .filter(|d| d.as_path() != root)
            .collect()
    } else {
        view.content_dirs.iter().collect()
    };
    if dirs.is_empty() {
        return true;
    }
    // The walk yields CANONICAL paths; the view dirs may be non-canonical
    // (macos tempdirs resolve /var -> /private/var). Canonicalize both
    // sides before the prefix test.
    let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let result = dirs.iter().any(|dir| {
        let canon_dir = dir.canonicalize().unwrap_or_else(|_| (*dir).clone());
        canon_path.starts_with(canon_dir)
    });
    result
}

fn standard_scope_file(path: &Path) -> bool {
    let ext_ok = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xml") || e.eq_ignore_ascii_case("txt"));
    if !ext_ok {
        return false;
    }
    // Windows: the raw path carries '\\' separators, so a forward-slash
    // segment test would reject every file — normalize first.
    let s = path.to_string_lossy().replace('\\', "/");
    ["Defs/", "Patches/", "Languages/"]
        .iter()
        .any(|seg| s.contains(seg))
}

/// Confinement: the walk stays INSIDE the project root. Symlinks are
/// rejected outright (a link pointing outside the tree would drag foreign
/// files into the results); every accepted path must canonicalize under
/// the canonical root — the CodeQL path-injection findings on the previous
/// unconfined walker.
fn confined_collect_text_files(
    root: &Path,
    canonical_root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<PathBuf>,
    cancelled: &impl Fn() -> bool,
) -> std::io::Result<()> {
    if depth > 12 || cancelled() {
        return Ok(());
    }
    let _ = root;
    for entry in std::fs::read_dir(dir)? {
        if cancelled() {
            return Ok(());
        }
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "node_modules" || name == "About" {
            continue;
        }
        let p = entry.path();
        let meta = std::fs::symlink_metadata(&p)?;
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            // Never follow links: an outside-the-tree target must not enter
            // the results (acceptance fixture: symlinks out of the project).
            continue;
        }
        // Containment BEFORE any use of the path: canonicalize and require
        // the result to stay under the canonical project root — both for
        // files and for directories we might descend into.
        let canon = p.canonicalize()?;
        if !canon.starts_with(canonical_root) {
            continue;
        }
        if file_type.is_dir() {
            // Descend by the CANONICAL path only (CodeQL path-injection:
            // no unsanitized read_dir output reaches a filesystem call).
            confined_collect_text_files(root, canonical_root, &canon, depth + 1, out, cancelled)?;
        } else if file_type.is_file() {
            out.push(canon);
        }
    }
    Ok(())
}

fn collect_text_files(
    dir: &Path,
    depth: usize,
    out: &mut Vec<PathBuf>,
    cancelled: &impl Fn() -> bool,
) -> std::io::Result<()> {
    let canonical_root = dir.canonicalize()?;
    confined_collect_text_files(dir, &canonical_root, dir, depth, out, cancelled)
}

// ---------------------------------------------------------------------------
// Fixtures + tests (review acceptance list): every classification gets a
// deterministic fixture; symlinks out of the tree and huge trees are
// handled by the walker caps.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn write_mod(root: &Path) {
        // About + Defs (indexed label; NoTranslate candidate; unknown field;
        // same text in two def types) + Languages/English Keyed + TKey +
        // an INACTIVE version dir (1.5) with a Defs file + a broken XML.
        std::fs::create_dir_all(root.join("About")).unwrap();
        std::fs::write(
            root.join("About/About.xml"),
            "<ModMetaData><name>Search Fixture</name><packageId>rimloc.test.search</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("1.6/Defs")).unwrap();
        std::fs::write(
            root.join("1.6/Defs/Things.xml"),
            "<Defs>\n  <ThingDef>\n    <defName>Apple</defName>\n      <label>Golden Apple</label>\n    </ThingDef>\n  <HediffDef>\n    <defName>Glow</defName>\n    <label>Golden Apple</label>\n  </HediffDef>\n  <ThingDef>\n    <defName>Stone</defName>\n    <label mod_note=\"NT\">Untranslatable Stone</label>\n  </ThingDef>\n  <ResearchProjectDef>\n    <defName>Alchemy</defName>\n    <description>Golden studies</description>\n  </ResearchProjectDef>\n</Defs>\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("Languages/English/Keyed")).unwrap();
        std::fs::write(
            root.join("Languages/English/Keyed/Misc.xml"),
            "<LanguageData>\n  <GoldenKey>Golden hammer</GoldenKey>\n</LanguageData>\n",
        )
        .unwrap();
        // Inactive version dir: 1.5 content when 1.6 is requested.
        std::fs::create_dir_all(root.join("1.5/Defs")).unwrap();
        std::fs::write(
            root.join("1.5/Defs/Old.xml"),
            "<Defs><ThingDef><defName>Old</defName><label>Golden Old</label></ThingDef></Defs>",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("1.6/Defs")).unwrap();
        std::fs::write(
            root.join("1.6/Defs/Current.xml"),
            "<Defs><ThingDef><defName>Cur</defName><label>Current thing</label></ThingDef></Defs>",
        )
        .unwrap();
        // Malformed XML.
        std::fs::write(
            root.join("1.6/Defs/Broken.xml"),
            "<Defs><ThingDef><label>Golden broken</Defs>\n",
        )
        .unwrap();
    }

    fn req(root: &Path, query: &str) -> SourceSearchRequest {
        SourceSearchRequest {
            mod_root: root.to_path_buf(),
            target_version: Some("1.6".into()),
            source_lang_dir: None,
            query: query.into(),
            scope: SearchScope::Standard,
            offset: 0,
            max_results: 200,
        }
    }

    fn statuses_of(rep: &SourceSearchReport, file_part: &str) -> Vec<(MatchStatus, String)> {
        rep.matches
            .iter()
            .filter(|m| m.file.contains(file_part))
            .map(|m| (m.status, m.line_text.clone()))
            .collect()
    }

    #[test]
    fn no_cancel_ken() {
        // cancellation probe never fires in these tests.
    }

    #[test]
    fn indexed_label_classified_already_indexed() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let rep = search_sources(&req(dir.path(), "golden apple"), &|| false).unwrap();
        // Exactly two matches: the ThingDef label and the HediffDef label
        // (same text, two structural identities — no other line contains
        // "golden apple").
        assert_eq!(rep.total_matches, 2, "{:?}", rep.matches);
        let things = statuses_of(&rep, "1.6/Defs/Things.xml");
        let indexed = things
            .iter()
            .filter(|(s, t)| *s == MatchStatus::AlreadyIndexed && t.contains("Golden Apple"))
            .count();
        assert!(indexed >= 1, "expected an indexed Golden Apple: {things:?}");
    }

    #[test]
    fn same_text_two_identities_distinct_classifications() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let rep = search_sources(&req(dir.path(), "golden apple"), &|| false).unwrap();
        let apple_matches: Vec<_> = rep
            .matches
            .iter()
            .filter(|m| m.line_text.contains("Golden Apple") && m.file.contains("Things.xml"))
            .collect();
        assert_eq!(apple_matches.len(), 2, "two structural identities");
        let ids: Vec<_> = apple_matches
            .iter()
            .filter_map(|m| m.identity.as_ref())
            .map(|i| i.key.clone())
            .collect();
        assert!(ids.contains(&"Apple.label".to_string()));
        assert!(ids.contains(&"Glow.label".to_string()));
    }

    #[test]
    fn inactive_version_dir_outside_effective_view() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let rep = search_sources(&req(dir.path(), "golden old"), &|| false).unwrap();
        assert!(
            rep.matches
                .iter()
                .all(|m| m.status == MatchStatus::OutsideEffectiveView),
            "{:?}",
            rep.matches
        );
        assert!(rep.total_matches >= 1);
    }

    #[test]
    fn parse_failure_reported_not_indexed() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let rep = search_sources(&req(dir.path(), "golden broken"), &|| false).unwrap();
        assert!(
            rep.parse_failures
                .iter()
                .any(|(f, _)| f.contains("Broken.xml")),
            "broken file must be reported: {:?}",
            rep.parse_failures
        );
        assert!(
            rep.matches
                .iter()
                .all(|m| m.status == MatchStatus::ParseFailure),
            "{:?}",
            rep.matches
        );
    }

    #[test]
    fn pagination_and_case_insensitivity() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let mut r = req(dir.path(), "GOLDEN");
        r.max_results = 2;
        let rep = search_sources(&r, &|| false).unwrap();
        assert_eq!(rep.matches.len(), 2);
        assert!(rep.truncated);
        assert!(rep.total_matches >= 3);
        // offset paging continues after the first page.
        let mut r2 = req(dir.path(), "golden");
        r2.offset = 2;
        let rep2 = search_sources(&r2, &|| false).unwrap();
        assert_eq!(rep2.matches.len(), rep.total_matches - 2);
    }

    #[test]
    fn too_short_query_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let r = req(dir.path(), "g");
        assert!(search_sources(&r, &|| false).is_err());
    }

    #[test]
    fn cancellation_stops_the_walk() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        let rep = search_sources(&req(dir.path(), "golden"), &|| true).unwrap();
        assert_eq!(rep.total_matches, 0);
    }

    #[test]
    fn symlink_pointing_outside_is_never_walked() {
        let dir = tempfile::tempdir().unwrap();
        write_mod(dir.path());
        // Outside file + symlink inside the mod tree pointing to it.
        // Outside the MOD ROOT but inside the (writable) tempdir.
        let outside = dir.path().join("outside-secret.txt");
        std::fs::write(&outside, "Golden secret").unwrap();
        std::fs::create_dir_all(dir.path().join("mod/Defs")).unwrap();
        let link = dir.path().join("mod/Defs/link-to-secret.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(not(unix))]
        let _ = &link;
        let rep = search_sources(&req(dir.path(), "golden secret"), &|| false).unwrap();
        assert_eq!(
            rep.total_matches, 0,
            "a symlink out of the project must never surface content: {:?}",
            rep.matches
        );
    }

    #[test]
    fn unicode_query_matches() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Defs")).unwrap();
        std::fs::write(
            dir.path().join("Defs/U.xml"),
            "<Defs>\n  <ThingDef>\n    <defName>Юнит</defName>\n    <label>Золотой Юнит</label>\n  </ThingDef>\n</Defs>\n",
        )
        .unwrap();
        let rep = search_sources(&req(dir.path(), "золотой"), &|| false).unwrap();
        assert!(rep.total_matches >= 1, "{:?}", rep.matches);
        let m = rep.matches.first().unwrap();
        assert!(m.line_text.contains("Золотой Юнит"));
    }
}
