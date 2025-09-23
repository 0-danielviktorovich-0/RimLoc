use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;


pub const CTX_NONE: &str = "n/a";

#[macro_export]
macro_rules! fail_with_context {
    ($ctx_locale:expr, $ctx_file:expr, $ctx_key:expr, $msg:expr, $needle:expr, $haystack:expr $(,)?) => {{
        let head = $haystack.lines().take(10).collect::<Vec<_>>().join("\n");
        let sample = if std::env::var("RIMLOC_TEST_FULL_LOG").ok().as_deref() == Some("1") {
            $haystack.to_string()
        } else {
            let mut s = String::new();
            for c in $haystack.chars().take(200) { s.push(c); }
            if $haystack.chars().count() > 200 { s.push('…'); }
            s
        };
        panic!(
            "[{}] {:?} :: key = {}\n{}\n--- needle ---\n{}\n--- head(10) ---\n{}\n--- sample ---\n{}",
            $ctx_locale,
            $ctx_file,
            $ctx_key,
            $msg,
            $needle,
            head,
            sample
        );
    }}
}

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
    let mut content = std::fs::read_to_string(&path).ok()?;

    // убираем BOM, если есть
    if content.starts_with('\u{FEFF}') {
        content = content.trim_start_matches('\u{FEFF}').to_string();
    }

    let mut current_key: Option<String> = None;
    let mut current_val = String::new();

    for raw_line in content.lines() {
        // также убираем BOM на всякий случай в первой строке
        let line = raw_line.trim_start_matches('\u{FEFF}').trim();

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
pub fn assert_contains_with_context(
    haystack: &str,
    needle: &str,
    ctx_expected: &str,
    ctx_locale: &str,
    path: &Path,
    ctx_key: &str,
) {
    if !haystack.contains(needle) {
        let mut msg = String::new();

        msg.push_str(&format!(
            "[{locale}] \"{path}\" :: key = {ctx_key}\n\
             {{missing-test-i18n:{ctx_expected}}}\n",
            locale = ctx_locale,
            path = path.display(),
            ctx_key = ctx_key,
            ctx_expected = ctx_expected,
        ));

        // Новый блок: список кандидатов
        msg.push_str("--- candidates ---\n");
        msg.push_str(&format!("[needle]    {needle}\n"));
        msg.push_str(&format!("[expected]  {ctx_expected}\n"));
        msg.push_str(&format!("[locale]    {ctx_locale}\n"));
        msg.push_str("\n");

        // Старый head(10)
        msg.push_str("--- head(10) ---\n");
        for line in haystack.lines().take(10) {
            msg.push_str(line);
            msg.push('\n');
        }

        // Новый блок: полный дамп если короткий
        if haystack.len() < 2000 {
            msg.push_str("--- full output ---\n");
            msg.push_str(haystack);
            msg.push('\n');
        } else {
            // sample остаётся на случай длинного вывода
            msg.push_str("--- sample(200 chars) ---\n");
            let preview: String = haystack.chars().take(200).collect();
            msg.push_str(&preview);
            msg.push_str("…\n");
        }

        panic!("{}", msg);
    }
}

/// Упрощённая проверка для случаев без локали (например, XML/PO файлы)
pub fn assert_contains_file(
    haystack: &str,
    needle: &str,
    context_msg: &str,
    ctx_file: &Path,
    ctx_key: &str,
) {
    assert_contains_with_context(haystack, needle, context_msg, CTX_NONE, ctx_file, ctx_key);
}

/// Проверка, когда ожидаемая строка может быть как в stdout, так и в stderr
pub fn assert_contains_in_outputs(
    stdout: &str,
    stderr: &str,
    needle: &str,
    context_msg: &str,
    ctx_locale: &str,
    ctx_file: &Path,
    ctx_key: &str,
) {
    let combined = format!("{}{}", stdout, stderr);
    assert_contains_with_context(&combined, needle, context_msg, ctx_locale, ctx_file, ctx_key);
}

/// Проверить, что в тексте присутствуют **все** заданные элементы (needle, file, key)
pub fn assert_all_present(
    haystack: &str,
    items: &[(&str, &Path, &str)], // (needle, file, key)
    ctx_locale: &str,
    context_msg: &str,
) {
    for (needle, file, key) in items {
        assert_contains_with_context(haystack, needle, context_msg, ctx_locale, file, key);
    }
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
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args);
    // If developer wants more logs during tests: export RUST_LOG=debug
    if std::env::var("RIMLOC_TEST_VERBOSE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        cmd.env("RUST_LOG", "debug");
    }
    let output = cmd.output().expect("failed to spawn rimloc-cli");
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
    let ftl_path = i18n_dir.join(lang).join("rimloc.ftl");

    assert_success_with_context(
        code,
        &stdout,
        &stderr,
        lang,
        &ftl_path,
        key,
        &format!("command failed; expected key `{}` for lang `{}`", key, lang),
    );

    let output = format!("{}{}", stdout, stderr);
    assert_contains_with_context(
        &output,
        &expected,
        &format!("missing FTL key `{}` for lang `{}`", key, lang),
        lang,
        &ftl_path,
        key,
    );
}

