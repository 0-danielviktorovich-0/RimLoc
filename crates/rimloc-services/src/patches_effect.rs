//! Patch-applied effective content — bounded subset (pre-Gate-I).
//!
//! Game authority (GAME_SOURCE_FINDINGS §1.4 step 8): `ApplyPatches` runs on
//! the unified Defs XML BEFORE inheritance/def creation, so POST-patch content
//! is what the translator should see. RimLoc's raw scan is PRE-patch; this
//! stage applies the highest-value literal-xpath subset and explicitly
//! classifies everything else so a pre-patch inventory is never silently
//! presented as exact runtime truth.
//!
//! Supported subset: `replace` / `add` / `remove` (incl. inside
//! `<operations>` lists) whose xpath is an absolute simple path
//! `/Defs/Tag[defName="X"]/field/...` — one `defName` predicate on the first
//! segment, plain tag segments after it. Everything else (attribute
//! predicates, `ancestor::`/`@`, functions, add-to-`/Defs` root, …) is
//! counted as unsupported and keeps the view at PARTIAL coverage.

use crate::Result;
use roxmltree::Document;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize)]
pub struct UnsupportedOp {
    pub file: String,
    pub op: String,
    pub xpath: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchCoverage {
    /// No Patches directory: inventory is raw source.
    None,
    /// Every operation was evaluated by the supported subset.
    Full,
    /// Some operations were unsupported — inventory is only partially
    /// patch-applied (POTENTIAL view, not exact runtime truth).
    Partial,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PatchReport {
    pub files_scanned: usize,
    pub ops_total: usize,
    pub applied_replace: usize,
    pub applied_add: usize,
    pub applied_remove: usize,
    pub no_target: usize,
    pub unsupported_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unsupported: Vec<UnsupportedOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<PatchCoverage>,
}

const MAX_UNSUPPORTED_SAMPLE: usize = 20;

enum Op {
    Replace,
    Add,
    Remove,
}

struct ParsedXpath {
    def_name: String,
    field_path: String,
}

/// `/Defs/ThingDef[defName="X"]/label` → identity X, field "label".
/// Returns None for anything outside the supported subset.
fn parse_xpath(xpath: &str) -> Option<ParsedXpath> {
    let rest = xpath.trim().strip_prefix("/Defs/")?;
    if rest.contains("::") || rest.contains('@') || rest.contains('(') {
        return None;
    }
    let mut def_name: Option<String> = None;
    let mut fields: Vec<String> = Vec::new();
    for seg in rest.split('/') {
        let seg = seg.trim();
        if seg.is_empty() {
            return None;
        }
        let (tag, predicate) = match (seg.find('['), seg.find(']')) {
            (Some(a), Some(b)) if b > a => (&seg[..a], Some(&seg[a + 1..b])),
            (None, None) => (seg, None),
            _ => return None,
        };
        if tag.is_empty() || !tag.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        match predicate {
            None => fields.push(tag.to_string()),
            Some(p) => {
                // Only the identity predicate on the first segment.
                if def_name.is_some() || !fields.is_empty() {
                    return None;
                }
                let value = p
                    .trim()
                    .strip_prefix("defName=")
                    .map(str::trim)
                    .and_then(|v| v.strip_prefix('"'))
                    .map(|s| s.strip_suffix('"').unwrap_or(s));
                def_name = Some(value?.to_string());
                if def_name.as_deref().unwrap_or("").is_empty() {
                    return None;
                }
            }
        }
    }
    let def_name = def_name?;
    Some(ParsedXpath {
        def_name,
        field_path: fields.join("."),
    })
}

fn op_kind(tag: &str) -> Option<Op> {
    match tag.to_ascii_lowercase().as_str() {
        "replace" => Some(Op::Replace),
        "add" => Some(Op::Add),
        "remove" => Some(Op::Remove),
        _ => None,
    }
}

/// Apply the supported subset to a Defs-namespace inventory
/// (units keyed `{defName}.{field}`). Returns the transformed units and the
/// report; `units` must be the PRE-patch inventory.
pub fn apply_patch_stage(
    mut units: Vec<rimloc_core::TransUnit>,
    patches_dir: &Path,
) -> (Vec<rimloc_core::TransUnit>, PatchReport) {
    let mut report = PatchReport::default();
    if !patches_dir.is_dir() {
        report.coverage = Some(PatchCoverage::None);
        return (units, report);
    }

    let mut files: Vec<std::path::PathBuf> = WalkDir::new(patches_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
        })
        .collect();
    files.sort();

    for file in &files {
        report.files_scanned += 1;
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        let Ok(doc) = Document::parse(&content) else {
            report.unsupported_count += 1;
            if report.unsupported.len() < MAX_UNSUPPORTED_SAMPLE {
                report.unsupported.push(UnsupportedOp {
                    file: file.display().to_string(),
                    op: "<document>".into(),
                    xpath: None,
                    reason: "unparseable patch XML".into(),
                });
            }
            continue;
        };
        for op_node in doc
            .root_element()
            .children()
            .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("Operation"))
        {
            apply_operation_node(op_node, &mut units, &mut report, file);
        }
    }

