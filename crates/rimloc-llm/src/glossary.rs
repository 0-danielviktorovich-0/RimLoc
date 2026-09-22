//! Glossary: user terms override the built-in RimWorld baseline.

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Glossary {
    pub terms: BTreeMap<String, String>,
}

impl Glossary {
    /// Built-in community-consistent baseline (extendable; kept small).
    pub fn builtin() -> Self {
        let mut terms = BTreeMap::new();
        for (en, ru) in [
            ("colonist", "поселенец"),
            ("raider", "рейдер"),
            ("turret", "турель"),
            ("mortal error", "смертельная рана"),
            ("downed", "сбит с ног"),
            ("mental break", "нервный срыв"),
            ("recruit", "завербовать"),
            ("tame", "приручить"),
            ("harvest", "собрать урожай"),
            ("research", "исследование"),
            ("power", "электричество"),
            ("raid", "налёт"),
            ("trader", "торговец"),
            ("message", "сообщение"),
            ("letter", "уведомление"),
        ] {
            terms.insert(en.to_string(), ru.to_string());
        }
        Self { terms }
    }

    /// Load a JSON file: {"en term": "ru term", ...}. User terms override built-ins.
    pub fn load_json(path: &Path) -> Result<Self, crate::LlmError> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            crate::LlmError::CheckpointIo(format!("glossary {}: {e}", path.display()))
        })?;
        let terms: BTreeMap<String, String> = serde_json::from_str(&raw)
            .map_err(|e| crate::LlmError::CheckpointIo(format!("glossary parse: {e}")))?;
        Ok(Self { terms })
    }

    /// Built-in + user overrides.
    pub fn merged(&self) -> BTreeMap<String, String> {
        let mut all = Self::builtin().terms;
        for (k, v) in &self.terms {
            all.insert(k.clone(), v.clone());
        }
        all
    }
}