/// Assert that a path exists, with a short context key for diagnostics.
pub fn assert_path_exists(path: &Path, context_msg: &str, ctx_key: &str) {
    assert!(
        path.exists(),
        "{} :: missing path {:?} (key = {})",
        context_msg,
        path,
        ctx_key
    );
}

/// Assert that `haystack` contains at least `min` occurrences of `needle`, with rich context.
pub fn assert_count_at_least(
    haystack: &str,
    needle: &str,
    min: usize,
    context_msg: &str,
    ctx_locale: &str,
    ctx_file: &Path,
    ctx_key: &str,
) {
    let count = haystack.matches(needle).count();
    if count >= min {
        return;
    }
    fail_with_context!(ctx_locale, ctx_file, ctx_key, context_msg, needle, haystack);
}

/// Assert that two sets are equal; shows missing/extra elements with context.
pub fn assert_set_eq(
    got: &std::collections::BTreeSet<String>,
    expected: &std::collections::BTreeSet<String>,
    context_msg: &str,
    ctx_locale: &str,
    ctx_file: &Path,
    ctx_key: &str,
) {
    if got == expected {
        return;
    }
    let missing: Vec<_> = expected.difference(got).cloned().collect();
    let extra: Vec<_> = got.difference(expected).cloned().collect();
    let detail = format!(
        "{}\n--- missing in GOT ({}) ---\n{:?}\n--- extra in GOT ({}) ---\n{:?}",
        context_msg,
        missing.len(),
        missing,
        extra.len(),
        extra
    );
    // Use empty needle and haystack since we compare sets, not substrings
    fail_with_context!(ctx_locale, ctx_file, ctx_key, &detail, "<set-diff>", "<no-haystack>");
}

/// Assert that a key exists in a map; useful for FTL key presence checks.
pub fn assert_map_contains_key(
    map: &std::collections::BTreeMap<String, String>,
    key: &str,
    ctx_locale: &str,
    ctx_file: &Path,
    context_msg: &str,
) {
    if map.contains_key(key) {
        return;
    }
    let detail = format!(
        "{}\n--- missing key ---\n{}\n--- available keys ({} first) ---\n{:?}",
        context_msg,
        key,
        map.len().min(20),
        map.keys().take(20).collect::<Vec<_>>()
    );
    fail_with_context!(ctx_locale, ctx_file, key, &detail, key, "<map-keys>");
}

fn extract_fluent_vars(s: &str) -> std::collections::BTreeSet<String> {
    // Find tokens like { $var } without regex
    let mut vars = std::collections::BTreeSet::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' {
            // skip spaces to potential '$'
            let mut j = i + 1;
            while j + 1 < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'$' {
                // read identifier
                j += 1;
                let start = j;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'-')
                {
                    j += 1;
                }
                if j > start {
                    let name = &s[start..j];
                    vars.insert(name.to_string());
                }
            }
        }
        i += 1;
    }
    vars
}