    report.coverage = Some(if report.unsupported_count > 0 || report.no_target > 0 {
        PatchCoverage::Partial
    } else {
        PatchCoverage::Full
    });
    (units, report)
}

fn apply_operation_node(
    op_node: roxmltree::Node,
    units: &mut Vec<rimloc_core::TransUnit>,
    report: &mut PatchReport,
    file: &Path,
) {
    // Canonical form: <Operation Class="PatchOperationX"> carries xpath/value
    // on ITSELF; dispatch it directly instead of walking its children.
    let own_class = op_node.attribute("Class").unwrap_or("");
    if let Some(kind) = op_kind(own_class.trim_start_matches("PatchOperation")) {
        execute_op(kind, op_node, units, report, file);
        return;
    }
    for child in op_node.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        if tag.eq_ignore_ascii_case("success") {
            continue;
        }
        if tag.eq_ignore_ascii_case("operations") {
            // Nested op lists compose; recurse.
            apply_operation_node(child, units, report, file);
            continue;
        }
        let Some(kind) = op_kind(tag) else {
            report.ops_total += 1;
            report.unsupported_count += 1;
            if report.unsupported.len() < MAX_UNSUPPORTED_SAMPLE {
                report.unsupported.push(UnsupportedOp {
                    file: file.display().to_string(),
                    op: tag.to_string(),
                    xpath: None,
                    reason: "unsupported operation class".into(),
                });
            }
            continue;
        };
        execute_op(kind, child, units, report, file);
    }
}

