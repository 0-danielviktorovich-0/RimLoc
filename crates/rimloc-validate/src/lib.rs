use rimloc_core::TransUnit;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use color_eyre::eyre::Result;

/// Результат одной проверки
#[derive(Debug)]
pub struct ValidationMessage {
    pub key: String,
    pub path: String,
    pub line: Option<u32>,
    pub kind: String,
    pub message: String,
}

/// Запустить все валидации
pub fn validate(units: &[TransUnit]) -> Result<Vec<ValidationMessage>> {
    let mut messages = Vec::new();

    // --- Проверка на дубликаты ---
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for u in units {
        if let Some(count) = seen.get(u.key.as_str()) {
            messages.push(ValidationMessage {
                key: u.key.clone(),
                path: u.path.display().to_string(),
                line: u.line,
                kind: "duplicate".into(),
                message: format!("ключ встречается повторно (всего уже {count} раз)"),
            });
        }
        *seen.entry(&u.key).or_insert(0) += 1;
    }

    // --- Проверка пустых значений ---
    for u in units {
        if u.source.as_deref().unwrap_or("").trim().is_empty() {
            messages.push(ValidationMessage {
                key: u.key.clone(),
                path: u.path.display().to_string(),
                line: u.line,
                kind: "empty".into(),
                message: "пустое значение".into(),
            });
        }
    }

    // --- Проверка плейсхолдеров ---
    let re = Regex::new(r"(\{\w+\}|\{\d+\}|%s|%d)").unwrap();
    for u in units {
        let text = u.source.as_deref().unwrap_or("");
        let placeholders: HashSet<_> = re
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect();

        // пока у нас нет перевода отдельно от source,
        // поэтому просто проверяем, что плейсхолдеры существуют (как подсказка)
        if !placeholders.is_empty() {
            messages.push(ValidationMessage {
                key: u.key.clone(),
                path: u.path.display().to_string(),
                line: u.line,
                kind: "placeholder-check".into(),
                message: format!("найдены плейсхолдеры: {:?}", placeholders),
            });
        }
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*; // подтянет validate(..) и типы из этого же крейта
    use rimloc_core::TransUnit;
    use std::path::PathBuf;

    fn tu(key: &str, src: Option<&str>, line: u32) -> TransUnit {
        TransUnit {
            key: key.to_string(),
            source: src.map(|s| s.to_string()),
            path: PathBuf::from("Dummy/Keyed/Bad.xml"),
            line: Some(line),
        }
    }

    #[test]
    fn detects_duplicate_and_empty_and_placeholders() {
        // два одинаковых ключа -> duplicate
        // пустая строка -> empty
        // плейсхолдеры -> placeholder-check
        let units = vec![
            tu("DuplicateKey", Some("Hello"), 3),
            tu("DuplicateKey", Some("World"), 5), // дубликат
            tu("EmptyKey", Some("   "), 7),       // пустое значение (или None)
            tu("WithPlaceholders", Some("Value {NAME} %d"), 11),
        ];

        let msgs = validate(&units).expect("validate should succeed");

        // ожидаем три вида сообщений
        let kinds: std::collections::HashSet<_> =
            msgs.iter().map(|m| m.kind.as_str()).collect();

        assert!(kinds.contains("duplicate"), "should report duplicate");
        assert!(kinds.contains("empty"), "should report empty");
        assert!(kinds.contains("placeholder-check"), "should report placeholders");

        // и суммарно хотя бы 3 сообщения
        assert!(msgs.len() >= 3, "should have at least 3 messages");
    }
}
