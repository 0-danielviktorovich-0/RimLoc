//! Extraction of player-visible text values from RimWorld PatchOperations.
//!
//! WHY: `scan --with-patches` and `learn-patches` reported zero text for mods
//! that patch defs via Patches/*.xml (competitive gap, MUST_FIX_BEFORE_BETA
//! #1). Three root causes, all fixed here / in the integration layer:
//!
//! 1. `roxmltree::Node::text()` on a `<value>` element returns only the text
//!    BEFORE the first child element (usually whitespace) — element values
//!    like `<value><title>Foo</title></value>` yielded nothing.
//! 2. Real-world patches nest operations one or more levels deep:
//!    `PatchOperationSequence/operations/li[Class=PatchOperation*]`,
//!    `PatchOperationFindMod/match[Class=…]`,
//!    `PatchOperationIfModActive/operations/li[Class=…]`. Matching only the
//!    `Operation` tag name missed all of them.
//! 3. (integration layer) xpath inference accepted only `…/Defs/…` with a
//!    leading slash and single-quoted conditions, while the corpus writes
//!    `Defs/ThingDef[defName="X"]`.
//!
//! WHAT is extracted: leaf text values inside `<value>` of
//! PatchOperationAdd / PatchOperationReplace / PatchOperationInsert whose
//! resolved field name is a player-visible string field. The whitelist is
//! derived from the same embedded defs dictionary (`defs_fields.json`) used
//! for ordinary Defs, plus the explicitly mandated fields
//! (title, titleShort, description, baseDesc, label, descriptionShort, …).
//! A noise blacklist (translate = corruption) is applied first:
//! bodyType*, *Tags/*Categories vectors, defName/Name, non-string literals
//! (numbers, booleans).

use roxmltree::Document;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

/// Explicit noise fields: patching them is data-shaping, translating them is
/// corruption. `bodyType*` and `*Tags`/`*Categories` families are covered by
/// prefix/suffix rules in [`is_patch_noise_field`], this list names the rest.
pub const PATCH_NOISE_FIELDS: &[&str] = &[
    "spawnCategories",
    "requiredWorkTags",
    "defName",
    "Name",
    "bodyTypeGlobal",
    "defaultOutfitTags",
    "forcedTraits",
    "requiredSkills",
    "skillGains",
    "allowedArchotypes",
];

/// True when the field must NEVER surface as a translation candidate.
/// Matches case-insensitively like the rest of the XML pipeline.
pub fn is_patch_noise_field(field: &str) -> bool {
    if PATCH_NOISE_FIELDS
        .iter()
        .any(|n| n.eq_ignore_ascii_case(field))
    {
        return true;
    }
    // bodyType, bodyTypeMale, bodyTypeFemale, bodyTypeGlobal, …
    if field
        .get(..8)
        .map(|p| p.eq_ignore_ascii_case("bodyType"))
        .unwrap_or(false)
    {
        return true;
    }
    // Vectors of identifiers/tags: workTags, allowedTags, weaponTags,
    // apparelTags, spawnCategories, techHediffsCategories, …
    let lower = field.to_ascii_lowercase();
    lower.ends_with("tags") || lower.ends_with("categories")
}

/// Player-visible leaf fields: the last segment of every field path in the
/// embedded defs dictionary (the SAME dictionary ordinary Defs scanning uses),
/// plus the explicitly mandated patch-relevant names.
fn player_visible_fields() -> &'static BTreeSet<String> {
    static SET: OnceLock<BTreeSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for f in [
            "title",
            "titleShort",
            "titleFemale",
            "titleShortFemale",
            "description",
            "baseDesc",
            "label",
            "descriptionShort",
        ] {
            set.insert(f.to_string());
        }
        let dict = super::load_embedded_defs_dict();
        for fields in dict.0.values() {
            for f in fields {
                if let Some(leaf) = leaf_segment(f) {
                    set.insert(leaf.to_string());
                }
            }
        }
        set
    })
}

/// Last meaningful segment of a dotted dict field path
/// (`comps.li{h}.label` -> `label`, `rulesStrings` -> `rulesStrings`).
fn leaf_segment(field_path: &str) -> Option<&str> {
    field_path
        .split('.')
        .filter_map(|seg| {
            let s = seg.trim().trim_end_matches("{h}");
            if s.is_empty() || s == "li" {
                None
            } else {
                Some(s)
            }
        })
        .next_back()
}

/// Resolve the whitelist/noise field name for a dotted `<value>` path:
/// `li` items resolve to their container (`rulesStrings.li` -> `rulesStrings`).
fn resolve_field(field_path: &str) -> &str {
    leaf_segment(field_path).unwrap_or(field_path)
}

/// Non-string literal guard: numbers and booleans inside `<value>` are data,
/// not translatable text ("1.5", "42", "true").
fn looks_non_string(trimmed: &str) -> bool {
    trimmed.parse::<f64>().is_ok()
        || trimmed.eq_ignore_ascii_case("true")
        || trimmed.eq_ignore_ascii_case("false")
}

