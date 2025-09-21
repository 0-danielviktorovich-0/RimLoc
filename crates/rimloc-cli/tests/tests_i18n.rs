// -----------------------------------------------------------------------------
// Test-only i18n loader: reads crates/rimloc-cli/i18n/{loc}/rimloc-tests.ftl
// Minimal parser: "key = value" lines, "#" comments. Supports placeholders
// of the form "{name}" and "{ $name }".
// Locale is chosen via env var RIMLOC_TESTS_LANG (default: "en").
// -----------------------------------------------------------------------------

#![allow(dead_code)]

use once_cell::sync::Lazy;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

// Detects any ESC byte; used only to fast-path `strip_ansi`.
fn has_ansi(s: &str) -> bool {
    s.bytes().any(|b| b == 0x1B)
}

// Strips CSI and OSC sequences (tolerant to malformed inputs) to make test matching robust.
fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    if !has_ansi(s) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // CSI: ESC '[' params intermediates final
        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2; // skip ESC '['
            while i < bytes.len() && (b'0'..=b'?').contains(&bytes[i]) {
                i += 1;
            }
            while i < bytes.len() && (b' '..=b'/').contains(&bytes[i]) {
                i += 1;
            }
            if i < bytes.len() && (b'@'..=b'~').contains(&bytes[i]) {
                i += 1; // consume final and drop sequence
                continue;
            } else {
                // invalid CSI; emit literally
                out.push('\x1b');
                out.push('[');
                continue;
            }
        } else if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b']' {
            // OSC: ESC ] ... BEL or ESC ] ... ESC \\ (ST)
            let osc_start = i;
            i += 2; // skip ESC ]
            let mut found_terminator = false;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    i += 1; // BEL
                    found_terminator = true;
                    break;
                } else if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                    i += 2; // ST
                    found_terminator = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if found_terminator {
                continue; // drop sequence
            } else {
                // Unterminated OSC — emit literally from ESC ] to end
                out.push('\x1b');
                out.push(']');
                let mut j = osc_start + 2;
                while j < bytes.len() {
                    out.push(bytes[j] as char);
                    j += 1;
                }
                break;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    std::borrow::Cow::Owned(out)
}

// Компиляционный бэкап английского rimloc-tests.ftl на случай отсутствия файлов во время рантайма.
const EMBEDDED_EN_TESTS_FTL: &str = include_str!("../i18n/en/rimloc-tests.ftl");

fn tests_locale() -> String {
    std::env::var("RIMLOC_TESTS_LANG").unwrap_or_else(|_| "en".to_string())
}

// Minimal FTL-ish parser: supports "key = value" and multiline values; comments start with '#'.
// Keeps indentation for continuation lines so tests can assert exact blocks if needed.
fn parse_ftl_to_map(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // blank line ends a current value (if any)
        if line.is_empty() {
            if let Some(k) = current_key.take() {
                map.insert(k, current_val.trim_end().to_string());
                current_val.clear();
            }
            continue;
        }

        // comments are ignored (do not terminate a multiline value)
        if line.starts_with('#') {
            continue;
        }

        // A new key-value line starts a new record. Close the previous one.
        if let Some(eq) = line.find('=') {
            if let Some(k) = current_key.take() {
                map.insert(k, current_val.trim_end().to_string());
                current_val.clear();
            }

            let key = line[..eq].trim().to_string();
            let val_part = line[eq + 1..].trim_start(); // keep trailing spaces significant only on continuations
            current_key = Some(key);
            if !val_part.is_empty() {
                current_val.push_str(val_part);
            }
        } else if current_key.is_some() {
            // Continuation line for the current key: preserve raw indentation/content.
            current_val.push('\n');
            current_val.push_str(raw_line);
        }
    }

    // Flush the last pending entry
    if let Some(k) = current_key.take() {
        map.insert(k, current_val.trim_end().to_string());
    }

    map
}

fn load_tests_ftl_map(locale: &str) -> BTreeMap<String, String> {
    // Базовая директория i18n для текущего крейта (CARGO_MANIFEST_DIR указывает на crates/rimloc-cli)
    let i18n_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");

    // 1) пробуем нужную локаль с диска
    let ftl_path = i18n_dir.join(locale).join("rimloc-tests.ftl");
    if let Ok(content) = fs::read_to_string(&ftl_path) {
        return parse_ftl_to_map(&content);
    }

    // 2) fallback на en с диска
    let en_path = i18n_dir.join("en").join("rimloc-tests.ftl");
    if let Ok(content) = fs::read_to_string(&en_path) {
        return parse_ftl_to_map(&content);
    }

    // 3) последний fallback — встроенный EN
    parse_ftl_to_map(EMBEDDED_EN_TESTS_FTL)
}

