use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

/// Strip common ANSI escape sequences (CSI/OSC) from a string.  
/// Keeps everything else verbatim and is tolerant to malformed sequences.
pub fn strip_ansi<'a>(s: &'a str) -> Cow<'a, str> {
    if !has_ansi(s) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Handle CSI sequences: ESC '[' params intermediates final
        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2; // skip ESC '['
                    // parameters
            while i < bytes.len() && (b'0'..=b'?').contains(&bytes[i]) {
                i += 1;
            }
            // intermediates
            while i < bytes.len() && (b' '..=b'/').contains(&bytes[i]) {
                i += 1;
            }
            // final byte ends the CSI
            if i < bytes.len() && (b'@'..=b'~').contains(&bytes[i]) {
                i += 1; // consume final and drop the whole sequence
                continue;
            } else {
                // Not a valid CSI; emit literally the skipped ESC '[' and continue.
                out.push('\x1b');
                out.push('[');
                continue;
            }
        } else if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b']' {
            // Handle OSC sequences: ESC ] ... BEL or ESC ] ... ESC \
            let osc_start = i;
            i += 2; // skip ESC ]
            let mut found_terminator = false;
            while i < bytes.len() {
                if bytes[i] == 0x07 {
                    // BEL terminator
                    i += 1;
                    found_terminator = true;
                    break;
                } else if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                    // ST terminator (ESC \)
                    i += 2;
                    found_terminator = true;
                    break;
                } else {
                    i += 1;
                }
            }
            if found_terminator {
                // sequence dropped
                continue;
            } else {
                // Unterminated OSC, emit literally the skipped ESC ] and the following bytes until end
                // (emit ESC then ] then all bytes from osc_start+2 up to i)
                out.push('\x1b');
                out.push(']');
                // emit the rest literally (from osc_start+2 up to bytes.len())
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
    Cow::Owned(out)
}

fn has_ansi(s: &str) -> bool {
    s.bytes().any(|b| b == 0x1B)
}

/// Прочитать одно сообщение по ключу из crates/rimloc-cli/i18n/{locale}/rimloc.ftl.
/// Fallback на en выполняется в вызывающем коде.
pub fn read_ftl_message(i18n_dir: &Path, locale: &str, key: &str) -> Option<String> {
    let path = i18n_dir.join(locale).join("rimloc.ftl");
    let content = std::fs::read_to_string(&path).ok()?;

    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // пустая строка — завершение предыдущего значения
        if line.is_empty() {
            if let Some(k) = current_key.take() {
                if k == key {
                    return Some(current_val.trim_end().to_string());
                }
                current_val.clear();
            }
            continue;
        }

        // комменты игнорируем
        if line.starts_with('#') {
            continue;
        }

        // новая запись
        if let Some(eq) = line.find('=') {
            if let Some(k) = current_key.take() {
                if k == key {
                    return Some(current_val.trim_end().to_string());
                }
                current_val.clear();
            }
            let k = line[..eq].trim().to_string();
            let v = line[eq + 1..].trim_start();
            current_key = Some(k);
            if !v.is_empty() {
                current_val.push_str(v);
            }
        } else if current_key.is_some() {
            // продолжение многострочного значения — сохраняем как есть
            current_val.push('\n');
            current_val.push_str(raw_line);
        }
    }

    if let Some(k) = current_key {
        if k == key {
            return Some(current_val.trim_end().to_string());
        }
    }
    None
}

/// Полностью распарсить rimloc.ftl в карту key -> value (с базовой поддержкой многострочности)
pub fn get_map(i18n_dir: &Path, locale: &str) -> BTreeMap<String, String> {
    let path = i18n_dir.join(locale).join("rimloc.ftl");
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    let mut map = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() {
            if let Some(k) = current_key.take() {
                map.insert(k, current_val.trim_end().to_string());
                current_val.clear();
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            if let Some(k) = current_key.take() {
                map.insert(k, current_val.trim_end().to_string());
                current_val.clear();
            }
            let k = line[..eq].trim().to_string();
            let v = line[eq + 1..].trim_start();
            current_key = Some(k);
            if !v.is_empty() {
                current_val.push_str(v);
            }
        } else if current_key.is_some() {
            current_val.push('\n');
            current_val.push_str(raw_line);
        }
    }

    if let Some(k) = current_key.take() {
        map.insert(k, current_val.trim_end().to_string());
    }
    map
}

/// Проверка «строка содержит подстроку» с контекстом.
pub fn assert_contains_with_context(haystack: &str, needle: &str, context_msg: &str) {
    if haystack.contains(needle) {
        return;
    }
    let head = haystack.lines().take(10).collect::<Vec<_>>().join("\n");
    let mut sample = String::new();
    for c in haystack.chars().take(200) {
        sample.push(c);
    }
    if haystack.chars().count() > 200 {
        sample.push('…');
    }
    panic!(
        "{}\n--- needle ---\n{}\n--- head(10) ---\n{}\n--- sample(200 chars) ---\n{}",
        context_msg, needle, head, sample
    );
}

/// Проверка отсутствия ANSI-последовательностей (для --no-color).
pub fn assert_no_ansi(s: &str, context_msg: &str) {
    if has_ansi(s) {
        let mut esc_positions = Vec::new();
        for (i, b) in s.as_bytes().iter().enumerate() {
            if *b == 0x1B {
                esc_positions.push(i);
                if esc_positions.len() >= 8 {
                    break;
                }
            }
        }
        let sample = s.lines().take(8).collect::<Vec<_>>().join("\n");
        panic!(
            "{}\nANSI escapes detected at byte positions {:?}\n--- sample (first 8 lines) ---\n{}",
            context_msg, esc_positions, sample
        );
    }
}

pub fn run_cli(args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_rimloc-cli");
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("failed to spawn rimloc-cli");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Ожидать, что запуск с `args` в локали `lang` выведет ключ `key` из FTL (fallback на EN).
pub fn expect_ftl_contains_lang(args: &[&str], lang: &str, key: &str) {
    use std::path::PathBuf;
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let expected = read_ftl_message(&i18n_dir, lang, key)
        .or_else(|| read_ftl_message(&i18n_dir, "en", key))
        .unwrap_or_else(|| format!("{{missing-ftl:{}/{}}}", lang, key));

    let (code, stdout, stderr) = run_cli(args);
    let output = format!("{}{}", stdout, stderr);
    assert!(
        code == 0,
        "command failed with code {}.\nstdout:\n{}\nstderr:\n{}",
        code,
        stdout,
        stderr
    );
    assert_contains_with_context(
        &output,
        &expected,
        &format!("missing FTL key `{}` for lang `{}`", key, lang),
    );
}
