use crate::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchTextCandidate {
    pub operation_class: String,
    pub xpath: Option<String>,
    pub tag_path: String,
    pub value: String,
    pub source_file: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred: Option<InferredDefInjected>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InferredDefInjected {
    pub def_type: String,
    pub def_name: String,
    pub field_path: String,
}

/// Scan PatchOperations in Patches/ and collect player-visible text values
/// introduced via <value> nodes. Extraction (leaf walking, whitelist/noise
/// filtering, wrapper-aware operation discovery) lives in
/// `rimloc_parsers_xml::patches_extract`; this layer only adds DefInjected
/// key inference from the operation xpath.
pub fn scan_patches_texts(root: &Path, min_len: usize) -> Result<Vec<PatchTextCandidate>> {
    let (cands, _stats) = rimloc_parsers_xml::scan_patch_values(root, min_len)?;
    let strict = std::env::var("RIMLOC_PATCH_STRICT_XPATH")
        .map(|v| v == "1")
        .unwrap_or(false);
    Ok(cands
        .into_iter()
        .map(|c| {
            let inferred = c
                .xpath
                .as_deref()
                .and_then(|xp| infer_definj_from_xpath_mode(xp, &c.field_path, strict));
            PatchTextCandidate {
                operation_class: c.operation_class,
                xpath: c.xpath,
                tag_path: c.field_path,
                value: c.value,
                source_file: c.source_file,
                inferred,
            }
        })
        .collect())
}

pub(crate) fn infer_definj_from_xpath_mode(
    xpath: &str,
    tag_path: &str,
    strict: bool,
) -> Option<InferredDefInjected> {
    // Heuristic: looking for .../Defs/<DefType>[defName='X' or @defName='X' or @Name='X']/rest/of/path
    // Then map rest/of/path + tag_path into dot path; normalize li's.
    // Corpus xpaths are usually RELATIVE (`Defs/ThingDef[…]`) and quote
    // conditions with double quotes — both accepted here.
    let xp = xpath.replace('\\', "/");
    let after_defs = match xp.find("/Defs/") {
        Some(idx) => &xp[idx + "/Defs/".len()..],
        // relative form `Defs/ThingDef[…]`
        None => xp.strip_prefix("Defs/")?,
    };
    // take the first segment (DefType[...] or DefType)
    let seg_end = after_defs.find('/').unwrap_or(after_defs.len());
    let first = &after_defs[..seg_end];
    // capture def_type and condition
    // patterns like ThingDef[defName='Foo'] or ThingDef[@Name='Bar']
    let re = regex::Regex::new(r"^(?P<ty>[^\[]+)(?P<cond>\[[^\]]+\])?").ok()?;
    let caps = re.captures(first)?;
    let def_type = caps.name("ty")?.as_str().to_string();
    let cond = caps
        .name("cond")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    if def_type.trim().is_empty() {
        return None;
    }
    // try extract def_name from condition (single- AND double-quoted xpath
    // strings are both legal in RimWorld patch xpaths)
    let name_re = if strict {
        regex::Regex::new(r#"(?i)^\[@?(?:defName|Name)\s*=\s*["']([^"']+)["']\]$"#).ok()?
    } else {
        regex::Regex::new(r#"(?i)(?:@?defName|@?Name)\s*=\s*["']([^"']+)["']"#).ok()?
    };
    let def_name = name_re
        .captures(&cond)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())?;
    // the remainder path after the first segment
    let rest = if seg_end < after_defs.len() {
        &after_defs[seg_end + 1..]
    } else {
        ""
    };
    // Build field path: rest segments + tag_path segments
    let mut segs: Vec<String> = Vec::new();
    for part in rest.split('/') {
        if part.is_empty() {
            continue;
        }
        // normalize predicates [..] to li (strict: reject indexed paths like [3])
        let (name, pred) = match part.split_once('[') {
            Some((n, p)) => (n, Some(p)),
            None => (part, None),
        };
        if strict {
            if let Some(preds) = pred {
                // allow @Name/@defName only; reject numeric indexes
                if preds
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    return None;
                }
            }
        }
        if name.eq_ignore_ascii_case("li") {
            segs.push("li".to_string());
        } else if name.is_empty() {
            continue;
        } else {
            segs.push(name.to_string());
        }
    }
    for (i, part) in tag_path.split('.').enumerate() {
        if part.is_empty() {
            continue;
        }
        let name = part.split('[').next().unwrap_or("");
        if i == 0
            && segs
                .last()
                .map(|s| s.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        {
            // xpath already names this field (`…/baseDesc` + <baseDesc>…):
            // appending it again would double the segment (`X.baseDesc.baseDesc`).
            continue;
        }
        if name.eq_ignore_ascii_case("li") {
            segs.push("li".to_string());
        } else if name.is_empty() {
            continue;
        } else {
            segs.push(name.to_string());
        }
    }
    if segs.is_empty() {
        return None;
    }
    let field_path = segs.join(".");
    Some(InferredDefInjected {
        def_type,
        def_name,
        field_path,
    })
}

// Backward-compatible helper used by tests
#[allow(dead_code)]
fn infer_definj_from_xpath(xpath: &str, tag_path: &str) -> Option<InferredDefInjected> {
    infer_definj_from_xpath_mode(xpath, tag_path, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infer_from_xpath_basic() {
        let xp = "/Mods/Test/Defs/ThingDef[defName='Apparel_Parka']/description";
        let inf = infer_definj_from_xpath(xp, "").expect("infer");
        assert_eq!(inf.def_type, "ThingDef");
        assert_eq!(inf.def_name, "Apparel_Parka");
        assert_eq!(inf.field_path, "description");
    }
    #[test]
    fn infer_from_xpath_with_name_and_li() {
        let xp = "/Defs/RecipeDef[@Name='Cook_SimpleMeal']/ingredients/li";
        let inf = infer_definj_from_xpath(xp, "label").expect("infer");
        assert_eq!(inf.def_type, "RecipeDef");
        assert_eq!(inf.def_name, "Cook_SimpleMeal");
        assert_eq!(inf.field_path, "ingredients.li.label");
    }

    #[test]
    fn infer_from_relative_xpath_double_quotes() {
        // Corpus style: relative `Defs/…` and multiline double-quoted condition
        let xp = "Defs/BackstoryDef[\n\t\t\t\tdefName=\"MenagerieKeeper1\"\n\t\t\t]/title";
        let inf = infer_definj_from_xpath_mode(xp, "title", false).expect("infer");
        assert_eq!(inf.def_type, "BackstoryDef");
        assert_eq!(inf.def_name, "MenagerieKeeper1");
        assert_eq!(inf.field_path, "title");
    }

    #[test]
    fn infer_atname_double_quotes_strict() {
        let xp = "/Defs/RecipeDef[@Name=\"Cook_SimpleMeal\"]/label";
        let inf = infer_definj_from_xpath_mode(xp, "label", true).expect("infer");
        assert_eq!(inf.def_name, "Cook_SimpleMeal");
        assert_eq!(inf.field_path, "label");
    }

    #[test]
    fn infer_collapses_xpath_tail_duplicate() {
        // xpath tail names the same field as the <value> leaf: no doubling
        let inf = infer_definj_from_xpath_mode(
            "Defs/BackstoryDef[defName=\"X\"]/baseDesc",
            "baseDesc",
            false,
        )
        .expect("infer");
        assert_eq!(inf.field_path, "baseDesc");
        // different tail keeps both segments
        let inf2 =
            infer_definj_from_xpath_mode("Defs/ThingDef[defName=\"Y\"]/verbs", "li.label", false)
                .expect("infer");
        assert_eq!(inf2.field_path, "verbs.li.label");
    }
}