/// Evaluate one concrete operation; `holder` is the node carrying the
/// `<xpath>`/`<value>` children.
fn execute_op(
    kind: Op,
    holder: roxmltree::Node,
    units: &mut Vec<rimloc_core::TransUnit>,
    report: &mut PatchReport,
    file: &Path,
) {
    report.ops_total += 1;
    let xpath = holder
        .children()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("xpath"))
        .and_then(|n| n.text())
        .map(str::trim)
        .map(String::from);
    let Some(xp) = xpath.as_deref().and_then(parse_xpath) else {
        report.unsupported_count += 1;
        if report.unsupported.len() < MAX_UNSUPPORTED_SAMPLE {
            report.unsupported.push(UnsupportedOp {
                file: file.display().to_string(),
                op: "operation".into(),
                xpath: xpath.clone(),
                reason: "xpath outside the supported literal subset".into(),
            });
        }
        return;
    };

    match kind {
        Op::Replace => {
            let key = format!("{}.{}", xp.def_name, xp.field_path);
            let value = holder
                .children()
                .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("value"))
                .and_then(|n| n.text())
                .map(str::trim)
                .map(String::from);
            match value {
                Some(text) => {
                    if let Some(u) = units.iter_mut().find(|u| u.key == key) {
                        u.source = Some(text);
                        report.applied_replace += 1;
                    } else {
                        report.no_target += 1;
                    }
                }
                None => {
                    report.unsupported_count += 1;
                    if report.unsupported.len() < MAX_UNSUPPORTED_SAMPLE {
                        report.unsupported.push(UnsupportedOp {
                            file: file.display().to_string(),
                            op: "replace".into(),
                            xpath: Some(key),
                            reason: "replace without a text <value>".into(),
                        });
                    }
                }
            }
        }
        Op::Remove => {
            let key = format!("{}.{}", xp.def_name, xp.field_path);
            let before = units.len();
            units.retain(|u| u.key != key);
            if units.len() < before {
                report.applied_remove += 1;
            } else {
                report.no_target += 1;
            }
        }
        Op::Add => {
            let target_key = if xp.field_path.is_empty() {
                xp.def_name.clone()
            } else {
                format!("{}.{}", xp.def_name, xp.field_path)
            };
            let value_node = holder
                .children()
                .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("value"));
            match value_node {
                Some(vn) => {
                    let element_children: Vec<_> =
                        vn.children().filter(|n| n.is_element()).collect();
                    if element_children.is_empty() {
                        if let Some(text) = vn.text().map(str::trim).filter(|t| !t.is_empty()) {
                            if let Some(u) = units.iter_mut().find(|u| u.key == target_key) {
                                u.source = Some(text.to_string());
                                report.applied_add += 1;
                            } else {
                                report.no_target += 1;
                            }
                        }
                    } else {
                        for el in element_children {
                            let leaf = collect_leaf_fields(el, el.tag_name().name());
                            for (sub_path, text) in leaf {
                                units.push(rimloc_core::TransUnit {
                                    key: format!("{target_key}.{sub_path}"),
                                    source: Some(text),
                                    path: file.to_path_buf(),
                                    line: None,
                                    tkey: None,
                                });
                                report.applied_add += 1;
                            }
                        }
                    }
                }
                None => {
                    report.unsupported_count += 1;
                    if report.unsupported.len() < MAX_UNSUPPORTED_SAMPLE {
                        report.unsupported.push(UnsupportedOp {
                            file: file.display().to_string(),
                            op: "add".into(),
                            xpath: Some(target_key),
                            reason: "add without a <value>".into(),
                        });
                    }
                }
            }
        }
    }
}

/// Flatten a patch value element into (field sub-path, text) leaves.
fn collect_leaf_fields(node: roxmltree::Node, prefix: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let element_children: Vec<_> = node.children().filter(|n| n.is_element()).collect();
    if element_children.is_empty() {
        if let Some(text) = node.text().map(str::trim).filter(|t| !t.is_empty()) {
            out.push((prefix.to_string(), text.to_string()));
        }
        return out;
    }
    for child in element_children {
        let path = format!("{prefix}.{}", child.tag_name().name());
        out.extend(collect_leaf_fields(child, &path));
    }
    out
}

/// Convenience: stage applied only when the mod carries Patches/.
pub fn maybe_apply_patch_stage(
    units: Vec<rimloc_core::TransUnit>,
    mod_root: &Path,
) -> (Vec<rimloc_core::TransUnit>, PatchReport) {
    let patches = mod_root.join("Patches");
    apply_patch_stage(units, &patches)
}

pub type CoreResult<T> = Result<T>;

impl PatchReport {
    /// Fold a per-directory report into an aggregate (patch dirs are applied
    /// in game-equivalent order: root first, then content dirs sorted).
    pub fn merge_from(&mut self, other: &PatchReport) {
        self.files_scanned += other.files_scanned;
        self.ops_total += other.ops_total;
        self.applied_replace += other.applied_replace;
        self.applied_add += other.applied_add;
        self.applied_remove += other.applied_remove;
        self.no_target += other.no_target;
        self.unsupported_count += other.unsupported_count;
        self.unsupported.extend(other.unsupported.iter().cloned());
    }

