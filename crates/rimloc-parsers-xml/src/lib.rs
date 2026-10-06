pub use rimloc_core::parse_simple_po as parse_po_string;

use quick_xml::events::BytesRef;
use quick_xml::events::Event;
use quick_xml::Reader;
use rimloc_core::path_text::has_path_marker;
use rimloc_core::{Result as CoreResult, TransUnit};

pub mod patches_extract;
pub use patches_extract::{
    is_patch_noise_field, scan_patch_values, PatchScanStats, PatchValueCandidate,
    PATCH_NOISE_FIELDS,
};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;

/// Options to tune Keyed/DefInjected scanning behaviour.
#[derive(Clone, Debug)]
pub struct KeyedScanOptions {
    /// Emit dotted keys for nested elements under `<LanguageData>`.
    /// Example: `<Root><Menu><Open>…</Open></Menu></Root>` -> `Menu.Open`.
    pub nested: bool,
    /// Join `<li>` items with a newline when collapsing list content into parent value.
    pub join_li_with_newline: bool,
    /// Include self-closing empty keys.
    pub include_empty_keys: bool,
    /// Process files in parallel where possible. Final result is sorted deterministically.
    pub parallel: bool,
    /// When scanning nested keys under `DefInjected/`, drop leading Def type segment
    /// (e.g., ThingDef.Apparel_Parka.label -> Apparel_Parka.label).
    pub definj_drop_def_type: bool,
}

impl Default for KeyedScanOptions {
    fn default() -> Self {
        // Preserve historical defaults unless gated via env
        let nested = matches!(std::env::var("RIMLOC_KEYED_NESTED"), Ok(v) if v.trim() == "1");
        let join_li_with_newline = true;
        let include_empty_keys = true;
        let parallel = matches!(std::env::var("RIMLOC_PARALLEL"), Ok(v) if v.trim() == "1");
        Self {
            nested,
            join_li_with_newline,
            include_empty_keys,
            parallel,
            definj_drop_def_type: true,
        }
    }
}

/// One open element in the Keyed scanner's event stack.
struct ElementFrame {
    name: String,
    line: Option<usize>,
    has_text: bool,
    buffer: String,
    /// Whitespace seen since the last committed chunk. Formatting indentation
    /// (a run carrying a newline) is dropped; a same-line run is content and
    /// survives to the value edges.
    pending_ws: String,
    /// Serialized `<name attrs>` for an inline markup child (any element
    /// inside a value except `li`, which folds with a newline instead).
    /// Empty when this element itself is a key, not markup.
    open_tag: String,
}

/// Whitespace semantics for keyed values (game-accurate): RimWorld reads the
/// element text as-is, so the parser returns the exact characters between the
/// tags minus FORMATTING INDENTATION — and a whitespace run is formatting
/// precisely when it carries a newline (indent alignment across lines). A
/// same-line run is content: `<Label> Search: </Label>` keeps both edge
/// spaces, because the trailing space of `Search: ` is meaningful when the
/// game concatenates strings.
///
/// Interior runs are always preserved verbatim (newlines inside a value are
/// the translator's line breaks, as with `<LineBreak/>`).
fn ws_run_is_formatting(run: &str) -> bool {
    run.contains('\n')
}

/// Commit a character-data chunk into a value buffer.
///
/// Whole-whitespace chunks defer into `pending_ws` unless they carry a
/// newline before any content (layout indentation). The first content chunk
/// drops its own leading run only when that run is indentation; a same-line
/// leading run is content. Interior whitespace (deferred runs between
/// content) is preserved verbatim.
fn commit_text_chunk(
    buffer: &mut String,
    pending_ws: &mut String,
    has_text: &mut bool,
    chunk: &str,
) {
    if chunk.chars().all(char::is_whitespace) {
        if !buffer.is_empty() || !ws_run_is_formatting(chunk) {
            pending_ws.push_str(chunk);
        }
        return;
    }
    if buffer.is_empty() {
        let trimmed = chunk.trim_start();
        let leading = &chunk[..chunk.len() - trimmed.len()];
        // `pending_ws` before any content only ever holds same-line runs
        // (newline runs are dropped on sight), i.e. deferred content.
        let deferred = std::mem::take(pending_ws);
        buffer.push_str(&deferred);
        match leading.find('\n') {
            // Same-line head is content; the newline + indent behind it is layout.
            Some(head) => buffer.push_str(&leading[..head]),
            // Whole run stays on the open-tag line: content edge (` Search:`).
            None => buffer.push_str(leading),
        }
        buffer.push_str(trimmed);
    } else {
        let ws = std::mem::take(pending_ws);
        buffer.push_str(&ws);
        buffer.push_str(chunk);
    }
    *has_text = true;
}

/// Commit a decoded entity reference (`&lt;` -> `<`, `&#38;` -> `&`).
/// A reference always carries content — even when it decodes to whitespace
/// (`&#32;`) — but formatting indentation still precedes it.
fn commit_ref_chunk(
    buffer: &mut String,
    pending_ws: &mut String,
    has_text: &mut bool,
    decoded: &str,
) {
    if !buffer.is_empty() || !ws_run_is_formatting(pending_ws) {
        // Interior run, or a deferred same-line leading run (content edge
        // before the reference: `<Label> &lt;b&gt;…` keeps its space).
        let ws = std::mem::take(pending_ws);
        buffer.push_str(&ws);
    } else {
        pending_ws.clear();
    }
    buffer.push_str(decoded);
    *has_text = true;
}

/// Close a finished value: decide its trailing edge. Everything from the last
/// newline to the closing tag (the final line break + closing indent) is
/// formatting and is cut; same-line whitespace before it — or a trailing run
/// with no newline at all (`Search: `) — is content and stays.
fn close_value_buffer(buffer: &mut String, pending_ws: &mut String) {
    if ws_run_is_formatting(pending_ws) {
        // Pending carries the final newline: drop it together with the
        // closing indent already committed to the buffer behind it.
        pending_ws.clear();
        if let Some(pos) = buffer.rfind('\n') {
            if buffer[pos + 1..].chars().all(char::is_whitespace) {
                buffer.truncate(pos);
            }
        }
        return;
    }
    if !pending_ws.is_empty() {
        let ws = std::mem::take(pending_ws);
        buffer.push_str(&ws);
    }
    if let Some(pos) = buffer.rfind('\n') {
        if buffer[pos + 1..].chars().all(char::is_whitespace) {
            buffer.truncate(pos);
        }
    }
}

/// Re-serialize an inline markup child opening tag (`<b>`, `<color r="1">`)
/// so real elements round-trip into the value instead of vanishing together
/// with their text. Attribute values are copied raw (entities intact).
fn serialize_start_tag(name: &str, attrs: quick_xml::events::attributes::Attributes<'_>) -> String {
    let mut tag = String::from("<");
    tag.push_str(name);
    for attr in attrs {
        let Ok(attr) = attr else { break };
        tag.push(' ');
        tag.push_str(attr.key.as_ref());
        tag.push_str("=\"");
        // Raw value: escape sequences stay as written, round-trip is exact.
        tag.push_str(&attr.value);
        tag.push('"');
    }
    tag.push('>');
    tag
}

/// Decode a `&ref;` event into its in-game text: predefined XML entities
/// (`lt/gt/amp/quot/apos`) and numeric character references. Unknown names
/// (custom DTD entities) are skipped — RimWorld never declares any.
fn decode_general_ref(r: &BytesRef<'_>) -> Option<String> {
    if r.is_char_ref() {
        r.resolve_char_ref().ok().flatten().map(|c| c.to_string())
    } else {
        quick_xml::escape::resolve_xml_entity(r.xml10_content().as_ref()).map(str::to_string)
    }
}

/// Very small XML scanner for RimWorld "Keyed" XML files.
/// It walks `root` and finds files that match `*/Languages/*/{Keyed,DefInjected}/*.xml`.
/// For every `<Key>Value</Key>` pair found, it produces a `TransUnit` with
/// the key name, text value, file path and (approximate) line number.
pub fn scan_keyed_xml(root: &Path) -> CoreResult<Vec<TransUnit>> {
    scan_keyed_xml_with_options(root, &KeyedScanOptions::default())
}