/// Сравнить карты FTL для локали с эталоном EN: те же ключи и те же наборы плейсхолдеров.
/// Пишет подробный отчёт; при RIMLOC_TEST_ARTIFACTS=1 записывает JSON артефакт.
pub fn assert_locale_diff(
    loc: &str,
    reference: &std::collections::BTreeMap<String, String>, // en
    other: &std::collections::BTreeMap<String, String>,      // loc
    ctx_file: &std::path::Path,
    section_for_key: fn(&str) -> &'static str,
    context_msg: &str,
) {
    use std::collections::{BTreeMap, BTreeSet};

    let en_keys: BTreeSet<_> = reference.keys().cloned().collect();
    let other_keys: BTreeSet<_> = other.keys().cloned().collect();

    // allowlist (опционально): crates/rimloc-cli/tests/i18n_allowlist.toml
    let allowlist: std::collections::BTreeSet<String> = (|| {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("i18n_allowlist.toml");
        let mut set = BTreeSet::new();
        if let Ok(text) = std::fs::read_to_string(&p) {
            // очень простой формат: строки "loc:key" или "[loc]\nkey=1"
            for line in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
                if let Some((l, k)) = line.split_once(':') {
                    if l == loc { set.insert(k.to_string()); }
                }
            }
        }
        set
    })();

    let missing: Vec<String> =
        en_keys.difference(&other_keys).filter(|k| !allowlist.contains(*k)).cloned().collect();
    let extra: Vec<String> = other_keys.difference(&en_keys).cloned().collect();

    // параметрические несовпадения на общих ключах
    let common: Vec<_> = en_keys.intersection(&other_keys).cloned().collect();
    let mut arg_mismatches: Vec<(String, BTreeSet<String>, BTreeSet<String>)> = Vec::new();
    for k in common {
        if allowlist.contains(&k) { continue; }
        let en_vars = extract_fluent_vars(reference.get(&k).unwrap());
        let ot_vars = extract_fluent_vars(other.get(&k).unwrap());
        if en_vars != ot_vars {
            arg_mismatches.push((k, en_vars, ot_vars));
        }
    }

    if missing.is_empty() && extra.is_empty() && arg_mismatches.is_empty() {
        return;
    }

    // Собираем человекочитаемый отчёт с группировкой
    let mut msg = String::new();
    msg.push_str(&format!("Locale {} has issues:\n", loc));

    let cap = 20usize; // сколько элементов выводить в каждой группе
    if !missing.is_empty() {
        let mut by_sec: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
        for k in &missing { by_sec.entry(section_for_key(k)).or_default().push(k); }
        msg.push_str(&format!("  • Missing keys ({} total):\n", missing.len()));
        for (sec, items) in by_sec {
            let shown = items.len().min(cap);
            msg.push_str(&format!("    - in section {} ({}): {:?}\n", sec, items.len(), &items[..shown]));
            if items.len() > cap { msg.push_str(&format!("      +{} more…\n", items.len() - cap)); }
        }
    }
    if !extra.is_empty() {
        let mut by_sec: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
        for k in &extra { by_sec.entry(section_for_key(k)).or_default().push(k); }
        msg.push_str(&format!("  • Extra keys ({} total):\n", extra.len()));
        for (sec, items) in by_sec {
            let shown = items.len().min(cap);
            msg.push_str(&format!("    - in section {} ({}): {:?}\n", sec, items.len(), &items[..shown]));
            if items.len() > cap { msg.push_str(&format!("      +{} more…\n", items.len() - cap)); }
        }
    }
    if !arg_mismatches.is_empty() {
        msg.push_str(&format!("  • Keys with argument set mismatch ({}):\n", arg_mismatches.len()));
        for (i, (k, en_vars, ot_vars)) in arg_mismatches.iter().enumerate().take(cap) {
            msg.push_str(&format!("    - {}\n      en: {:?}\n      {}: {:?}\n", k, en_vars, loc, ot_vars));
            if i + 1 == cap && arg_mismatches.len() > cap {
                msg.push_str(&format!("      +{} more…\n", arg_mismatches.len() - cap));
                break;
            }
        }
    }

    // Артефакт для CI (по флагу)
    if std::env::var("RIMLOC_TEST_ARTIFACTS").ok().as_deref() == Some("1") {
        let outdir = std::path::Path::new("target").join("test-artifacts");
        let _ = std::fs::create_dir_all(&outdir);
        let file = outdir.join(format!("locale-diff-{}.json", loc));
        #[derive(serde::Serialize)]
        struct Mismatch<'a> {
            missing: &'a [String],
            extra: &'a [String],
            arg_mismatches: Vec<(&'a String, Vec<String>, Vec<String>)>,
        }
        let payload = Mismatch {
            missing: &missing,
            extra: &extra,
            arg_mismatches: arg_mismatches.iter()
                .map(|(k, a, b)| (k, a.iter().cloned().collect(), b.iter().cloned().collect()))
                .collect(),
        };
        let _ = std::fs::write(&file, serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string()));
    }

    // Единый формат падения
    fail_with_context!(loc, ctx_file, "<locale-diff>", context_msg, "<locale-mismatch>", &msg);
}