    /// Finalize aggregate coverage from the folded counters (services stay
    /// logging-free; structured logging lands with Gate L observability).
    pub fn finalize(&mut self) {
        self.coverage = Some(if self.ops_total == 0 {
            PatchCoverage::None
        } else if self.unsupported_count > 0 || self.no_target > 0 {
            PatchCoverage::Partial
        } else {
            PatchCoverage::Full
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn fixture(patch_xml: &str) -> (tempfile::TempDir, Vec<rimloc_core::TransUnit>) {
        let dir = tempdir().unwrap();
        let defs = dir.path().join("Defs");
        fs::create_dir_all(&defs).unwrap();
        fs::write(
            defs.join("Thing.xml"),
            r#"<Defs>
  <ThingDef>
    <defName>Widget</defName>
    <label>Old label</label>
    <description>Old description</description>
  </ThingDef>
</Defs>"#,
        )
        .unwrap();
        let patches = dir.path().join("Patches");
        fs::create_dir_all(&patches).unwrap();
        fs::write(patches.join("P.xml"), patch_xml).unwrap();
        // Inventory as if scanned pre-patch (label/description present).
        let units = vec![
            u("Widget.label", "Old label"),
            u("Widget.description", "Old description"),
        ];
        (dir, units)
    }

    fn u(key: &str, text: &str) -> rimloc_core::TransUnit {
        rimloc_core::TransUnit {
            key: key.into(),
            source: Some(text.into()),
            path: "Defs/Thing.xml".into(),
            line: None,
            tkey: None,
        }
    }

    /// Patch changes a translatable label -> the inventory must show the
    /// POST-patch text a player actually sees.
    #[test]
    fn patch_replaces_translatable_label() {
        let (dir, units) = fixture(
            r#"<Patch><Operation Class="PatchOperationReplace">
              <success>Always</success>
              <xpath>/Defs/ThingDef[defName="Widget"]/label</xpath>
              <value>New label</value>
            </Operation></Patch>"#,
        );
        let (units, rep) = apply_patch_stage(units, &dir.path().join("Patches"));
        let l = units.iter().find(|u| u.key == "Widget.label").unwrap();
        assert_eq!(l.source.as_deref(), Some("New label"));
        assert_eq!(rep.applied_replace, 1);
        assert_eq!(rep.coverage, Some(PatchCoverage::Full));
    }

    /// Patch adds a translatable field -> it becomes an inventory entry
    /// even though no scanner dictionary knows it.
    #[test]
    fn patch_adds_translatable_field() {
        let (dir, units) = fixture(
            r#"<Patch><Operation Class="PatchOperationAdd">
              <xpath>/Defs/ThingDef[defName="Widget"]</xpath>
              <value><flavorText>Added flavor text</flavorText></value>
            </Operation></Patch>"#,
        );
        let (units, rep) = apply_patch_stage(units, &dir.path().join("Patches"));
        assert_eq!(rep.applied_add, 1);
        let f = units.iter().find(|u| u.key == "Widget.flavorText").unwrap();
        assert_eq!(f.source.as_deref(), Some("Added flavor text"));
    }

    /// Patch removes/replaces a field -> the unit leaves the inventory.
    #[test]
    fn patch_removes_field() {
        let (dir, units) = fixture(
            r#"<Patch><Operation Class="PatchOperationRemove">
              <xpath>/Defs/ThingDef[defName="Widget"]/description</xpath>
            </Operation></Patch>"#,
        );
        let (units, rep) = apply_patch_stage(units, &dir.path().join("Patches"));
        assert!(!units.iter().any(|u| u.key == "Widget.description"));
        assert_eq!(rep.applied_remove, 1);
        assert_eq!(rep.coverage, Some(PatchCoverage::Full));
    }

    /// Unsupported xpath shapes are classified, never silently pre-patch.
    #[test]
    fn unsupported_xpath_keeps_view_partial() {
        let (dir, units) = fixture(
            r#"<Patch><Operation Class="PatchOperationReplace">
              <xpath>*/ThingDef[defName="Widget"]/label</xpath>
              <value>Nope</value>
            </Operation></Patch>"#,
        );
        let (units, rep) = apply_patch_stage(units, &dir.path().join("Patches"));
        let l = units.iter().find(|u| u.key == "Widget.label").unwrap();
        assert_eq!(l.source.as_deref(), Some("Old label"));
        assert_eq!(rep.unsupported_count, 1);
        assert_eq!(rep.coverage, Some(PatchCoverage::Partial));
    }
}