/// Patch operations whose `<value>` carries insertable/replaceable def data.
fn is_value_operation_class(class: &str) -> bool {
    class.eq_ignore_ascii_case("PatchOperationAdd")
        || class.eq_ignore_ascii_case("PatchOperationReplace")
        || class.eq_ignore_ascii_case("PatchOperationInsert")
}

/// A player-visible text value found inside a patch `<value>` node.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchValueCandidate {
    /// `Class` attribute of the operation, e.g. `PatchOperationReplace`.
    pub operation_class: String,
    /// `<xpath>` of the operation (the source locator of the patched def).
    pub xpath: Option<String>,
    /// Dotted path of the value inside `<value>` (e.g. `description`,
    /// `rulesStrings.li`).
    pub field_path: String,
    /// Resolved field name used for whitelist/noise filtering
    /// (`rulesStrings.li` -> `rulesStrings`).
    pub field: String,
    /// Full trimmed text of the value.
    pub value: String,
    pub source_file: PathBuf,
}

/// Counters describing what the extractor saw and what it filtered.
/// `filtered_not_whitelisted` counts structurally valid but non-visible
/// leaves (containers, unknown fields); `filtered_noise` counts
/// blacklist hits; `filtered_non_string` counts numeric/boolean literals.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct PatchScanStats {
    pub operations_seen: u32,
    pub value_operations: u32,
    pub emitted: u32,
    pub filtered_noise: u32,
    pub filtered_non_string: u32,
    pub filtered_not_whitelisted: u32,
    pub filtered_short: u32,
}

/// Depth-first leaf collection inside a `<value>` node.
/// A leaf is an element without element children and with non-empty text.
fn collect_value_leaves(node: roxmltree::Node, prefix: &str, out: &mut Vec<(String, String)>) {
    for child in node.children().filter(|n| n.is_element()) {
        let name = child.tag_name().name();
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}.{name}")
        };
        if child.children().any(|n| n.is_element()) {
            collect_value_leaves(child, &path, out);
        } else if let Some(t) = child.text() {
            let t = t.trim();
            if !t.is_empty() {
                out.push((path, t.to_string()));
            }
        }
    }
}

/// Field name from the tail of an xpath (for plain-text `<value>`:
/// `Defs/ThingDef[defName="X"]/description` -> `description`).
fn field_from_xpath_tail(xpath: &str) -> Option<String> {
    let tail = xpath.split('/').map(str::trim).rfind(|s| !s.is_empty())?;
    let name = tail.split('[').next()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Extract player-visible values from one patch XML document.
fn extract_from_str(
    content: &str,
    source_file: &Path,
    min_len: usize,
) -> (Vec<PatchValueCandidate>, PatchScanStats) {
    let mut stats = PatchScanStats::default();
    let mut out = Vec::new();
    let Ok(doc) = Document::parse(content) else {
        return (out, stats);
    };
    let whitelist = player_visible_fields();
    for op in doc.root_element().descendants().filter(|n| n.is_element()) {
        // Any-depth `Class` match covers Sequence/FindMod/Conditional/
        // IfModActive wrappers: nested operations are `<li Class=…>` or
        // `<match Class=…>` siblings, not `<Operation>` tags.
        let Some(class) = op.attribute("Class") else {
            continue;
        };
        if !is_value_operation_class(class) {
            continue;
        }
        stats.operations_seen += 1;
        let xpath = op
            .children()
            .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("xpath"))
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string());
        let Some(value_node) = op
            .children()
            .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("value"))
        else {
            continue;
        };
        stats.value_operations += 1;

        let mut leaves: Vec<(String, String)> = Vec::new();
        collect_value_leaves(value_node, "", &mut leaves);
        if leaves.is_empty() {
            // Plain-text value (`<value>text</value>`): the field comes from
            // the xpath tail.
            if let Some(t) = value_node.text().map(str::trim).filter(|t| !t.is_empty()) {
                if let Some(field) = xpath.as_deref().and_then(field_from_xpath_tail) {
                    leaves.push((field, t.to_string()));
                }
            }
        }

        for (field_path, text) in leaves {
            let mut field = resolve_field(&field_path).to_string();
            if field_path == "li" {
                // Bare list items inherit their container field from the
                // xpath tail (`…/spawnCategories` + <li>…</li> -> noise).
                if let Some(tail) = xpath.as_deref().and_then(field_from_xpath_tail) {
                    field = tail;
                }
            }
            if is_patch_noise_field(&field) {
                stats.filtered_noise += 1;
                continue;
            }
            if !whitelist.contains(&field) {
                stats.filtered_not_whitelisted += 1;
                continue;
            }
            if looks_non_string(&text) {
                stats.filtered_non_string += 1;
                continue;
            }
            if text.chars().count() < min_len {
                stats.filtered_short += 1;
                continue;
            }
            stats.emitted += 1;
            out.push(PatchValueCandidate {
                operation_class: class.to_string(),
                xpath: xpath.clone(),
                field_path,
                field,
                value: text,
                source_file: source_file.to_path_buf(),
            });
        }
    }
    (out, stats)
}