/// Same as `scan_keyed_xml` but allows configuring behaviour via `KeyedScanOptions`.
pub fn scan_keyed_xml_with_options(
    root: &Path,
    opts: &KeyedScanOptions,
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

    // (no cross-file logic in keyed scanner)

    // Collect candidate files first to allow optional parallel processing
    let mut files: Vec<std::path::PathBuf> = Vec::new();
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
        // filter to .../Languages/<Locale>/{Keyed,DefInjected}/....xml
        let p_str = p.to_string_lossy();
        // Separator-agnostic markers: a Windows path may mix `/` and `\`
        // in one spelling (e.g. `--tm-root D:\mod\Languages/Russian`) —
        // same-separator contains() used to silently collect NOTHING for
        // such roots, emptying the TM merge on export (windows CI).
        let in_languages = has_path_marker(&p_str, "Languages");
        if !in_languages {
            continue;
        }
        let has_keyed = has_path_marker(&p_str, "Keyed");
        let has_definj = has_path_marker(&p_str, "DefInjected");
        if !(has_keyed || has_definj) {
            continue;
        }
        files.push(p.to_path_buf());
    }

    #[allow(dead_code)]
    fn is_def_injected_path(p: &std::path::Path) -> bool {
        def_injected_type_from_path(p).is_some()
    }

    fn def_injected_type_from_path(p: &std::path::Path) -> Option<String> {
        // Find segment immediately following "DefInjected" in the path
        let mut seen = false;
        for c in p.components() {
            let s = c.as_os_str().to_string_lossy();
            if seen {
                return Some(s.into_owned());
            }
            if s.eq_ignore_ascii_case("DefInjected") {
                seen = true;
            }
        }
        None
    }

    fn drop_def_type_if_needed(
        mut key: String,
        maybe_def_type: Option<&str>,
        drop: bool,
    ) -> String {
        if !(drop) {
            return key;
        }
        let Some(pos) = key.find('.') else { return key };
        let head = &key[..pos];
        if let Some(dt) = maybe_def_type {
            if head.eq_ignore_ascii_case(dt) {
                key.drain(..=pos);
            }
        } else if head.ends_with("Def") {
            // Fallback: when path parsing failed but it still looks like a Def type
            key.drain(..=pos);
        }
        key
    }

    let process_one = |p: &std::path::PathBuf| -> Vec<TransUnit> {
        let mut local: Vec<TransUnit> = Vec::new();

        let content = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => return local,
        };
        let mut line_starts = Vec::new();
        line_starts.push(0usize);
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }

        let mut reader = Reader::from_str(&content);
        // Whitespace is trimmed manually per element value: quick-xml 0.42
        // splits character data around `&ref;` into separate Text/GeneralRef
        // events, so reader-level trimming would eat spaces that surround an
        // inline tag (`after &lt;b&gt;Core` -> `after<b>Core`). Those spaces
        // are part of the translator-visible string.
        reader.config_mut().trim_text(false);
        let mut buf = Vec::new();
        let mut stack: Vec<ElementFrame> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name().as_ref().to_owned();
                    let offset = reader.buffer_position();
                    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                    let line = line_for_offset(offset, &line_starts);
                    // An element nested inside a value is inline markup and
                    // round-trips verbatim (`<b>Search:</b>`); `li` folds
                    // into the parent with a newline instead, and root
                    // children are keys, not markup.
                    let open_tag = if stack.len() >= 2 && !name.eq_ignore_ascii_case("li") {
                        serialize_start_tag(&name, e.attributes())
                    } else {
                        String::new()
                    };
                    stack.push(ElementFrame {
                        name,
                        line,
                        has_text: false,
                        buffer: String::new(),
                        pending_ws: String::new(),
                        open_tag,
                    });
                }
                Ok(Event::End(_)) => {
                    if let Some(mut frame) = stack.pop() {
                        // Optional: emit nested dotted keys under LanguageData when enabled
                        if opts.nested && frame.has_text && !frame.name.is_empty() {
                            // stack after pop contains ancestors; expect root[0] == LanguageData
                            if stack
                                .first()
                                .map(|f| f.name.eq_ignore_ascii_case("LanguageData"))
                                .unwrap_or(false)
                                && stack.len() >= 2
                            {
                                let mut parts: Vec<String> =
                                    stack.iter().skip(1).map(|f| f.name.clone()).collect();
                                parts.push(frame.name.clone());
                                if !parts.iter().any(|p| p.eq_ignore_ascii_case("li")) {
                                    let def_type = def_injected_type_from_path(p);
                                    let key = drop_def_type_if_needed(
                                        parts.join("."),
                                        def_type.as_deref(),
                                        opts.definj_drop_def_type,
                                    );
                                    close_value_buffer(&mut frame.buffer, &mut frame.pending_ws);
                                    local.push(TransUnit {
                                        tkey: None,
                                        key,
                                        source: Some(std::mem::take(&mut frame.buffer)),
                                        path: p.clone(),
                                        line: frame.line,
                                        ..Default::default()
                                    });
                                    continue;
                                }
                            }
                        }
                        // If closing a <li> directly under a top-level key, fold into the parent buffer
                        if frame.name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                            if let Some(parent) = stack.last_mut() {
                                if opts.join_li_with_newline && !parent.buffer.is_empty() {
                                    let trimmed_len = parent.buffer.trim_end().len();
                                    parent.buffer.truncate(trimmed_len);
                                    parent.buffer.push('\n');
                                }
                                let li_text = frame.buffer.trim();
                                if !li_text.is_empty() {
                                    parent.buffer.push_str(li_text);
                                    parent.has_text = true;
                                }
                            }
                            continue;
                        }

                        // Closing a top-level <Key> under <LanguageData>
                        if stack.len() == 1 && !frame.name.is_empty() {
                            close_value_buffer(&mut frame.buffer, &mut frame.pending_ws);
                            let source = if frame.has_text {
                                std::mem::take(&mut frame.buffer)
                            } else if opts.include_empty_keys {
                                String::new()
                            } else {
                                // skip empty when not requested
                                String::new()
                            };
                            if opts.include_empty_keys || !source.is_empty() {
                                local.push(TransUnit {
                                    tkey: None,
                                    key: frame.name,
                                    source: Some(source),
                                    path: p.clone(),
                                    line: frame.line,
                                    ..Default::default()
                                });
                            }
                        } else if stack.len() >= 2 && !frame.name.eq_ignore_ascii_case("li") {
                            // Inline markup child inside a value: re-serialize
                            // it verbatim (`<b>Search:</b>`). It used to be
                            // dropped together with all of its text.
                            if let Some(parent) = stack.last_mut() {
                                if frame.has_text {
                                    parent.has_text = true;
                                }
                                let ws = std::mem::take(&mut parent.pending_ws);
                                parent.buffer.push_str(&ws);
                                parent.buffer.push_str(&frame.open_tag);
                                parent.buffer.push_str(&frame.buffer);
                                parent.buffer.push_str("</");
                                parent.buffer.push_str(&frame.name);
                                parent.buffer.push('>');
                            }
                        } else if opts.nested
                            && stack
                                .last()
                                .map(|f| f.name == "LanguageData")
                                .unwrap_or(false)
                            && !frame.name.is_empty()
                            && frame.name != "LineBreak"
                            && frame.has_text
                        {
                            // Optional nested keyed: dotted path under LanguageData
                            let mut parts: Vec<&str> =
                                stack.iter().skip(1).map(|f| f.name.as_str()).collect();
                            parts.push(&frame.name);
                            let def_type = def_injected_type_from_path(p);
                            let key = drop_def_type_if_needed(
                                parts.join("."),
                                def_type.as_deref(),
                                opts.definj_drop_def_type,
                            );
                            close_value_buffer(&mut frame.buffer, &mut frame.pending_ws);
                            local.push(TransUnit {
                                tkey: None,
                                key,
                                source: Some(std::mem::take(&mut frame.buffer)),
                                path: p.clone(),
                                line: frame.line,
                                ..Default::default()
                            });
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = e.name().as_ref().to_owned();
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
                        if opts.include_empty_keys {
                            local.push(TransUnit {
                                tkey: None,
                                key: name,
                                source: Some(String::new()),
                                path: p.clone(),
                                line,
                                ..Default::default()
                            });
                        }
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
                                if opts.join_li_with_newline && !parent.buffer.is_empty() {
                                    parent.buffer.push('\n');
                                }
                                parent.has_text = true;
                            }
                        }
                        // Self-closing inline markup inside a value (`<foo/>`)
                        // round-trips verbatim, like its paired sibling above.
                        if !name.eq_ignore_ascii_case("LineBreak")
                            && !name.eq_ignore_ascii_case("li")
                        {
                            if let Some(parent) = stack.last_mut() {
                                let ws = std::mem::take(&mut parent.pending_ws);
                                parent.buffer.push_str(&ws);
                                parent.buffer.push('<');
                                parent.buffer.push_str(&name);
                                for attr in e.attributes() {
                                    let Ok(attr) = attr else { break };
                                    parent.buffer.push(' ');
                                    parent.buffer.push_str(attr.key.as_ref());
                                    parent.buffer.push_str("=\"");
                                    parent.buffer.push_str(&attr.value);
                                    parent.buffer.push('"');
                                }
                                parent.buffer.push_str("/>");
                            }
                        }
                        // Optional nested empty key: produce dotted key
                        if opts.nested
                            && stack
                                .last()
                                .map(|f| f.name == "LanguageData")
                                .unwrap_or(false)
                            && name != "LineBreak"
                        {
                            let mut parts: Vec<&str> =
                                stack.iter().skip(1).map(|f| f.name.as_str()).collect();
                            parts.push(&name);
                            let def_type = def_injected_type_from_path(p);
                            let key = drop_def_type_if_needed(
                                parts.join("."),
                                def_type.as_deref(),
                                opts.definj_drop_def_type,
                            );
                            local.push(TransUnit {
                                tkey: None,
                                key,
                                source: Some(String::new()),
                                path: p.clone(),
                                line,
                                ..Default::default()
                            });
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    // quick-xml 0.42 carves `&ref;` out of character data and
                    // emits it as a separate GeneralRef event, so a single
                    // logical text run arrives as several chunks. Chunks are
                    // committed with whitespace bookkeeping instead of a
                    // per-chunk trim, which used to erase spaces around inline
                    // tags and lose the tags themselves (MUST_FIX_BEFORE_BETA):
                    // `<b>The HugsLib mod</b>` degenerated into `bThe HugsLib
                    // mod/b`.
                    let text = t.xml10_content().to_string();
                    if let Some(frame) = stack.last_mut() {
                        let ElementFrame {
                            buffer,
                            pending_ws,
                            has_text,
                            ..
                        } = frame;
                        commit_text_chunk(buffer, pending_ws, has_text, &text);
                    }
                }
                Ok(Event::GeneralRef(r)) => {
                    if let Some(decoded) = decode_general_ref(&r) {
                        if let Some(frame) = stack.last_mut() {
                            let ElementFrame {
                                buffer,
                                pending_ws,
                                has_text,
                                ..
                            } = frame;
                            commit_ref_chunk(buffer, pending_ws, has_text, &decoded);
                        }
                    }
                }
                Ok(Event::CData(t)) => {
                    // CDATA is explicit character data — never layout — so it
                    // commits verbatim under the same edge rule as text.
                    let raw = t.as_ref().to_string();
                    if let Some(frame) = stack.last_mut() {
                        if !raw.is_empty() {
                            let ws = std::mem::take(&mut frame.pending_ws);
                            frame.buffer.push_str(&ws);
                            frame.buffer.push_str(&raw);
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

        local
    };

    if opts.parallel {
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            let mut collected: Vec<TransUnit> = files.par_iter().flat_map(process_one).collect();
            collected.sort_by(|a, b| {
                (
                    a.path.to_string_lossy(),
                    a.line.unwrap_or(0),
                    a.key.as_str(),
                )
                    .cmp(&(
                        b.path.to_string_lossy(),
                        b.line.unwrap_or(0),
                        b.key.as_str(),
                    ))
            });
            return Ok(collected);
        }
        // If parallel requested but rayon feature not enabled, fall back to sequential
    }

    for p in &files {
        out.extend(process_one(p));
    }
    // Deterministic order
    out.sort_by(|a, b| {
        (
            a.path.to_string_lossy(),
            a.line.unwrap_or(0),
            a.key.as_str(),
        )
            .cmp(&(
                b.path.to_string_lossy(),
                b.line.unwrap_or(0),
                b.key.as_str(),
            ))
    });
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
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let p_str = p.to_string_lossy();
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            has_path_marker(&p_str, "Defs")
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
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        // filter to .../Defs/....xml (including versioned folders like 1.4/Defs, v1.6/Defs)
        let p_str = p.to_string_lossy();
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            has_path_marker(&p_str, "Defs")
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
                                        .find(|n| {
                                            n.is_element() && n.tag_name().name() == "defName"
                                        })
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
                        if let Some(val) = find_field_in_parents_across_files_simple(
                            &defs_index,
                            &name_index,
                            node.tag_name().name(),
                            parent_name,
                            field,
                        ) {
                            found_val = Some(val);
                            line = line_for_offset(node.range().start, &line_starts);
                        }
                    }
                }
                if let Some(val) = found_val {
                    out.push(TransUnit {
                        tkey: None,
                        key: format!("{}.{}", def_name, field),
                        source: Some(val),
                        path: p.to_path_buf(),
                        line,
                        ..Default::default()
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
    // TKey nodes are part of the complete translation-unit inventory; every
    // consumer of scan_all_units* (coverage, validate, wordinfo) must see them.
    if let Ok(mut tkey) = scan_defs_tkey(root, defs_root) {
        let mut seen: std::collections::HashSet<(String, String)> = units
            .iter()
            .map(|u| (u.path.to_string_lossy().to_string(), u.key.clone()))
            .collect();
        for u in tkey.drain(..) {
            if seen.insert((u.path.to_string_lossy().to_string(), u.key.clone())) {
                units.push(u);
            }
        }
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

/// Load a simple type schema and convert it to a Defs dictionary.
/// Supported JSON shapes:
/// - { "ThingDef": ["label", "description", ...], ... }
/// - { "ThingDef": { "fields": ["label", "description", ...] }, ... }
pub fn load_type_schema_as_dict(path: &Path) -> CoreResult<DefsDict> {
    let s = fs::read_to_string(path)?;
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Flat(Vec<String>),
        Obj { fields: Vec<String> },
    }
    let raw: std::collections::HashMap<String, Entry> = serde_json::from_str(&s)?;
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (k, v) in raw {
        let fields = match v {
            Entry::Flat(v) => v,
            Entry::Obj { fields } => fields,
        };
        out.entry(k).or_default().extend(fields);
    }
    Ok(DefsDict(out))
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
    // Allow alias segments like "label|labelShort" – take any that matches
    // Allow a minimal predicate syntax: name[attr=value] or name[@attr='value']
    #[derive(Clone)]
    struct Sel {
        name: String,
        attrs: Vec<(String, String)>,
        index: Option<usize>,
    }
    let parse_sel = |s: &str| -> Sel {
        if let Some(br) = s.find('[') {
            let name = &s[..br];
            let inside = &s[br + 1..s.len().saturating_sub(1)];
            // Support multiple predicates separated by '&': a=b&c=d
            // Index forms: "#2", "2", "index=2", "i=2"
            let mut attrs: Vec<(String, String)> = Vec::new();
            let mut index: Option<usize> = None;
            for token in inside
                .split('&')
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
            {
                let t = token.trim_matches(|c| c == '[' || c == ']');
                if let Some(num) = t.strip_prefix('#') {
                    if let Ok(i) = num.parse::<usize>() {
                        index = Some(i);
                        continue;
                    }
                }
                if t.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(i) = t.parse::<usize>() {
                        index = Some(i);
                        continue;
                    }
                }
                let mut parts = t.splitn(2, '=');
                let left = parts.next().unwrap_or("").trim().trim_start_matches('@');
                if left.eq_ignore_ascii_case("index") || left.eq_ignore_ascii_case("i") {
                    if let Some(v) = parts.next() {
                        if let Ok(i) = v.trim().parse::<usize>() {
                            index = Some(i);
                        }
                    }
                    continue;
                }
                if let Some(right) = parts.next() {
                    let val = right
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    attrs.push((left.to_string(), val));
                }
            }
            Sel {
                name: name.to_string(),
                attrs,
                index,
            }
        } else {
            Sel {
                name: s.to_string(),
                attrs: Vec::new(),
                index: None,
            }
        }
    };
    let aliases: Vec<Sel> = head.split('|').map(parse_sel).collect();
    let tail = &path[1..];
    if aliases.iter().any(|a| a.name.eq_ignore_ascii_case("li")) {
        for child in node
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("li"))
        {
            collect_values_by_path(child, tail, out);
        }
    } else {
        // If any alias specifies an index, handle indexed selection by name first.
        if let Some(sel) = aliases.iter().find(|a| a.index.is_some()).cloned() {
            // Collect candidates matching name and attrs
            let mut candidates: Vec<roxmltree::Node<'a, 'a>> = Vec::new();
            for child in node.children().filter(|c| c.is_element()) {
                let cname = child.tag_name().name();
                if !cname.eq_ignore_ascii_case(sel.name.as_str()) {
                    continue;
                }
                // Check attributes
                let mut ok = true;
                for (attr, val) in &sel.attrs {
                    if let Some(av) = child.attribute(attr.as_str()) {
                        if av != val {
                            ok = false;
                            break;
                        }
                    } else {
                        // Special: defName/Name child element
                        if attr.eq_ignore_ascii_case("defName") || attr.eq_ignore_ascii_case("Name")
                        {
                            if let Some(t) = child
                                .children()
                                .find(|n| {
                                    n.is_element()
                                        && (n.tag_name().name() == "defName"
                                            || n.tag_name().name() == "Name")
                                })
                                .and_then(|n| n.text())
                            {
                                if t != val {
                                    ok = false;
                                }
                            } else {
                                ok = false;
                            }
                        } else {
                            ok = false;
                        }
                        if !ok {
                            break;
                        }
                    }
                }
                if ok {
                    candidates.push(child);
                }
            }
            if let Some(i) = sel.index {
                if let Some(ch) = candidates.get(i) {
                    collect_values_by_path(*ch, tail, out);
                }
            } else {
                for ch in candidates {
                    collect_values_by_path(ch, tail, out);
                }
            }
            return;
        }
        'outer: for child in node.children().filter(|c| c.is_element()) {
            let cname = child.tag_name().name();
            for sel in &aliases {
                if !cname.eq_ignore_ascii_case(sel.name.as_str()) {
                    continue;
                }
                // Match all attrs (if any)
                let mut ok = true;
                for (attr, val) in &sel.attrs {
                    if let Some(av) = child.attribute(attr.as_str()) {
                        if av != val {
                            ok = false;
                            break;
                        }
                    } else {
                        if attr.eq_ignore_ascii_case("defName") || attr.eq_ignore_ascii_case("Name")
                        {
                            if let Some(t) = child
                                .children()
                                .find(|n| {
                                    n.is_element()
                                        && (n.tag_name().name() == "defName"
                                            || n.tag_name().name() == "Name")
                                })
                                .and_then(|n| n.text())
                            {
                                if t != val {
                                    ok = false;
                                }
                            } else {
                                ok = false;
                            }
                        } else {
                            ok = false;
                        }
                        if !ok {
                            break;
                        }
                    }
                }
                if ok {
                    collect_values_by_path(child, tail, out);
                    continue 'outer;
                } else {
                    continue;
                }
            }
        }
    }
}