static TESTS_FTL: Lazy<BTreeMap<String, String>> =
    Lazy::new(|| load_tests_ftl_map(&tests_locale()));

/// Very small placeholder expander: supports {name}, {$name}, { $name }, and { $name}.
fn apply_vars(mut template: String, args: &[(&str, String)]) -> String {
    for (name, value) in args {
        // Replace {name}
        let needle1 = format!("{{{}}}", name);
        template = template.replace(&needle1, value);
        // Replace { $name } with optional spaces
        let needle2 = format!("{{ ${} }}", name);
        template = template.replace(&needle2, value);
        // Replace {$name}
        let needle3 = format!("{{${}}}", name);
        template = template.replace(&needle3, value);
        // Replace { $name} (no trailing space before })
        let needle4 = format!("{{ ${}}}", name);
        template = template.replace(&needle4, value);
    }
    template
}

/// Lookup a test i18n string by key, optionally applying simple variables.
/// This function is `pub` so other integration tests can `mod tests_i18n;` and call it.
pub fn lookup(key: &str, args: &[(&str, String)]) -> String {
    let raw = TESTS_FTL
        .get(key)
        .cloned()
        .unwrap_or_else(|| format!("{{missing-test-i18n:{}}}", key));
    apply_vars(raw, args)
}

/// Which stream to check when asserting output
#[derive(Copy, Clone, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// Execute the built CLI with given args and return (status_code, stdout, stderr).
/// Uses the binary path provided by Cargo: CARGO_BIN_EXE_rimloc-cli
pub fn run_cli(args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_rimloc-cli");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to spawn rimloc-cli");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Build an expected string from tests FTL by key and optional vars supplied as (&str, &str)
pub fn expected_from_ftl(key: &str, vars: &[(&str, &str)]) -> String {
    let vars_owned: Vec<(&str, String)> =
        vars.iter().map(|(k, v)| (*k, (*v).to_string())).collect();
    lookup(key, &vars_owned)
}

/// Convenience: run CLI, pick a stream, and assert it contains the FTL translation by key.
/// Returns the captured stream to allow further ad-hoc assertions by the caller.
pub fn expect_ftl_contains(
    args: &[&str],
    key: &str,
    vars: &[(&str, &str)],
    stream: Stream,
) -> String {
    let (_code, stdout, stderr) = run_cli(args);
    let expected = expected_from_ftl(key, vars);

    let stdout_plain = strip_ansi(&stdout);
    let stderr_plain = strip_ansi(&stderr);

    let hay_plain = match stream {
        Stream::Stdout => stdout_plain.as_ref(),
        Stream::Stderr => stderr_plain.as_ref(),
    };

    assert!(
        hay_plain.contains(&expected),
        "expected to find translation for '{}' in {} (ANSI stripped)\n\
         expected substring: {:?}\n\
         --- plain {} ---\n{}\n\
         --- raw stdout ---\n{}\n\
         --- raw stderr ---\n{}",
        key,
        stream_name(&stream),
        expected,
        stream_name(&stream),
        hay_plain,
        stdout,
        stderr
    );

    // Return the original (raw) selected stream to allow further ad-hoc assertions if needed.
    match stream {
        Stream::Stdout => stdout,
        Stream::Stderr => stderr,
    }
}

fn stream_name(s: &Stream) -> &'static str {
    match s {
        Stream::Stdout => "stdout",
        Stream::Stderr => "stderr",
    }
}

/// Shortcut for the common case: check stdout contains FTL translation
pub fn expect_stdout_ftl_contains(args: &[&str], key: &str, vars: &[(&str, &str)]) -> String {
    expect_ftl_contains(args, key, vars, Stream::Stdout)
}

/// Shortcut for the error-flow case: check stderr contains FTL translation
pub fn expect_stderr_ftl_contains(args: &[&str], key: &str, vars: &[(&str, &str)]) -> String {
    expect_ftl_contains(args, key, vars, Stream::Stderr)
}