/// Scan `<root>/Patches/**/*.xml` (any depth, like the rest of the pipeline)
/// and return every player-visible patch value plus aggregate counters.
pub fn scan_patch_values(
    root: &Path,
    min_len: usize,
) -> rimloc_core::Result<(Vec<PatchValueCandidate>, PatchScanStats)> {
    let mut out = Vec::new();
    let mut total = PatchScanStats::default();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let s = p.to_string_lossy();
        if !(s.contains("/Patches/") || s.contains("\\Patches\\")) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(p) else {
            continue;
        };
        let (cands, stats) = extract_from_str(&content, p, min_len);
        total.operations_seen += stats.operations_seen;
        total.value_operations += stats.value_operations;
        total.emitted += stats.emitted;
        total.filtered_noise += stats.filtered_noise;
        total.filtered_non_string += stats.filtered_non_string;
        total.filtered_not_whitelisted += stats.filtered_not_whitelisted;
        total.filtered_short += stats.filtered_short;
        out.extend(cands);
    }
    // Deterministic order across platforms.
    out.sort_by(|a, b| {
        a.source_file
            .cmp(&b.source_file)
            .then(a.field_path.cmp(&b.field_path))
            .then(a.value.cmp(&b.value))
            .then(a.operation_class.cmp(&b.operation_class))
    });
    Ok((out, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
<Patch>
  <Operation Class="PatchOperationSequence">
    <success>Always</success>
    <operations>
      <li Class="PatchOperationAdd">
        <xpath>Defs/BackstoryDef[defName="X"]/spawnCategories</xpath>
        <value><li>Noise</li></value>
      </li>
      <li Class="PatchOperationReplace">
        <xpath>Defs/BackstoryDef[defName="X"]/description</xpath>
        <value><description>Visible text.</description></value>
      </li>
    </operations>
  </Operation>
  <Operation Class="PatchOperationReplace">
    <xpath>Defs/ThingDef[defName="Y"]/label</xpath>
    <value><label>42</label></value>
  </Operation>
</Patch>"#;

    #[test]
    fn noise_field_rules() {
        for f in [
            "spawnCategories",
            "requiredWorkTags",
            "bodyType",
            "bodyTypeMale",
            "bodyTypeGlobal",
            "allowedTags",
            "defName",
        ] {
            assert!(is_patch_noise_field(f), "{f} must be noise");
        }
        for f in ["title", "description", "baseDesc", "label"] {
            assert!(!is_patch_noise_field(f), "{f} must not be noise");
        }
    }

    #[test]
    fn extracts_visible_and_filters_noise() {
        let (cands, stats) = extract_from_str(SAMPLE, Path::new("Patches/x.xml"), 1);
        assert_eq!(cands.len(), 1, "only description must survive: {cands:?}");
        assert_eq!(cands[0].field, "description");
        assert_eq!(cands[0].value, "Visible text.");
        assert_eq!(cands[0].operation_class, "PatchOperationReplace");
        assert!(
            stats.filtered_noise >= 1,
            "spawnCategories.li must count as noise"
        );
        assert!(
            stats.filtered_non_string >= 1,
            "42 must count as non-string"
        );
        assert_eq!(stats.emitted, 1);
    }

    #[test]
    fn plain_text_value_uses_xpath_tail() {
        let xml = r#"<Patch><Operation Class="PatchOperationReplace">
            <xpath>Defs/ThingDef[defName="Z"]/title</xpath>
            <value>mad ruler</value>
        </Operation></Patch>"#;
        let (cands, stats) = extract_from_str(xml, Path::new("Patches/y.xml"), 1);
        assert_eq!(stats.emitted, 1);
        assert_eq!(cands[0].field_path, "title");
        assert_eq!(cands[0].value, "mad ruler");
    }

    #[test]
    fn nested_wrappers_are_reached() {
        let xml = r#"<Patch>
          <Operation Class="PatchOperationIfModActive">
            <modName>some.mod</modName>
            <operations>
              <li Class="PatchOperationAdd">
                <xpath>Defs/ThingDef[defName="A"]</xpath>
                <value><description>Nested desc.</description></value>
              </li>
            </operations>
          </Operation>
          <Operation Class="PatchOperationFindMod">
            <mods><li>Mod</li></mods>
            <match Class="PatchOperationReplace">
              <xpath>Defs/ThingDef[defName="B"]/label</xpath>
              <value><label>Nested label</label></value>
            </match>
          </Operation>
        </Patch>"#;
        let (cands, stats) = extract_from_str(xml, Path::new("Patches/z.xml"), 1);
        assert_eq!(
            stats.emitted, 2,
            "IfModActive + FindMod/match must be reached"
        );
        let fields: Vec<&str> = cands.iter().map(|c| c.field.as_str()).collect();
        assert!(fields.contains(&"description") && fields.contains(&"label"));
    }
}
