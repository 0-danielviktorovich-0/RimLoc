pub use rimloc_core::parse_simple_po as parse_po_string;

use quick_xml::events::Event;
use quick_xml::Reader;
use rimloc_core::{Result as CoreResult, TransUnit};
use serde::Deserialize;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;

/// Very small XML scanner for RimWorld "Keyed" XML files.
/// It walks `root` and finds files that match `*/Languages/*/Keyed/*.xml`.
/// For every `<Key>Value</Key>` pair found, it produces a `TransUnit` with
/// the key name, text value, file path and (approximate) line number.
pub fn scan_keyed_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<TransUnit> = Vec::new();
    fn keyed_nested_enabled() -> bool {
        matches!(std::env::var("RIMLOC_KEYED_NESTED"), Ok(v) if v.trim() == "1")
    }

    fn line_for_offset(offset: usize, starts: &[usize]) -> Option<usize> {
        if starts.is_empty() {
            return None;
        }
        match starts.binary_search(&offset) {
            Ok(idx) => Some(idx + 1),
            Err(idx) if idx > 0 => Some(idx),
            _ => Some(1),
        }
    }

    // (no cross-file logic in keyed scanner)

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Languages/<Locale>/{Keyed,DefInjected}/....xml
        let p_str = p.to_string_lossy();
        if !(p_str.contains("/Languages/") || p_str.contains("\\Languages\\")) {
            continue;
        }
        let has_keyed = p_str.contains("/Keyed/") || p_str.contains("\\Keyed\\");
        let has_definj = p_str.contains("/DefInjected/") || p_str.contains("\\DefInjected\\");
        if !(has_keyed || has_definj) {
            continue;
        }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }

        let mut reader = Reader::from_str(&content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        struct ElementFrame {
            name: String,
            line: Option<usize>,
            has_text: bool,
            buffer: String,
        }
        let mut stack: Vec<ElementFrame> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let offset = reader.buffer_position();
                    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                    let line = line_for_offset(offset, &line_starts);
                    stack.push(ElementFrame {
                        name,
                        line,
                        has_text: false,
                        buffer: String::new(),
                    });
                }
                Ok(Event::End(_)) => {
                    if let Some(frame) = stack.pop() {
                        // Optional: emit nested dotted keys under LanguageData when enabled
                        let keyed_nested = std::env::var("RIMLOC_KEYED_NESTED").ok().map_or(false, |v| v == "1");
                        if keyed_nested {
                            if frame.has_text && !frame.name.is_empty() {
                                // stack after pop contains ancestors; expect root[0] == LanguageData
                                if stack.first().map(|f| f.name.eq_ignore_ascii_case("LanguageData")).unwrap_or(false) && stack.len() >= 2 {
                                    let mut parts: Vec<String> = stack.iter().skip(1).map(|f| f.name.clone()).collect();
                                    parts.push(frame.name.clone());
                                    if !parts.iter().any(|p| p.eq_ignore_ascii_case("li")) {
                                        let key = parts.join(".");
                                        out.push(TransUnit { key, source: Some(frame.buffer.clone()), path: p.to_path_buf(), line: frame.line });
                                        continue;
                                    }
                                }
                            }
                        }
                        // If closing a <li> directly under a top-level key, fold into the parent buffer
                        if frame.name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                            if let Some(parent) = stack.last_mut() {
                                if !parent.buffer.is_empty() {
                                    parent.buffer.push('\n');
                                }
                                if !frame.buffer.is_empty() {
                                    parent.buffer.push_str(&frame.buffer);
                                    parent.has_text = true;
                                }
                            }
                            continue;
                        }

                        // Closing a top-level <Key> under <LanguageData>
                        if stack.len() == 1 && !frame.name.is_empty() {
                            let source = if frame.has_text {
                                frame.buffer
                            } else {
                                String::new()
                            };
                            out.push(TransUnit {
                                key: frame.name,
                                source: Some(source),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
                        } else if keyed_nested_enabled()
                            && stack
                                .last()
                                .map(|f| f.name == "LanguageData")
                                .unwrap_or(false)
                            && !frame.name.is_empty()
                            && frame.name != "LineBreak"
                            && frame.has_text
                        {
                            // Optional nested keyed: dotted path under LanguageData
                            let mut parts: Vec<&str> = stack
                                .iter()
                                .skip(1)
                                .map(|f| f.name.as_str())
                                .collect();
                            parts.push(&frame.name);
                            let key = parts.join(".");
                            out.push(TransUnit {
                                key,
                                source: Some(frame.buffer),
                                path: p.to_path_buf(),
                                line: frame.line,
                            });
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let offset = reader.buffer_position();
                    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                    let line = line_for_offset(offset, &line_starts);
                    if stack.len() == 1
                        && stack
                            .last()
                            .map(|frame| frame.name == "LanguageData")
                            .unwrap_or(false)
                        && !name.is_empty()
                    {
                        out.push(TransUnit {
                            key: name,
                            source: Some(String::new()),
                            path: p.to_path_buf(),
                            line,
                        });
                    } else if stack.len() >= 2 {
                        // Support <LineBreak/> inside either a top-level key or nested <li>
                        if let Some(frame) = stack.last_mut() {
                            if name.eq_ignore_ascii_case("LineBreak") {
                                frame.has_text = true;
                                frame.buffer.push('\n');
                            }
                        }
                        // Self-closing <li/> directly under a top-level key contributes an empty line
                        if name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                            if let Some(parent) = stack.last_mut() {
                                if !parent.buffer.is_empty() {
                                    parent.buffer.push('\n');
                                }
                                parent.has_text = true;
                            }
                        }
                        // Optional nested empty key: produce dotted key
                        if keyed_nested_enabled()
                            && stack
                                .last()
                                .map(|f| f.name == "LanguageData")
                                .unwrap_or(false)
                            && name != "LineBreak"
                        {
                            let mut parts: Vec<&str> = stack
                                .iter()
                                .skip(1)
                                .map(|f| f.name.as_str())
                                .collect();
                            parts.push(&name);
                            let key = parts.join(".");
                            out.push(TransUnit {
                                key,
                                source: Some(String::new()),
                                path: p.to_path_buf(),
                                line,
                            });
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    let text = t
                        .unescape()
                        .unwrap_or_else(|_| {
                            Cow::Owned(String::from_utf8_lossy(t.as_ref()).into_owned())
                        })
                        .trim()
                        .to_string();
                    if let Some(frame) = stack.last_mut() {
                        if !text.is_empty() {
                            frame.buffer.push_str(&text);
                            frame.has_text = true;
                        }
                    }
                }
                Ok(Event::CData(t)) => {
                    let text = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                    if let Some(frame) = stack.last_mut() {
                        if !text.is_empty() {
                            frame.buffer.push_str(&text);
                            frame.has_text = true;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    Ok(out)
}

/// Scan RimWorld Defs XML to derive implicit English source keys like
/// "<defName>.<field>" for common translatable fields, e.g. label/description.
pub fn scan_defs_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    scan_defs_xml_under(root, None)
}

/// Same as `scan_defs_xml`, but restricts scanning to a specific `defs_root` when provided.
pub fn scan_defs_xml_under(root: &Path, defs_root: Option<&Path>) -> CoreResult<Vec<TransUnit>> {
    scan_defs_xml_under_with_fields(root, defs_root, &[])
}

/// Like `scan_defs_xml_under`, but allows adding extra field names to include.
/// Matching is case-insensitive and only considers immediate child elements under a Def entry.
pub fn scan_defs_xml_under_with_fields(
    root: &Path,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<TransUnit> = Vec::new();

    fn line_for_offset(offset: usize, starts: &[usize]) -> Option<usize> {
        if starts.is_empty() {
            return None;
        }
        match starts.binary_search(&offset) {
            Ok(idx) => Some(idx + 1),
            Err(idx) if idx > 0 => Some(idx),
            _ => Some(1),
        }
    }

    // Cross-file index for shallow field inheritance
    use std::collections::HashMap;
    let mut defs_index: HashMap<String, HashMap<String, std::path::PathBuf>> = HashMap::new();
    let mut name_index: HashMap<String, HashMap<String, std::path::PathBuf>> = HashMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let p_str = p.to_string_lossy();
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            p_str.contains("/Defs/") || p_str.contains("\\Defs\\")
        };
        if !in_scope {
            continue;
        }
        let Ok(content) = fs::read_to_string(p) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&content) else {
            continue;
        };
        for node in doc.root_element().children().filter(|n| n.is_element()) {
            let def_type = node.tag_name().name().to_string();
            if let Some(nm) = node.attribute("Name").map(|s| s.to_string()) {
                name_index
                    .entry(def_type.clone())
                    .or_default()
                    .entry(nm)
                    .or_insert_with(|| p.to_path_buf());
            }
            if let Some(def_name) = node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                defs_index
                    .entry(def_type)
                    .or_default()
                    .entry(def_name.to_string())
                    .or_insert_with(|| p.to_path_buf());
            }
        }
    }

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Defs/....xml (including versioned folders like 1.4/Defs, v1.6/Defs)
        let p_str = p.to_string_lossy();
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            p_str.contains("/Defs/") || p_str.contains("\\Defs\\")
        };
        if !in_scope {
            continue;
        }

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Precompute line starts for approximate line mapping
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }

        // Use a lightweight DOM for comfortable traversal
        let doc = match roxmltree::Document::parse(&content) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Recognized translatable fields commonly present across Def types (conservative defaults).
        const DEFAULT_FIELDS: &[&str] = &[
            "label",
            "labelShort",
            "labelPlural",
            "description",
            "helpText",
            "reportString",
            "gerundLabel",
        ];
        let mut all_fields: Vec<String> = DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect();
        for s in extra_fields {
            if !s.trim().is_empty() {
                all_fields.push(s.trim().to_string());
            }
        }

        for node in doc.root_element().descendants().filter(|n| n.is_element()) {
            // Def entries live directly under <Defs> or nested under lists, but
            // they always contain a <defName> child with text.
            let def_name = node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(def_name) = def_name else { continue };

            // For each known field present as an immediate child element, emit a unit
            for field in &all_fields {
                let mut found_val: Option<String> = None;
                let mut line: Option<usize> = None;
                if let Some(fnode) = node
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(field))
                {
                    found_val = fnode.text().map(|t| t.trim().to_string());
                    line = line_for_offset(fnode.range().start, &line_starts);
                }
                if found_val.is_none() && inherit_enabled() {
                    // try same-file ParentName chain by walking ancestors with matching def type
                    let root_el_local = doc.root_element();
                    // climb within same document
                    let mut current = node;
                    let def_tag = current.tag_name().name();
                    let mut guard = 0;
                    while found_val.is_none() {
                        guard += 1;
                        if guard > 16 {
                            break;
                        }
                        let Some(parent_name) = current.attribute("ParentName") else {
                            break;
                        };
                        let mut next_parent = root_el_local.children().find(|c| {
                            c.is_element()
                                && c.tag_name().name().eq_ignore_ascii_case(def_tag)
                                && c.attribute("Name").is_some_and(|n| n == parent_name)
                        });
                        if next_parent.is_none() {
                            next_parent = root_el_local.children().find(|c| {
                                c.is_element()
                                    && c.tag_name().name().eq_ignore_ascii_case(def_tag)
                                    && c.children()
                                        .find(|n| n.is_element() && n.tag_name().name() == "defName")
                                        .and_then(|n| n.text())
                                        .map(str::trim)
                                        .is_some_and(|n| n == parent_name)
                            });
                        }
                        if let Some(parent) = next_parent {
                            if let Some(fnode) = parent.children().find(|c| {
                                c.is_element() && c.tag_name().name().eq_ignore_ascii_case(field)
                            }) {
                                if let Some(val) = fnode.text().map(str::trim) {
                                    if !val.is_empty() {
                                        found_val = Some(val.to_string());
                                        line = line_for_offset(node.range().start, &line_starts);
                                        break;
                                    }
                                }
                            }
                            current = parent;
                        } else {
                            break;
                        }
                    }
                }
                if found_val.is_none() && inherit_enabled() {
                    if let Some(parent_name) = node.attribute("ParentName") {
                        if let Some(val) = find_field_in_parents_across_files_simple(&defs_index, &name_index, node.tag_name().name(), parent_name, field) {
                            found_val = Some(val);
                            line = line_for_offset(node.range().start, &line_starts);
                        }
                    }
                }
                if let Some(val) = found_val {
                    out.push(TransUnit {
                        key: format!("{}.{}", def_name, field),
                        source: Some(val),
                        path: p.to_path_buf(),
                        line,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Scan both Languages (Keyed/DefInjected) and Defs (implicit English) to provide
/// a complete view of translation units present in a mod.
pub fn scan_all_units(root: &Path) -> CoreResult<Vec<TransUnit>> {
    scan_all_units_with_defs(root, None)
}

/// Scan Languages (Keyed/DefInjected) and Defs with optional override of Defs root path.
pub fn scan_all_units_with_defs(
    root: &Path,
    defs_root: Option<&Path>,
) -> CoreResult<Vec<TransUnit>> {
    scan_all_units_with_defs_and_fields(root, defs_root, &[])
}

/// Scan with optional Defs root and extra fields for Defs extraction.
pub fn scan_all_units_with_defs_and_fields(
    root: &Path,
    defs_root: Option<&Path>,
    extra_fields: &[String],
) -> CoreResult<Vec<TransUnit>> {
    let mut units = scan_keyed_xml(root)?;
    if let Ok(mut defs) = scan_defs_xml_under_with_fields(root, defs_root, extra_fields) {
        units.append(&mut defs);
    }
    Ok(units)
}

// --------------------------
// Defs dictionary support
// --------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct DefsDict(pub std::collections::HashMap<String, Vec<String>>);

pub fn load_embedded_defs_dict() -> DefsDict {
    static JSON_BYTES: &[u8] = include_bytes!("../assets/defs_fields.json");
    let json = std::str::from_utf8(JSON_BYTES).unwrap_or("{}");
    let map: std::collections::HashMap<String, Vec<String>> =
        serde_json::from_str(json).unwrap_or_default();
    DefsDict(map)
}

pub fn load_defs_dict_from_str(s: &str) -> CoreResult<DefsDict> {
    let map: std::collections::HashMap<String, Vec<String>> = serde_json::from_str(s)?;
    Ok(DefsDict(map))
}

pub fn load_defs_dict_from_file(path: &Path) -> CoreResult<DefsDict> {
    let s = fs::read_to_string(path)?;
    load_defs_dict_from_str(&s)
}

pub fn merge_defs_dicts(dicts: &[DefsDict]) -> DefsDict {
    use std::collections::{BTreeSet, HashMap};
    let mut out: HashMap<String, BTreeSet<String>> = HashMap::new();
    for d in dicts {
        for (k, v) in &d.0 {
            let e = out.entry(k.clone()).or_default();
            for s in v {
                e.insert(s.clone());
            }
        }
    }
    let mut flat = std::collections::HashMap::new();
    for (k, v) in out {
        flat.insert(k, v.into_iter().collect());
    }
    DefsDict(flat)
}

/// Navigate a roxmltree node by a dot path like `ingestible.ingestCommandString` or `ingredients.li.label`.
fn collect_values_by_path<'a>(
    node: roxmltree::Node<'a, 'a>,
    path: &[&str],
    out: &mut Vec<&'a str>,
) {
    if path.is_empty() {
        if let Some(t) = node.text() {
            let t = t.trim();
            if !t.is_empty() {
                out.push(t);
            }
        }
        return;
    }
    let mut head = path[0];
    // Allow markers like li{h} to denote handle-preferred list segments; strip markers here
    if let Some(pos) = head.find('{') {
        head = &head[..pos];
    }
    let tail = &path[1..];
    if head.eq_ignore_ascii_case("li") {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
        {
            collect_values_by_path(child, tail, out);
        }
    } else {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(head))
        {
            collect_values_by_path(child, tail, out);
        }
    }
}

fn inherit_enabled() -> bool {
    match std::env::var("RIMLOC_INHERIT") {
        Ok(val) if val.trim() == "0" => false,
        _ => true,
    }
}

// Helper for shallow-field inheritance across files by ParentName.
fn find_field_in_parents_across_files_simple(
    index_defname: &std::collections::HashMap<
        String,
        std::collections::HashMap<String, std::path::PathBuf>,
    >,
    index_name: &std::collections::HashMap<
        String,
        std::collections::HashMap<String, std::path::PathBuf>,
    >,
    def_type: &str,
    start_parent_name: &str,
    field: &str,
) -> Option<String> {
    let mut current = Some(start_parent_name.to_string());
    let mut guard = 0;
    while let Some(name) = current {
        guard += 1;
        if guard > 32 {
            break;
        }
        let path = index_name
            .get(def_type)
            .and_then(|m| m.get(&name))
            .cloned()
            .or_else(|| index_defname.get(def_type).and_then(|m| m.get(&name)).cloned())?;
        let content = std::fs::read_to_string(path).ok()?;
        let doc = roxmltree::Document::parse(&content).ok()?;
        let root_el = doc.root_element();
        let def_node = root_el.children().find(|c| {
            c.is_element()
                && c.tag_name().name().eq_ignore_ascii_case(def_type)
                && (c.attribute("Name").is_some_and(|n| n == name)
                    || c.children()
                        .find(|n| n.is_element() && n.tag_name().name() == "defName")
                        .and_then(|n| n.text())
                        .map(str::trim)
                        .is_some_and(|t| t == name))
        });
        let Some(def_node) = def_node else { break };
        if let Some(fnode) = def_node
            .children()
            .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(field))
        {
            if let Some(val) = fnode.text().map(str::trim) {
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
        current = def_node.attribute("ParentName").map(|s| s.to_string());
    }
    None
}

/// Scan Defs using a dictionary of field paths per DefType and optional extra shallow fields.
#[derive(Debug, Clone)]
pub struct DefsMetaUnit {
    pub unit: TransUnit,
    pub def_type: String,
    pub def_name: String,
    pub field_path: String,
}

pub fn scan_defs_with_dict(
    root: &Path,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> CoreResult<Vec<TransUnit>> {
    Ok(
        scan_defs_with_dict_meta(root, defs_root, dict, extra_fields)?
            .into_iter()
            .map(|m| m.unit)
            .collect(),
    )
}

pub fn scan_defs_with_dict_meta(
    root: &Path,
    defs_root: Option<&Path>,
    dict: &std::collections::HashMap<String, Vec<String>>,
    extra_fields: &[String],
) -> CoreResult<Vec<DefsMetaUnit>> {
    use walkdir::WalkDir;
    let mut out: Vec<DefsMetaUnit> = Vec::new();

    // Build cross-file indexes:
    // - defType -> defName -> Path
    // - defType -> Name attribute -> Path
    use std::collections::HashMap;
    let mut defs_index: HashMap<String, HashMap<String, std::path::PathBuf>> = HashMap::new();
    let mut name_index: HashMap<String, HashMap<String, std::path::PathBuf>> = HashMap::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            let s = p.to_string_lossy();
            s.contains("/Defs/") || s.contains("\\Defs\\")
        };
        if !in_scope {
            continue;
        }
        let Ok(content) = fs::read_to_string(p) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&content) else {
            continue;
        };
        for node in doc.root_element().children().filter(|n| n.is_element()) {
            let def_type = node.tag_name().name().to_string();
            if let Some(nm) = node.attribute("Name").map(|s| s.to_string()) {
                name_index
                    .entry(def_type.clone())
                    .or_default()
                    .entry(nm)
                    .or_insert_with(|| p.to_path_buf());
            }
            if let Some(def_name) = node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                defs_index
                    .entry(def_type)
                    .or_default()
                    .entry(def_name.to_string())
                    .or_insert_with(|| p.to_path_buf());
            }
        }
    }

    // Helper: cross-file parent chain for a dot path
    fn collect_values_in_parents_across_files(
        index_defname: &std::collections::HashMap<
            String,
            std::collections::HashMap<String, std::path::PathBuf>,
        >,
        index_name: &std::collections::HashMap<
            String,
            std::collections::HashMap<String, std::path::PathBuf>,
        >,
        def_type: &str,
        start_parent_name: &str,
        segs: &[&str],
        out_vals: &mut Vec<String>,
    ) {
        let mut current = Some(start_parent_name.to_string());
        let mut guard = 0;
        while let Some(name) = current {
            guard += 1;
            if guard > 32 {
                break;
            }
            // Prefer Name index, then fall back to defName index
            let path_opt = index_name
                .get(def_type)
                .and_then(|m| m.get(&name))
                .cloned()
                .or_else(|| {
                    index_defname
                        .get(def_type)
                        .and_then(|m| m.get(&name))
                        .cloned()
                });
            let Some(path) = path_opt else { break };
            let Ok(content) = std::fs::read_to_string(path) else {
                break;
            };
            let Ok(doc) = roxmltree::Document::parse(&content) else {
                break;
            };
            let root_el = doc.root_element();
            // Try by Name attribute first, then defName element
            let mut maybe = root_el.children().find(|c| {
                c.is_element()
                    && c.tag_name().name().eq_ignore_ascii_case(def_type)
                    && c.attribute("Name").is_some_and(|n| n == name)
            });
            if maybe.is_none() {
                maybe = root_el.children().find(|c| {
                    c.is_element()
                        && c.tag_name().name().eq_ignore_ascii_case(def_type)
                        && c.children()
                            .find(|n| n.is_element() && n.tag_name().name() == "defName")
                            .and_then(|n| n.text())
                            .map(str::trim)
                            .is_some_and(|t| t == name)
                });
            }
            let def_node = match maybe {
                Some(n) => n,
                None => break,
            };
            let mut vals_local = Vec::new();
            collect_values_by_path(def_node, segs, &mut vals_local);
            if !vals_local.is_empty() {
                out_vals.extend(vals_local.into_iter().map(|s| s.to_string()));
                break;
            }
            current = def_node.attribute("ParentName").map(|s| s.to_string());
        }
    }
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|e| e.to_str())
            .map_or(true, |ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            let s = p.to_string_lossy();
            s.contains("/Defs/") || s.contains("\\Defs\\")
        };
        if !in_scope {
            continue;
        }
        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let doc = match roxmltree::Document::parse(&content) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }
        let root_el = doc.root_element();
        for def_node in root_el.children().filter(|n| n.is_element()) {
            let def_type = def_node.tag_name().name().to_string();
            let def_name = def_node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            if def_name.is_empty() {
                continue;
            }
            // Dict paths for this type
            if let Some(paths) = dict.get(&def_type) {
                for path in paths {
                    let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
                    // Prepare display path without any {..} markers (e.g., li{h} -> li)
                    let display_path = segs
                        .iter()
                        .map(|s| if let Some(pos) = s.find('{') { &s[..pos] } else { s })
                        .collect::<Vec<&str>>()
                        .join(".");
                    let mut vals = Vec::new();
                    collect_values_by_path(def_node, &segs, &mut vals);
                    if vals.is_empty() && inherit_enabled() {
                        // In-file ParentName chain; respect Inherit="false"
                        if !def_node
                            .attribute("Inherit")
                            .map(|v| v.eq_ignore_ascii_case("false"))
                            .unwrap_or(false)
                        {
                            let mut current = def_node;
                            let mut guard = 0;
                            while vals.is_empty() {
                                guard += 1;
                                if guard > 16 {
                                    break;
                                }
                                let Some(parent_name) = current.attribute("ParentName") else {
                                    break;
                                };
                                if let Some(parent) = root_el.children().find(|c| {
                                    c.is_element()
                                        && c.tag_name().name().eq_ignore_ascii_case(&def_type)
                                        && c.attribute("Name").is_some_and(|n| n == parent_name)
                                }) {
                                    collect_values_by_path(parent, &segs, &mut vals);
                                    current = parent;
                                } else if let Some(parent) = root_el.children().find(|c| {
                                    c.is_element()
                                        && c.tag_name().name().eq_ignore_ascii_case(&def_type)
                                        && c.children()
                                            .find(|n| {
                                                n.is_element() && n.tag_name().name() == "defName"
                                            })
                                            .and_then(|n| n.text())
                                            .map(str::trim)
                                            .is_some_and(|n| n == parent_name)
                                }) {
                                    collect_values_by_path(parent, &segs, &mut vals);
                                    current = parent;
                                } else {
                                    break;
                                }
                            }
                        }
                        if vals.is_empty() && inherit_enabled() {
                            if let Some(parent_name) = def_node.attribute("ParentName") {
                                let mut vals2: Vec<String> = Vec::new();
                                collect_values_in_parents_across_files(
                                    &defs_index,
                                    &name_index,
                                    &def_type,
                                    parent_name,
                                    &segs,
                                    &mut vals2,
                                );
                                for v in vals2 {
                                    let line = def_node.range().start;
                                    let line = Some(match line_starts.binary_search(&line) {
                                        Ok(idx) => idx + 1,
                                        Err(idx) if idx > 0 => idx,
                                        _ => 1,
                                    });
                                    out.push(DefsMetaUnit {
                                        unit: TransUnit {
                                            key: format!("{}.{path}", def_name),
                                            source: Some(v),
                                            path: p.to_path_buf(),
                                            line,
                                        },
                                        def_type: def_type.clone(),
                                        def_name: def_name.clone(),
                                        field_path: path.clone(),
                                    });
                                }
                            }
                        }
                    }
                    for v in vals {
                        // approximate line: start from first segment if present
                        let line = def_node.range().start;
                        let line = Some(match line_starts.binary_search(&line) {
                            Ok(idx) => idx + 1,
                            Err(idx) if idx > 0 => idx,
                            _ => 1,
                        });
                        out.push(DefsMetaUnit {
                            unit: TransUnit {
                                key: format!("{}.{}", def_name, display_path),
                                source: Some(v.to_string()),
                                path: p.to_path_buf(),
                                line,
                            },
                            def_type: def_type.clone(),
                            def_name: def_name.clone(),
                            field_path: display_path.clone(),
                        });
                    }
                }
            }
            // Shallow extra fields (immediate children)
            for f in extra_fields {
                let mut produced = false;
                if let Some(fnode) = def_node
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case(f))
                {
                    if let Some(val) = fnode.text().map(str::trim) {
                        let line = fnode.range().start;
                        let line = Some(match line_starts.binary_search(&line) {
                            Ok(idx) => idx + 1,
                            Err(idx) if idx > 0 => idx,
                            _ => 1,
                        });
                        out.push(DefsMetaUnit {
                            unit: TransUnit {
                                key: format!("{}.{}", def_name, f),
                                source: Some(val.to_string()),
                                path: p.to_path_buf(),
                                line,
                            },
                            def_type: def_type.clone(),
                            def_name: def_name.clone(),
                            field_path: f.clone(),
                        });
                        produced = true;
                    }
                }
                if !produced && inherit_enabled() {
                    // In-file parents; respect Inherit="false"
                    if !def_node
                        .attribute("Inherit")
                        .map(|v| v.eq_ignore_ascii_case("false"))
                        .unwrap_or(false)
                    {
                        let mut current = def_node;
                        let mut guard = 0;
                        while !produced {
                            guard += 1;
                            if guard > 16 {
                                break;
                            }
                            let Some(parent_name) = current.attribute("ParentName") else {
                                break;
                            };
                            let next_parent = root_el
                                .children()
                                .find(|c| {
                                    c.is_element()
                                        && c.tag_name().name().eq_ignore_ascii_case(&def_type)
                                        && c.attribute("Name").is_some_and(|n| n == parent_name)
                                })
                                .or_else(|| {
                                    root_el.children().find(|c| {
                                        c.is_element()
                                            && c.tag_name().name().eq_ignore_ascii_case(&def_type)
                                            && c.children()
                                                .find(|n| {
                                                    n.is_element()
                                                        && n.tag_name().name() == "defName"
                                                })
                                                .and_then(|n| n.text())
                                                .map(str::trim)
                                                .is_some_and(|n| n == parent_name)
                                    })
                                });
                            if let Some(parent) = next_parent {
                                if let Some(n) = parent.children().find(|c| {
                                    c.is_element() && c.tag_name().name().eq_ignore_ascii_case(f)
                                }) {
                                    if let Some(val) = n.text().map(str::trim) {
                                        let line = def_node.range().start;
                                        let line = Some(match line_starts.binary_search(&line) {
                                            Ok(idx) => idx + 1,
                                            Err(idx) if idx > 0 => idx,
                                            _ => 1,
                                        });
                                        out.push(DefsMetaUnit {
                                            unit: TransUnit {
                                                key: format!("{}.{}", def_name, f),
                                                source: Some(val.to_string()),
                                                path: p.to_path_buf(),
                                                line,
                                            },
                                            def_type: def_type.clone(),
                                            def_name: def_name.clone(),
                                            field_path: f.clone(),
                                        });
                                        produced = true;
                                        break;
                                    }
                                }
                                current = parent;
                            } else {
                                break;
                            }
                        }
                    }
                }
                if !produced && inherit_enabled() {
                    if let Some(parent_name) = def_node.attribute("ParentName") {
                        let segs: Vec<&str> = std::slice::from_ref(&f.as_str()).to_vec();
                        let mut vals2: Vec<String> = Vec::new();
                        collect_values_in_parents_across_files(
                            &defs_index,
                            &name_index,
                            &def_type,
                            parent_name,
                            &segs,
                            &mut vals2,
                        );
                        if let Some(val) = vals2.into_iter().find(|v| !v.trim().is_empty()) {
                            let line = def_node.range().start;
                            let line = Some(match line_starts.binary_search(&line) {
                                Ok(idx) => idx + 1,
                                Err(idx) if idx > 0 => idx,
                                _ => 1,
                            });
                            out.push(DefsMetaUnit {
                                unit: TransUnit {
                                    key: format!("{}.{}", def_name, f),
                                    source: Some(val),
                                    path: p.to_path_buf(),
                                    line,
                                },
                                def_type: def_type.clone(),
                                def_name: def_name.clone(),
                                field_path: f.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_keyed_xml_handles_self_closing_keys() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mods/TestMod/Languages/TestLang/Keyed");
        fs::create_dir_all(&keyed_dir)?;

        let file_path = keyed_dir.join("SelfClosing.xml");
        fs::write(
            &file_path,
            r#"<LanguageData>
    <FullKey>Hello RimWorld</FullKey>
    <EmptyKey/>
    <Nested>
        <NestedEmpty/>
    </Nested>
</LanguageData>
"#,
        )?;

        let units = scan_keyed_xml(dir.path())?;

        assert!(
            units
                .iter()
                .any(|u| u.key == "FullKey" && u.source.as_deref() == Some("Hello RimWorld")),
            "FullKey should be parsed with text",
        );

        let empty = units
            .iter()
            .find(|u| u.key == "EmptyKey")
            .expect("EmptyKey should be produced for self-closing elements");
        assert_eq!(empty.source.as_deref(), Some(""));
        assert_eq!(empty.path, file_path);
        assert!(empty.line.is_some());

        assert!(
            units.iter().all(|u| u.key != "NestedEmpty"),
            "Nested self-closing keys should not be emitted",
        );

        Ok(())
    }

    #[test]
    fn scan_keyed_xml_merges_fragmented_text() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mods/TestMod/Languages/TestLang/Keyed");
        fs::create_dir_all(&keyed_dir)?;

        let file_path = keyed_dir.join("Fragments.xml");
        fs::write(
            &file_path,
            r#"<LanguageData>
    <KeyWithBreak>Part<LineBreak/>Rest</KeyWithBreak>
</LanguageData>
"#,
        )?;

        let units = scan_keyed_xml(dir.path())?;

        let unit = units
            .iter()
            .find(|u| u.key == "KeyWithBreak")
            .expect("KeyWithBreak should be parsed");

        assert_eq!(unit.source.as_deref(), Some("Part\nRest"));
        assert_eq!(unit.path, file_path);

        Ok(())
    }
}

#[cfg(test)]
mod defs_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_defs_extracts_common_fields() -> CoreResult<()> {
        let dir = tempdir()?;
        let defs_dir = dir.path().join("Mods/TestMod/Defs/ThingDefs_Items");
        fs::create_dir_all(&defs_dir)?;
        let file_path = defs_dir.join("Apparel.xml");
        fs::write(
            &file_path,
            r#"<Defs>
  <ThingDef>
    <defName>Apparel_Parka</defName>
    <label>parka</label>
    <description>A warm parka for cold climates.</description>
  </ThingDef>
</Defs>
"#,
        )?;

        let units = scan_defs_xml(dir.path())?;
        assert!(units
            .iter()
            .any(|u| u.key == "Apparel_Parka.label" && u.source.as_deref() == Some("parka")));
        assert!(units.iter().any(|u| u.key == "Apparel_Parka.description"
            && u.source.as_deref() == Some("A warm parka for cold climates.")));
        Ok(())
    }
}