// --------------------------
// Keyed file reader (map)
// --------------------------

/// Read a single LanguageData XML file (Keyed) into a map of key -> value, preserving
/// multi-line values, handling `<li>` aggregation and `<LineBreak/>` semantics.
pub fn read_keyed_file_map(path: &Path) -> CoreResult<BTreeMap<String, String>> {
    read_keyed_file_map_with_comments(path, None)
}

/// Same as `read_keyed_file_map` but when `comment_prefix` is provided, a preceding
/// XML comment matching that prefix (e.g., "EN:") right before a top-level key
/// is used as the value for that key instead of the element text.
pub fn read_keyed_file_map_with_comments(
    path: &Path,
    comment_prefix: Option<&str>,
) -> CoreResult<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    let content = fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    #[derive(Default)]
    struct Frame {
        name: String,
        buffer: String,
        has_text: bool,
        comment_override: Option<String>,
        /// Whitespace seen since the last committed chunk (see
        /// [`commit_text_chunk`]): quick-xml 0.42 splits character data around
        /// `&ref;` into separate events, so per-chunk trimming would eat the
        /// spaces around inline tags.
        pending_ws: String,
    }

    let mut stack: Vec<Frame> = Vec::new();
    let mut pending_comment: Option<String> = None;
    let prefix = comment_prefix.map(|s| s.to_string());

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_owned();
                let mut fr = Frame {
                    name,
                    buffer: String::new(),
                    has_text: false,
                    comment_override: None,
                    pending_ws: String::new(),
                };
                if stack.len() == 1 {
                    // Starting a top-level key under <LanguageData>
                    if let (Some(pref), Some(cmt)) = (prefix.as_deref(), pending_comment.take()) {
                        let trimmed = cmt.trim();
                        if let Some(rest) = trimmed.strip_prefix(pref) {
                            let val = rest.trim().to_string();
                            if !val.is_empty() {
                                fr.comment_override = Some(val);
                            }
                        }
                    }
                }
                stack.push(fr);
            }
            Ok(Event::Comment(c)) => {
                // Remember comments that appear directly under <LanguageData>
                if stack.len() == 1 {
                    let s = c.as_ref().to_owned();
                    pending_comment = Some(s);
                }
            }
            Ok(Event::End(_)) => {
                if let Some(mut frame) = stack.pop() {
                    // Commit a top-level key under LanguageData
                    if stack.len() == 1 && !frame.name.is_empty() {
                        let mut val = if let Some(v) = frame.comment_override.take() {
                            v
                        } else if frame.has_text {
                            frame.buffer.trim().to_string()
                        } else {
                            String::new()
                        };
                        map.insert(frame.name, std::mem::take(&mut val));
                        continue;
                    }
                    // Folding list items
                    if frame.name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                        if let Some(parent) = stack.last_mut() {
                            if parent.has_text && !parent.buffer.ends_with('\n') {
                                let trimmed_len = parent.buffer.trim_end().len();
                                parent.buffer.truncate(trimmed_len);
                                parent.buffer.push('\n');
                            }
                            let li_text = frame.buffer.trim().to_string();
                            if !li_text.is_empty() {
                                parent.buffer.push_str(&li_text);
                            }
                            parent.has_text = true;
                        }
                        continue;
                    }
                    if let Some(parent) = stack.last_mut() {
                        if !frame.buffer.is_empty() {
                            parent.buffer.push_str(&frame.buffer);
                            parent.has_text = parent.has_text || frame.has_text;
                        }
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name().as_ref().to_owned();
                if name.eq_ignore_ascii_case("LineBreak") {
                    if let Some(parent) = stack.last_mut() {
                        parent.buffer.push('\n');
                        parent.has_text = true;
                    }
                } else if stack.len() == 1 {
                    map.insert(name, String::new());
                } else if name.eq_ignore_ascii_case("li") && stack.len() == 2 {
                    if let Some(parent) = stack.last_mut() {
                        if parent.has_text && !parent.buffer.ends_with('\n') {
                            parent.buffer.push('\n');
                        }
                        parent.has_text = true;
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(frame) = stack.last_mut() {
                    let Frame {
                        buffer,
                        pending_ws,
                        has_text,
                        ..
                    } = frame;
                    let text = t.xml10_content().to_string();
                    commit_text_chunk(buffer, pending_ws, has_text, &text);
                }
            }
            Ok(Event::GeneralRef(r)) => {
                if let Some(decoded) = decode_general_ref(&r) {
                    if let Some(frame) = stack.last_mut() {
                        let Frame {
                            buffer,
                            pending_ws,
                            has_text,
                            ..
                        } = frame;
                        commit_ref_chunk(buffer, pending_ws, has_text, &decoded);
                    }
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(frame) = stack.last_mut() {
                    let text = Cow::Borrowed(t.as_ref());
                    if !text.trim().is_empty() {
                        frame.buffer.push_str(text.trim());
                        frame.has_text = true;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(e.into()),
        }
        buf.clear();
    }

    Ok(map)
}

fn inherit_enabled() -> bool {
    !matches!(std::env::var("RIMLOC_INHERIT"), Ok(val) if val.trim() == "0")
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
            .or_else(|| {
                index_defname
                    .get(def_type)
                    .and_then(|m| m.get(&name))
                    .cloned()
            })?;
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

/// Heuristic scan for additional string fields ("fuzzy" candidates).
/// Enabled by env RIMLOC_FUZZY=1; intended to supplement dict-based extraction.
pub fn scan_defs_fuzzy(root: &Path, defs_root: Option<&Path>) -> CoreResult<Vec<TransUnit>> {
    use walkdir::WalkDir;
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
    // Fields already covered by defaults
    const DEFAULTS: &[&str] = &[
        "label",
        "labelShort",
        "labelPlural",
        "description",
        "helpText",
        "reportString",
        "gerundLabel",
        "defName",
    ];
    let mut out = Vec::new();
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
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            let s = p.to_string_lossy();
            has_path_marker(&s, "Defs")
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
        let mut line_starts = vec![0usize];
        for (idx, _) in content.match_indices('\n') {
            line_starts.push(idx + 1);
        }
        for def_node in doc.root_element().children().filter(|n| n.is_element()) {
            let def_name = def_node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "defName")
                .and_then(|n| n.text())
                .map(str::trim)
                .unwrap_or("");
            if def_name.is_empty() {
                continue;
            }
            for child in def_node.children().filter(|c| c.is_element()) {
                let name = child.tag_name().name();
                if DEFAULTS.iter().any(|d| d.eq_ignore_ascii_case(name)) {
                    continue;
                }
                if let Some(t) = child.text().map(str::trim) {
                    if !t.is_empty() {
                        // Heuristic: consider "fuzzy" only if likely human text (>=2 words or contains space)
                        let human_like = t.split_whitespace().count() >= 2;
                        if human_like {
                            let line = line_for_offset(child.range().start, &line_starts);
                            out.push(TransUnit {
                                tkey: None,
                                key: format!("{}.{}", def_name, name),
                                source: Some(t.to_string()),
                                path: p.to_path_buf(),
                                line,
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(out)
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
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("xml"))
        {
            continue;
        }
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            let s = p.to_string_lossy();
            has_path_marker(&s, "Defs")
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

    // Optional parallel processing of Defs files (deterministic output order is preserved)
    {
        use walkdir::WalkDir;
        let mut def_files: Vec<std::path::PathBuf> = Vec::new();
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
            let in_scope = if let Some(base) = defs_root {
                p.starts_with(base)
            } else {
                let s = p.to_string_lossy();
                has_path_marker(&s, "Defs")
            };
            if !in_scope {
                continue;
            }
            def_files.push(p.to_path_buf());
        }

        let parallel = matches!(std::env::var("RIMLOC_PARALLEL"), Ok(v) if v.trim() == "1");
        #[cfg(feature = "rayon")]
        if parallel {
            use rayon::prelude::*;
            let process_one = |p: &std::path::PathBuf| -> Vec<DefsMetaUnit> {
                let mut out_local: Vec<DefsMetaUnit> = Vec::new();
                let content = match fs::read_to_string(p) {
                    Ok(s) => s,
                    Err(_) => return out_local,
                };
                let doc = match roxmltree::Document::parse(&content) {
                    Ok(d) => d,
                    Err(_) => return out_local,
                };
                let mut line_starts = Vec::new();
                line_starts.push(0usize);
                for (idx, _) in content.match_indices('\n') {
                    line_starts.push(idx + 1);
                }
                let root_el = doc.root_element();
                const DEFAULT_FIELDS: &[&str] = &[
                    "label",
                    "labelShort",
                    "labelPlural",
                    "description",
                    "helpText",
                    "reportString",
                    "gerundLabel",
                ];
                let mut all_fields: Vec<String> =
                    DEFAULT_FIELDS.iter().map(|s| s.to_string()).collect();
                for s in extra_fields {
                    if !s.trim().is_empty() {
                        all_fields.push(s.trim().to_string());
                    }
                }
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
                    if let Some(paths) = dict.get(&def_type) {
                        for path in paths {
                            let segs: Vec<&str> =
                                path.split('.').filter(|s| !s.is_empty()).collect();
                            let display_path = segs
                                .iter()
                                .map(|s| {
                                    if let Some(pos) = s.find('{') {
                                        &s[..pos]
                                    } else {
                                        s
                                    }
                                })
                                .collect::<Vec<&str>>()
                                .join(".");
                            let mut vals = Vec::new();
                            collect_values_by_path(def_node, &segs, &mut vals);
                            if vals.is_empty() && inherit_enabled() {
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
                                        let Some(parent_name) = current.attribute("ParentName")
                                        else {
                                            break;
                                        };
                                        if let Some(parent) = root_el.children().find(|c| {
                                            c.is_element()
                                                && c.tag_name()
                                                    .name()
                                                    .eq_ignore_ascii_case(&def_type)
                                                && c.attribute("Name")
                                                    .is_some_and(|n| n == parent_name)
                                        }) {
                                            collect_values_by_path(parent, &segs, &mut vals);
                                            current = parent;
                                        } else if let Some(parent) = root_el.children().find(|c| {
                                            c.is_element()
                                                && c.tag_name()
                                                    .name()
                                                    .eq_ignore_ascii_case(&def_type)
                                                && c.children()
                                                    .find(|n| {
                                                        n.is_element()
                                                            && n.tag_name().name() == "defName"
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
                                            let line =
                                                Some(match line_starts.binary_search(&line) {
                                                    Ok(idx) => idx + 1,
                                                    Err(idx) if idx > 0 => idx,
                                                    _ => 1,
                                                });
                                            out_local.push(DefsMetaUnit {
                                                unit: TransUnit {
                                                    tkey: None,
                                                    key: format!("{}.{}", def_name, display_path),
                                                    source: Some(v),
                                                    path: p.clone(),
                                                    line,
                                                    ..Default::default()
                                                },
                                                def_type: def_type.clone(),
                                                def_name: def_name.clone(),
                                                field_path: display_path.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                            for v in vals {
                                let line = def_node.range().start;
                                let line = Some(match line_starts.binary_search(&line) {
                                    Ok(idx) => idx + 1,
                                    Err(idx) if idx > 0 => idx,
                                    _ => 1,
                                });
                                out_local.push(DefsMetaUnit {
                                    unit: TransUnit {
                                        tkey: None,
                                        key: format!("{}.{}", def_name, display_path),
                                        source: Some(v.to_string()),
                                        path: p.clone(),
                                        line,
                                        ..Default::default()
                                    },
                                    def_type: def_type.clone(),
                                    def_name: def_name.clone(),
                                    field_path: display_path.clone(),
                                });
                            }
                        }
                    }
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
                                out_local.push(DefsMetaUnit {
                                    unit: TransUnit {
                                        tkey: None,
                                        key: format!("{}.{}", def_name, f),
                                        source: Some(val.to_string()),
                                        path: p.clone(),
                                        line,
                                        ..Default::default()
                                    },
                                    def_type: def_type.clone(),
                                    def_name: def_name.clone(),
                                    field_path: f.clone(),
                                });
                                produced = true;
                            }
                        }
                        #[allow(clippy::collapsible_if)]
                        if !produced && inherit_enabled() {
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
                                                && c.tag_name()
                                                    .name()
                                                    .eq_ignore_ascii_case(&def_type)
                                                && c.attribute("Name")
                                                    .is_some_and(|n| n == parent_name)
                                        })
                                        .or_else(|| {
                                            root_el.children().find(|c| {
                                                c.is_element()
                                                    && c.tag_name()
                                                        .name()
                                                        .eq_ignore_ascii_case(&def_type)
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
                                            c.is_element()
                                                && c.tag_name().name().eq_ignore_ascii_case(f)
                                        }) {
                                            if let Some(val) = n.text().map(str::trim) {
                                                let line = def_node.range().start;
                                                let line =
                                                    Some(match line_starts.binary_search(&line) {
                                                        Ok(idx) => idx + 1,
                                                        Err(idx) if idx > 0 => idx,
                                                        _ => 1,
                                                    });
                                                out_local.push(DefsMetaUnit {
                                                    unit: TransUnit {
                                                        tkey: None,
                                                        key: format!("{}.{}", def_name, f),
                                                        source: Some(val.to_string()),
                                                        path: p.clone(),
                                                        line,
                                                        ..Default::default()
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
                                if let Some(val) = vals2.into_iter().find(|v| !v.trim().is_empty())
                                {
                                    let line = def_node.range().start;
                                    let line = Some(match line_starts.binary_search(&line) {
                                        Ok(idx) => idx + 1,
                                        Err(idx) if idx > 0 => idx,
                                        _ => 1,
                                    });
                                    out_local.push(DefsMetaUnit {
                                        unit: TransUnit {
                                            tkey: None,
                                            key: format!("{}.{}", def_name, f),
                                            source: Some(val),
                                            path: p.clone(),
                                            line,
                                            ..Default::default()
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
                out_local
            };
            let mut merged: Vec<DefsMetaUnit> =
                def_files.par_iter().flat_map(process_one).collect();
            merged.sort_by(|a, b| {
                (
                    a.unit.path.to_string_lossy(),
                    a.unit.line.unwrap_or(0),
                    a.unit.key.as_str(),
                )
                    .cmp(&(
                        b.unit.path.to_string_lossy(),
                        b.unit.line.unwrap_or(0),
                        b.unit.key.as_str(),
                    ))
            });
            return Ok(merged);
        }
    }
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
        let in_scope = if let Some(base) = defs_root {
            p.starts_with(base)
        } else {
            let s = p.to_string_lossy();
            has_path_marker(&s, "Defs")
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
                        .map(|s| {
                            if let Some(pos) = s.find('{') {
                                &s[..pos]
                            } else {
                                s
                            }
                        })
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
                                            tkey: None,
                                            key: format!("{}.{path}", def_name),
                                            source: Some(v),
                                            path: p.to_path_buf(),
                                            line,
                                            ..Default::default()
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
                                tkey: None,
                                key: format!("{}.{}", def_name, display_path),
                                source: Some(v.to_string()),
                                path: p.to_path_buf(),
                                line,
                                ..Default::default()
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
                                tkey: None,
                                key: format!("{}.{}", def_name, f),
                                source: Some(val.to_string()),
                                path: p.to_path_buf(),
                                line,
                                ..Default::default()
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
                                                tkey: None,
                                                key: format!("{}.{}", def_name, f),
                                                source: Some(val.to_string()),
                                                path: p.to_path_buf(),
                                                line,
                                                ..Default::default()
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
                                    tkey: None,
                                    key: format!("{}.{}", def_name, f),
                                    source: Some(val),
                                    path: p.to_path_buf(),
                                    line,
                                    ..Default::default()
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

    /// Windows mixed-separator regression: `root.join("Languages/Russian")`
    /// spells the TM root as `D:\mod\Languages/Russian` there, and walkdir
    /// yields paths mixing `/` and `\` in ONE spelling. The collector's
    /// same-separator `contains("/Languages/") || contains("\\Languages\\")`
    /// matched NEITHER, silently collected zero files, and the export TM
    /// merge shipped an all-empty PO (windows CI: every DefInjected element
    /// empty). The unix simulation spells the mixed shape literally as a
    /// `Languages\\Russian` directory name; on Windows that same name is a
    /// nested native tree — the scan must collect on both.
    #[test]
    fn scan_keyed_xml_collects_mixed_separator_language_paths() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mod/Languages\\Russian/Keyed");
        fs::create_dir_all(&keyed_dir)?;
        fs::write(
            keyed_dir.join("A.xml"),
            "<LanguageData><Greeting>привет</Greeting></LanguageData>",
        )?;

        let units = scan_keyed_xml(dir.path())?;
        assert!(
            units
                .iter()
                .any(|u| u.key == "Greeting" && u.source.as_deref() == Some("привет")),
            "mixed-separator Languages path must be collected, got {:?}",
            units
        );
        Ok(())
    }

    /// CRLF insurance: a checkout with CRLF line endings (no .gitattributes,
    /// Windows autocrlf) must parse identically — values never empty.
    #[test]
    fn scan_keyed_xml_handles_crlf_files() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed_dir = dir.path().join("Mods/TestMod/Languages/Russian/Keyed");
        fs::create_dir_all(&keyed_dir)?;
        fs::write(
            keyed_dir.join("Crlf.xml"),
            "<LanguageData>\r\n  <Greeting>Привет\r\n  мир</Greeting>\r\n</LanguageData>\r\n",
        )?;

        let units = scan_keyed_xml(dir.path())?;
        assert!(
            units
                .iter()
                .any(|u| u.key == "Greeting" && u.source.as_deref() == Some("Привет\n  мир")),
            "CRLF value must survive: {:?}",
            units
        );
        Ok(())
    }

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

    #[test]
    fn scan_keyed_nested_definj_drops_def_type() -> CoreResult<()> {
        let dir = tempdir()?;
        let definj_dir = dir
            .path()
            .join("Mods/TestMod/Languages/English/DefInjected/ThingDef");
        fs::create_dir_all(&definj_dir)?;

        let file_path = definj_dir.join("Nested.xml");
        fs::write(
            &file_path,
            r#"<LanguageData>
  <ThingDef>
    <Meal_Simple>
      <label>simple meal</label>
    </Meal_Simple>
  </ThingDef>
  <PawnKindDef>
    <Pawn_PlayerColony>
      <label>colonist</label>
    </Pawn_PlayerColony>
  </PawnKindDef>
  <NotADef>
    <Foo>
      <label>bar</label>
    </Foo>
  </NotADef>
  <Already.Flat>baz</Already.Flat>
  </LanguageData>
"#,
        )?;

        let opts = KeyedScanOptions {
            nested: true,
            join_li_with_newline: true,
            include_empty_keys: true,
            parallel: false,
            definj_drop_def_type: true,
        };
        let units = scan_keyed_xml_with_options(dir.path(), &opts)?;

        // For DefInjected nested structure, first segment ending with 'Def' must be dropped
        assert!(units
            .iter()
            .any(|u| u.key == "Meal_Simple.label" && u.source.as_deref() == Some("simple meal")));
        // Different def type inside same file should keep its prefix
        assert!(units
            .iter()
            .any(|u| u.key == "PawnKindDef.Pawn_PlayerColony.label"
                && u.source.as_deref() == Some("colonist")));

        // Non-Def prefix should remain (NotADef doesn't end with 'Def' → keep it)
        assert!(units
            .iter()
            .any(|u| u.key == "NotADef.Foo.label" && u.source.as_deref() == Some("bar")));

        // Already flat dotted key preserved as-is
        assert!(units
            .iter()
            .any(|u| u.key == "Already.Flat" && u.source.as_deref() == Some("baz")));

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

        let meta = scan_defs_with_dict_meta(dir.path(), None, &load_embedded_defs_dict().0, &[])?;
        let units: Vec<TransUnit> = meta.into_iter().map(|m| m.unit).collect();
        assert!(units
            .iter()
            .any(|u| u.key == "Apparel_Parka.label" && u.source.as_deref() == Some("parka")));
        assert!(units.iter().any(|u| u.key == "Apparel_Parka.description"
            && u.source.as_deref() == Some("A warm parka for cold climates.")));
        Ok(())
    }

    #[test]
    fn scan_defs_extracts_pawnkind_lifestages_and_thing_verbs() -> CoreResult<()> {
        let dir = tempdir()?;
        let defs_dir = dir.path().join("Mods/TestMod/Defs/Mixed");
        fs::create_dir_all(&defs_dir)?;
        let file_path = defs_dir.join("Mixed.xml");
        fs::write(
            &file_path,
            r#"<Defs>
  <PawnKindDef>
    <defName>Pawn_PlayerColony</defName>
    <label>colonist</label>
    <lifeStages>
      <li>
        <label>juvenile colonist</label>
      </li>
      <li>
        <label>adult colonist</label>
      </li>
    </lifeStages>
  </PawnKindDef>
  <ThingDef>
    <defName>Meal_Simple</defName>
    <label>simple meal</label>
    <verbs>
      <li>
        <label>eat</label>
      </li>
    </verbs>
    <ingestible>
      <ingestCommandString>Eat {0}</ingestCommandString>
    </ingestible>
  </ThingDef>
</Defs>
"#,
        )?;

        let meta = scan_defs_with_dict_meta(dir.path(), None, &load_embedded_defs_dict().0, &[])?;
        let units: Vec<TransUnit> = meta.into_iter().map(|m| m.unit).collect();
        // Pawn lifeStages labels
        assert!(units
            .iter()
            .any(|u| u.key == "Pawn_PlayerColony.lifeStages.li.label"
                && u.source.as_deref() == Some("juvenile colonist")));
        assert!(units
            .iter()
            .any(|u| u.key == "Pawn_PlayerColony.lifeStages.li.label"
                && u.source.as_deref() == Some("adult colonist")));
        // Thing verbs.li.label
        assert!(units
            .iter()
            .any(|u| u.key == "Meal_Simple.verbs.li.label" && u.source.as_deref() == Some("eat")));
        // ingestible.ingestCommandString
        assert!(units
            .iter()
            .any(|u| u.key == "Meal_Simple.ingestible.ingestCommandString"
                && u.source.as_deref() == Some("Eat {0}")));
        Ok(())
    }

    #[test]
    fn scan_defs_extracts_worldobject_and_sitepart_labels() -> CoreResult<()> {
        let dir = tempdir()?;
        let defs_dir = dir.path().join("Mods/TestMod/Defs/Misc");
        fs::create_dir_all(&defs_dir)?;
        let file_path = defs_dir.join("WorldSite.xml");
        fs::write(
            &file_path,
            r#"<Defs>
  <WorldObjectDef>
    <defName>MyWorldObject</defName>
    <label>some world object</label>
    <description>appears on world map</description>
  </WorldObjectDef>
  <SitePartDef>
    <defName>BanditCamp</defName>
    <label>bandit camp</label>
    <description>hostile site</description>
  </SitePartDef>
</Defs>
"#,
        )?;
        let meta = scan_defs_with_dict_meta(dir.path(), None, &load_embedded_defs_dict().0, &[])?;
        let units: Vec<TransUnit> = meta.into_iter().map(|m| m.unit).collect();
        assert!(units
            .iter()
            .any(|u| u.key == "MyWorldObject.label"
                && u.source.as_deref() == Some("some world object")));
        assert!(units
            .iter()
            .any(|u| u.key == "BanditCamp.description"
                && u.source.as_deref() == Some("hostile site")));
        Ok(())
    }

    // NOTE: RecipeDef.ingredients.* are schema-sensitive and vary across mods;
    // covered indirectly via dict/DSL integration tests elsewhere.
}

/// Extract translatable nodes carrying an explicit `TKey` attribute
/// (RimWorld TKey system, present since 1.1 in 2020; QuestScriptDefs,
/// TipSetDefs, etc. Primary tested corpus: installed 1.6).
///
/// Verified against installed 1.6 data and the official Russian pack:
/// - `<li TKey="DismissLetters">…` in a TipSetDef with defName `GameplayTips`
///   translates as DefInjected path `GameplayTips.DismissLetters`;
/// - `<label TKey="LetterLabelFavorReceiver">` on QuestScriptDef `TradeRequest`
///   translates as `TradeRequest.LetterLabelFavorReceiver.slateRef` (the game's
///   canonical path adds a type-dependent suffix — normalized away when matching).
///
/// RimLoc emits the base identity `<defName>.<TKey>`; matching layers normalize
/// suffixes (`.slateRef`, `.value.slateRef`).
/// Serialization strategy of a TKey node, derived from its XML context.
///
/// Proven on the whole vanilla 1.6 corpus — the primary tested version —
/// (DLC-TKEY-ADJUDICATION §3,
/// 351 matched nodes / 343 identities, zero exceptions):
/// - `bare` — TipSetDef `li` nodes (DefInjected path has no suffix);
/// - `parms_value_slate_ref` — nodes under a `<parms>` element of a
///   `QuestNode_SubScript` def (dictionary slot elided from the path);
/// - `slate_ref` — any other direct SlateRef field of the def.
fn tkey_strategy(def_tag: &str, node: roxmltree::Node) -> (&'static str, &'static str) {
    if def_tag == "TipSetDef" && node.tag_name().name() == "li" {
        return ("bare", "");
    }
    let mut under_parms = false;
    let mut under_subscript = false;
    for anc in node.ancestors().skip(1) {
        if anc.tag_name().name() == "parms" {
            under_parms = true;
        }
        if anc
            .attribute("Class")
            .is_some_and(|c| c.starts_with("QuestNode_SubScript"))
        {
            under_subscript = true;
        }
    }
    if under_parms && under_subscript {
        ("parms_value_slate_ref", ".value.slateRef")
    } else {
        ("slate_ref", ".slateRef")
    }
}

pub fn scan_defs_tkey(root: &Path, defs_root: Option<&Path>) -> CoreResult<Vec<TransUnit>> {
    use std::collections::BTreeMap;
    use walkdir::WalkDir;

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

    // Logical identity -> accumulated unit. RimWorld load semantics:
    // a duplicate defName in a LATER file is rejected (first file owns the
    // identity), while a repeated field inside the SAME file is a plain
    // assignment overwrite (last node in document order wins). The context
    // count preserves how many nodes share the identity, and every accepted
    // node's real location is recorded (Source Inspector mandate §14:
    // Primary location + Other usages — captured in the SAME parser pass,
    // never a second scanner).
    struct Acc {
        unit: TransUnit,
        file: std::path::PathBuf,
        contexts: u32,
        /// Real line numbers of every accepted node, document order.
        usages: Vec<Option<usize>>,
        strategy: &'static str,
        suffix: &'static str,
        def_type: String,
    }
    let mut acc: BTreeMap<String, Acc> = BTreeMap::new();

    // An explicit defs_root is authoritative: walk IT directly so an external
    // directory outside `root` also works (P2-8). Without it, fall back to the
    // "/Defs/" path-shape heuristic under `root`.
    let walk_base: &Path = defs_root.unwrap_or(root);
    for entry in WalkDir::new(walk_base)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
        if defs_root.is_none() {
            let s = p.to_string_lossy();
            if !(has_path_marker(&s, "Defs")) {
                continue;
            }
        }
        let Ok(content) = fs::read_to_string(p) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&content) else {
            continue;
        };
        // Parser-guaranteed line numbers for every TKey node of this file.
        let line_starts: Vec<usize> = content
            .bytes()
            .enumerate()
            .filter(|(_, b)| *b == b'\n')
            .map(|(i, _)| i + 1)
            .collect();
        for node in doc.root_element().descendants().filter(|n| n.is_element()) {
            let Some(tkey) = node.attribute("TKey") else {
                continue;
            };
            if tkey.trim().is_empty() {
                continue;
            }
            // Owning def: nearest ancestor (incl. self) that has a <defName> child.
            let mut owner = Some(node);
            let mut def_name: Option<&str> = None;
            let mut def_tag = "";
            while let Some(cur) = owner {
                if let Some(dn) = cur
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name() == "defName")
                    .and_then(|n| n.text())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    def_name = Some(dn);
                    def_tag = cur.tag_name().name();
                    break;
                }
                owner = cur.parent().filter(|n| n.is_element());
            }
            let Some(def_name) = def_name else { continue };
            // The accumulation key includes the DEF TYPE: two def types may
            // share defName and TKey, and those are different identities
            // (the logical unit key stays `{defName}.{TKey}`).
            let acc_key = format!("{def_tag}\u{1}{def_name}.{}", tkey.trim());
            let identity = format!("{def_name}.{}", tkey.trim());
            let text = node.text().unwrap_or_default().trim().to_string();
            if text.is_empty() {
                continue;
            }
            let (strategy, suffix) = tkey_strategy(def_tag, node);
            let line = line_for_offset(node.range().start, &line_starts);
            match acc.get_mut(&acc_key) {
                // Identity already owned by an earlier file: that def wins,
                // later files are rejected duplicates — skip entirely. Their
                // locations are NOT usages (rejected Def ≠ same-file node).
                Some(a) if a.file != p => {}
                // Same file again: RimWorld field-assignment last-wins.
                Some(a) => {
                    a.unit.source = Some(text);
                    a.contexts += 1;
                    a.usages.push(line);
                    a.strategy = strategy;
                    a.suffix = suffix;
                }
                None => {
                    acc.insert(
                        acc_key,
                        Acc {
                            contexts: 1,
                            file: p.to_path_buf(),
                            usages: vec![line],
                            strategy,
                            suffix,
                            def_type: def_tag.to_string(),
                            unit: TransUnit {
                                key: identity,
                                source: Some(text),
                                path: p.to_path_buf(),
                                ..Default::default()
                            },
                        },
                    );
                }
            }
        }
    }
    Ok(acc
        .into_values()
        .map(|a| {
            let mut unit = a.unit;
            // Primary location = the effective (last) assignment; earlier
            // nodes are other usages. Single-node identities are plain
            // first-file winners; repeated same-file assignment has its own
            // reason.
            let last_line = a.usages.last().copied().flatten();
            unit.line = last_line;
            unit.src = Some(rimloc_core::SourceRef {
                file: a.file.clone(),
                line: last_line,
            });
            unit.selected_by = Some(
                if a.contexts > 1 {
                    rimloc_core::winner_reason::TKEY_LAST_ASSIGNMENT
                } else {
                    rimloc_core::winner_reason::DEFS_FIRST_FILE
                }
                .into(),
            );
            let locations: Vec<rimloc_core::SourceRef> = a
                .usages
                .iter()
                .map(|line| rimloc_core::SourceRef {
                    file: a.file.clone(),
                    line: *line,
                })
                .collect();
            unit.tkey = Some(rimloc_core::TKeyMeta {
                strategy: a.strategy.to_string(),
                suffix: a.suffix.to_string(),
                def_type: a.def_type,
                contexts: a.contexts,
                locations,
            });
            unit
        })
        .collect())
}

#[cfg(test)]
mod tkey_tests {
    use super::*;

    #[test]
    fn tkey_nodes_extract_with_defname_tkey_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let defs = tmp.path().join("Defs/Misc");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(
            defs.join("TKeySamples.xml"),
            r#"<Defs>
  <TipSetDef>
    <defName>SampleTips</defName>
    <tips><li TKey="DismissLetters">You can dismiss letters by right-clicking.</li></tips>
  </TipSetDef>
  <QuestScriptDef>
    <defName>SampleQuest</defName>
    <label TKey="LetterLabelFavorReceiver">sample favor label</label>
    <customLetterText TKey="LetterTextSample">Sample quest text.</customLetterText>
    <node Class="QuestNode_SubScript">
      <parms>
        <li Class="NamedParm">
          <key>letterText</key>
          <value TKey="LetterTextParms">parms-proven text</value>
        </li>
      </parms>
    </node>
  </QuestScriptDef>
</Defs>"#,
        )
        .unwrap();
        let units = scan_defs_tkey(tmp.path(), None).unwrap();
        let keys: Vec<&str> = units.iter().map(|u| u.key.as_str()).collect();
        assert!(keys.contains(&"SampleTips.DismissLetters"), "{keys:?}");
        assert!(
            keys.contains(&"SampleQuest.LetterLabelFavorReceiver"),
            "{keys:?}"
        );
        assert!(keys.contains(&"SampleQuest.LetterTextSample"), "{keys:?}");
        let tip = units
            .iter()
            .find(|u| u.key == "SampleTips.DismissLetters")
            .unwrap();
        assert!(tip
            .source
            .as_deref()
            .unwrap()
            .starts_with("You can dismiss"));
        let meta = |k: &str| {
            units
                .iter()
                .find(|u| u.key == k)
                .unwrap()
                .tkey
                .as_ref()
                .unwrap()
        };
        // Per-field asserts: each parsed identity now also carries its real
        // node location (mandate §14), which the expected literals below do
        // not pin — locations are asserted separately.
        fn fields(m: &rimloc_core::TKeyMeta) -> (&str, &str, &str, u32) {
            (
                m.strategy.as_str(),
                m.suffix.as_str(),
                m.def_type.as_str(),
                m.contexts,
            )
        }
        // TipSetDef li: bare, no suffix.
        assert_eq!(
            fields(meta("SampleTips.DismissLetters")),
            ("bare", "", "TipSetDef", 1)
        );
        // Direct QuestScriptDef field: slate_ref / .slateRef.
        assert_eq!(
            fields(meta("SampleQuest.LetterLabelFavorReceiver")),
            ("slate_ref", ".slateRef", "QuestScriptDef", 1)
        );
        // parms descendant of QuestNode_SubScript: parms_value_slate_ref.
        assert_eq!(
            fields(meta("SampleQuest.LetterTextParms")),
            (
                "parms_value_slate_ref",
                ".value.slateRef",
                "QuestScriptDef",
                1
            )
        );
        // Every identity carries exactly one real, parser-guaranteed location.
        for k in [
            "SampleTips.DismissLetters",
            "SampleQuest.LetterLabelFavorReceiver",
            "SampleQuest.LetterTextParms",
        ] {
            let m = meta(k);
            assert_eq!(m.locations.len(), 1, "{k}");
            assert!(
                m.locations[0]
                    .file
                    .to_string_lossy()
                    .ends_with("TKeySamples.xml"),
                "{k}: {:?}",
                m.locations[0].file
            );
            assert!(m.locations[0].line.is_some(), "{k}");
        }
    }

    #[test]
    fn tkey_duplicate_identity_last_wins_with_context_count() {
        let tmp = tempfile::tempdir().unwrap();
        let defs = tmp.path().join("Defs/Misc");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(
            defs.join("DupIdentity.xml"),
            r#"<Defs>
  <QuestScriptDef>
    <defName>IntroQuest</defName>
    <node Class="QuestNode_Letter">
      <label TKey="LetterLabelDied">first branch: died</label>
    </node>
    <node Class="QuestNode_Letter">
      <label TKey="LetterLabelDied">second branch: left behind</label>
    </node>
  </QuestScriptDef>
</Defs>"#,
        )
        .unwrap();
        let units = scan_defs_tkey(tmp.path(), None).unwrap();
        assert_eq!(units.len(), 1, "{units:?}");
        let u = &units[0];
        assert_eq!(u.key, "IntroQuest.LetterLabelDied");
        // RimWorld field-assignment semantics: the LAST node wins at load.
        assert_eq!(u.source.as_deref(), Some("second branch: left behind"));
        let meta = u.tkey.as_ref().unwrap();
        assert_eq!(meta.contexts, 2);
        assert_eq!(meta.strategy, "slate_ref");
    }

    #[test]
    fn tkey_duplicate_defname_later_file_is_rejected_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let defs = tmp.path().join("Defs/Misc");
        std::fs::create_dir_all(&defs).unwrap();
        for (name, text) in [
            ("A_First.xml", "from first file"),
            ("B_Second.xml", "from second"),
        ] {
            std::fs::write(
                defs.join(name),
                format!(
                    r#"<Defs>
  <QuestScriptDef>
    <defName>SharedDef</defName>
    <label TKey="SharedKey">{text}</label>
  </QuestScriptDef>
</Defs>"#
                ),
            )
            .unwrap();
        }
        let units = scan_defs_tkey(tmp.path(), None).unwrap();
        assert_eq!(units.len(), 1, "{units:?}");
        assert_eq!(units[0].source.as_deref(), Some("from first file"));
        assert_eq!(units[0].tkey.as_ref().unwrap().contexts, 1);
    }
}

#[cfg(test)]
mod tkey_defs_dir_tests {
    use super::*;

    #[test]
    fn tkey_scan_accepts_external_defs_dir_outside_root() {
        // P2-8: an explicit --defs-dir outside the scan root must work.
        let mod_dir = tempfile::tempdir().unwrap();
        let ext_defs = tempfile::tempdir().unwrap();
        let defs = ext_defs.path().join("Defs");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(
            defs.join("Ext.xml"),
            r#"<Defs>
  <QuestScriptDef>
    <defName>ExtQuest</defName>
    <label TKey="ExtKey">external defs text</label>
  </QuestScriptDef>
</Defs>"#,
        )
        .unwrap();
        // Root contains NO Defs at all.
        std::fs::create_dir_all(mod_dir.path().join("About")).unwrap();
        let units = scan_defs_tkey(mod_dir.path(), Some(defs.as_path())).unwrap();
        assert_eq!(units.len(), 1, "{units:?}");
        assert_eq!(units[0].key, "ExtQuest.ExtKey");
        assert_eq!(units[0].tkey.as_ref().unwrap().suffix, ".slateRef");
    }
}

#[cfg(test)]
mod entity_escape_tests {
    //! MUST_FIX_BEFORE_BETA: entity-разметка Keyed-значений.
    //!
    //! quick-xml 0.42 выносит `&lt;`/`&amp;`/`&#..;` из текста в отдельные
    //! события `Event::GeneralRef`. Сканер молча выбрасывал их (`_ => {}`) и
    //! тримел каждый текстовый кусок по отдельности, поэтому
    //! `&lt;b&gt;The HugsLib mod&lt;/b&gt;` выгружалось как `bThe HugsLib
    //! mod/b`. Эти тесты держат контракт: в TransUnit попадает ТЕКСТ ЗНАЧЕНИЯ,
    //! как его видит переводчик в игре — с сохранённой разметкой и пробелами.

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_keyed(root: &Path, body: &str) {
        let keyed = root.join("Mods/TestMod/Languages/English/Keyed");
        fs::create_dir_all(&keyed).unwrap();
        fs::write(keyed.join("Entities.xml"), body).unwrap();
    }

    fn find<'a>(units: &'a [TransUnit], key: &str) -> &'a str {
        units
            .iter()
            .find(|u| u.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "key {key} not found in {:?}",
                    units.iter().map(|u| u.key.as_str()).collect::<Vec<_>>()
                )
            })
            .source
            .as_deref()
            .unwrap()
    }

    #[test]
    fn scan_keyed_preserves_entity_markup_like_hugslib() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            r#"<LanguageData>
    <HugsLib_loadOrderWarning_text>&lt;b&gt;The HugsLib mod&lt;/b&gt; should always be loaded after &lt;b&gt;Core&lt;/b&gt; to avoid issues.\nPlease adjust your mod order in the Mods menu and restart the game.</HugsLib_loadOrderWarning_text>
</LanguageData>
"#,
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(
            find(&units, "HugsLib_loadOrderWarning_text"),
            "<b>The HugsLib mod</b> should always be loaded after <b>Core</b> to avoid issues.\\nPlease adjust your mod order in the Mods menu and restart the game.",
            "msgid source must carry parsed markup and surrounding spaces, not `b.../b`"
        );
        Ok(())
    }

    #[test]
    fn scan_keyed_resolves_named_and_numeric_entities() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            r#"<LanguageData>
    <Amp>a &amp; b</Amp>
    <Quotes>&quot;q&quot; and &apos;s&apos;</Quotes>
    <Numeric>&#65;&#x42;C</Numeric>
    <GtCompare>1 &lt; 2 &gt; 0</GtCompare>
</LanguageData>
"#,
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Amp"), "a & b");
        assert_eq!(find(&units, "Quotes"), "\"q\" and 's'");
        assert_eq!(find(&units, "Numeric"), "ABC");
        assert_eq!(find(&units, "GtCompare"), "1 < 2 > 0");
        Ok(())
    }

    #[test]
    fn scan_keyed_keeps_interior_spaces_but_drops_indentation() -> CoreResult<()> {
        // Pretty-printed value: leading/trailing indentation is formatting,
        // spaces around inline entities are content.
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            r#"<LanguageData>
    <Spaced>
        head &lt;b&gt;mid&lt;/b&gt; tail
    </Spaced>
</LanguageData>
"#,
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Spaced"), "head <b>mid</b> tail");
        Ok(())
    }

    #[test]
    fn scan_keyed_li_fold_keeps_entities_and_newlines() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            r#"<LanguageData>
    <Listed>Intro <li>&lt;i&gt;first&lt;/i&gt;</li><li>second</li></Listed>
</LanguageData>
"#,
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Listed"), "Intro\n<i>first</i>\nsecond");
        Ok(())
    }

    #[test]
    fn read_keyed_file_map_resolves_entities() -> CoreResult<()> {
        let dir = tempdir()?;
        let keyed = dir.path().join("Keyed");
        fs::create_dir_all(&keyed)?;
        let file = keyed.join("M.xml");
        fs::write(
            &file,
            r#"<LanguageData>
    <K1>&lt;b&gt;value&lt;/b&gt; tail</K1>
    <K2>a &amp; b</K2>
</LanguageData>
"#,
        )?;

        let map = read_keyed_file_map(&file)?;
        assert_eq!(map.get("K1").map(String::as_str), Some("<b>value</b> tail"));
        assert_eq!(map.get("K2").map(String::as_str), Some("a & b"));
        Ok(())
    }
}

#[cfg(test)]
mod edge_trim_tests {
    //! I1 (RELEASE_PARITY_MATRIX): край-тримминг Keyed-значений.
    //!
    //! Правило (game-семантика): RimWorld читает значение как есть, парсер
    //! возвращает точный текст между тегами минус ОТСТУПЫ ФОРМАТИРОВАНИЯ.
    //! Форматирование — пробельный рун, несущий перенос строки (выравнивание
    //! отступами); краевой рун однострочного контента — контент: хвостовой
    //! пробел `Search: ` значим при конкатенации в игре. Реальные inline-
    //! элементы (`<b>…</b>`) раньше терялись ЦЕЛИКОМ (пустое значение) —
    //! теперь сериализуются обратно.

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_keyed(root: &Path, body: &str) {
        let keyed = root.join("Mods/TestMod/Languages/English/Keyed");
        fs::create_dir_all(&keyed).unwrap();
        fs::write(keyed.join("Edges.xml"), body).unwrap();
    }

    fn find<'a>(units: &'a [TransUnit], key: &str) -> &'a str {
        units
            .iter()
            .find(|u| u.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "key {key} not found in {:?}",
                    units.iter().map(|u| u.key.as_str()).collect::<Vec<_>>()
                )
            })
            .source
            .as_deref()
            .unwrap()
    }

    /// Однострочный элемент: оба края — контент, сохраняются дословно.
    #[test]
    fn scan_keyed_single_line_keeps_edge_spaces() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData>\n    <Label> Search: </Label>\n</LanguageData>\n",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Label"), " Search: ");
        Ok(())
    }

    /// Многострочный элемент: отступы сняты, внутренние края строк сохранены,
    /// включая контентный пробел перед закрывающим отступом.
    #[test]
    fn scan_keyed_multiline_drops_indent_but_keeps_line_content() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData>\n    <Spaced>\n        Search: \n    </Spaced>\n</LanguageData>\n",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Spaced"), "Search: ");
        Ok(())
    }

    /// A/B на реальной строке HugsLib (`HugsLib_loadOrderWarning_text`,
    /// апстрим UnlimitedHugs/RimworldHugsLib, English/Keyed/English.xml:78-79)
    /// плюс sibling класса «Search: » из w5-remis SEMANTIC (хвостовой пробел
    /// при конкатенации). До фикса: `…issues.\n…` сохранялся, но любой
    /// хвостовой пробел съедался (`Search: ` → `Search:`); после — край
    /// сохранён и разметка, и хвостовой пробел.
    #[test]
    fn scan_keyed_hugslib_loadorderwarning_keeps_markup_and_edges() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData>\n\t<HugsLib_loadOrderWarning_title>Improper mod load order</HugsLib_loadOrderWarning_title>\n\t<HugsLib_loadOrderWarning_text>&lt;b&gt;The HugsLib mod&lt;/b&gt; should always be loaded after &lt;b&gt;Core&lt;/b&gt; to avoid issues.\\nPlease adjust your mod order in the Mods menu and restart the game.</HugsLib_loadOrderWarning_text>\n\t<HugsLib_search_label>Search: </HugsLib_search_label>\n</LanguageData>\n",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(
            find(&units, "HugsLib_loadOrderWarning_text"),
            "<b>The HugsLib mod</b> should always be loaded after <b>Core</b> to avoid issues.\\nPlease adjust your mod order in the Mods menu and restart the game."
        );
        assert_eq!(find(&units, "HugsLib_search_label"), "Search: ");
        Ok(())
    }

    /// Реальный inline-элемент: раньше его контент исчезал весь (значение
    /// пустое) — теперь сериализуется дословно.
    #[test]
    fn scan_keyed_real_inline_elements_round_trip() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData><Label><b>Search:</b></Label></LanguageData>",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Label"), "<b>Search:</b>");
        Ok(())
    }

    /// Inline-элемент с краевыми пробелами и атрибутами: края однострочника
    /// контент, тег с атрибутами восстанавливается как написан.
    #[test]
    fn scan_keyed_inline_element_keeps_edges_and_attrs() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData><Label>before <color r=\"1\" g=\"0\">mid</color> after </Label></LanguageData>",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(
            find(&units, "Label"),
            "before <color r=\"1\" g=\"0\">mid</color> after "
        );
        Ok(())
    }

    /// Самозакрытый inline-элемент внутри значения.
    #[test]
    fn scan_keyed_self_closing_inline_element_round_trips() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData><Label>a<br/>b</Label></LanguageData>",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Label"), "a<br/>b");
        Ok(())
    }

    /// CDATA — явные символьные данные, краевой трим к ним не применяется.
    #[test]
    fn scan_keyed_cdata_keeps_edges() -> CoreResult<()> {
        let dir = tempdir()?;
        write_keyed(
            dir.path(),
            "<LanguageData><Label><![CDATA[ Search: ]]></Label></LanguageData>",
        );

        let units = scan_keyed_xml(dir.path())?;
        assert_eq!(find(&units, "Label"), " Search: ");
        Ok(())
    }
}