/// Упрощённый макрос для вызова [`assert_contains_with_context`].
/// Используется, чтобы убрать дублирование аргументов в тестах.
#[macro_export]
macro_rules! assert_has {
    ($out:expr, $needle:expr, $path:expr, $ctx_locale:expr, $ctx_key:expr, $ctx_expected:expr $(,)?) => {
        crate::helpers::assert_contains_with_context(
            $out,
            $needle,
            $ctx_expected,
            $ctx_locale,
            $path,
            $ctx_key,
        )
    };
}

/// Проверяет, что в каждом FTL-файле для указанных локалей есть нужный ключ.
/// Если ключ отсутствует, печатает список доступных ключей для диагностики.
pub fn assert_ftl_key_present_all(locales: &[(&str, &std::path::Path)], key: &str) {
    for (locale, ftl_path) in locales {
        let content = std::fs::read_to_string(ftl_path)
            .unwrap_or_else(|e| panic!("не удалось прочитать {:?}: {}", ftl_path, e));

        let keys: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    None
                } else if let Some((k, _)) = line.split_once('=') {
                    Some(k.trim().to_string())
                } else {
                    None
                }
            })
            .collect();

        if !keys.iter().any(|k| k == key) {
            let list = keys
                .iter()
                .map(|k| format!(" - {}", k))
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "[{}] {:?} отсутствует ключ = {}\n--- available keys ---\n{}\n--- end of list ---",
                locale,
                ftl_path,
                key,
                list
            );
        }
    }
}

/// Assert that process exited successfully (code == 0) with rich context.
pub fn assert_success_with_context(
    code: i32,
    stdout: &str,
    stderr: &str,
    ctx_locale: &str,
    ctx_file: &std::path::Path,
    ctx_key: &str,
    context_msg: &str,
) {
    if code == 0 {
        return;
    }
    let joined = format!(
        "exit={}\n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        code, stdout, stderr
    );
    // needle — «ожидаем» успешный код (0), haystack — объединённый вывод
    fail_with_context!(
        ctx_locale,
        ctx_file,
        ctx_key,
        context_msg,
        "<exit code 0>",
        &joined
    );
}

/// Assert file exists and non-empty; on failure show size and folder listing.
pub fn assert_file_nonempty(
    path: &std::path::Path,
    context_msg: &str,
    ctx_key: &str,
) {
    if !path.exists() {
        assert_path_exists(path, context_msg, ctx_key); // выбросит подробную панику
        return; // на всякий
    }
    let meta = std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("{} :: cannot stat {:?}: {}", context_msg, path, e));
    if meta.len() > 0 {
        return;
    }
    // Соберём список соседей для быстрой отладки
    let mut listing = String::new();
    if let Some(dir) = path.parent() {
        let _ = std::fs::read_dir(dir).map(|rd| {
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("<non-utf8>");
                let sz = std::fs::metadata(&p).ok().map(|m| m.len()).unwrap_or(0);
                listing.push_str(&format!(" - {} ({} B)\n", name, sz));
            }
        });
    }
    let detail = format!(
        "{}\n--- file ---\n{:?}\nsize=0 B\n--- siblings ---\n{}",
        context_msg, path, listing
    );
    fail_with_context!(CTX_NONE, path, ctx_key, &detail, "<non-empty file>", "<empty file>");
}

/// Assert that directory contains at least one *.xml; otherwise print directory listing.
pub fn assert_dir_contains_xml(
    dir: &std::path::Path,
    context_msg: &str,
    ctx_key: &str,
) {
    let mut xmls = Vec::new();
    let mut listing = String::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("<non-utf8>");
            let sz = std::fs::metadata(&p).ok().map(|m| m.len()).unwrap_or(0);
            listing.push_str(&format!(" - {} ({} B)\n", name, sz));
            if p.extension().and_then(|s| s.to_str()) == Some("xml") {
                xmls.push(p);
            }
        }
    }
    if !xmls.is_empty() {
        return;
    }
    let detail = format!(
        "{}\n--- dir ---\n{:?}\n--- listing ---\n{}",
        context_msg, dir, listing
    );
    fail_with_context!(CTX_NONE, dir, ctx_key, &detail, "<*.xml present>", "<no xml files>");
}
