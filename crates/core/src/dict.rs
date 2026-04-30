use crate::rules::apply_rules;
use crate::syllabify::syllabify;
use crate::{Confidence, StressResult};
use ahash::AHashMap;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One entry in the exception dictionary.
/// Stored in the binary blob compiled into the WASM / Python extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictEntry {
    /// 0-based stressed syllable index from the start of the word.
    pub stress_idx: u8,
    /// IPA transcription, if captured from Wiktionary.
    pub ipa: Option<String>,
}

static DICT: OnceCell<StressDict> = OnceCell::new();

pub struct StressDict {
    exceptions: AHashMap<String, DictEntry>,
}

impl StressDict {
    /// Deserialise from the bincode blob produced by the builder.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        let std_map: HashMap<String, DictEntry> = bincode::deserialize(bytes)?;
        Ok(Self {
            exceptions: std_map.into_iter().collect(),
        })
    }

    /// Empty dict (rule-only mode, useful for testing without a built dict).
    pub fn empty() -> Self {
        Self {
            exceptions: AHashMap::new(),
        }
    }

    /// Resolve stress for a word, returning the full [`StressResult`].
    pub fn stress(&self, word: &str) -> StressResult {
        let lower = word.to_lowercase();
        let syllables = syllabify(&lower);
        let n = syllables.len();

        // 1. Exact exception dictionary lookup
        if let Some(entry) = self.exceptions.get(&lower) {
            let idx = (entry.stress_idx as usize).min(n.saturating_sub(1));
            return StressResult {
                syllable_index: idx,
                syllables,
                ipa: entry.ipa.clone(),
                confidence: Confidence::Exact,
            };
        }

        // 2. Productive grammatical rules
        if let Some((idx, conf)) = apply_rules(&lower, n) {
            return StressResult {
                syllable_index: idx,
                syllables,
                ipa: None,
                confidence: conf,
            };
        }

        // 3. Default: penultimate
        let penultimate = if n >= 2 { n - 2 } else { 0 };
        StressResult {
            syllable_index: penultimate,
            syllables,
            ipa: None,
            confidence: Confidence::Default,
        }
    }

    /// Convenience: return only the 0-based syllable index.
    pub fn stress_index(&self, word: &str) -> usize {
        self.stress(word).syllable_index
    }
}

/// Initialise the global singleton from raw bytes (call once at startup).
pub fn init_global_dict(bytes: Vec<u8>) -> Result<(), &'static str> {
    let dict = StressDict::from_bytes(&bytes).map_err(|_| "Failed to deserialise dictionary")?;
    DICT.set(dict).map_err(|_| "Dictionary already initialised")?;
    Ok(())
}

/// Access the global dictionary. Returns `None` if not yet initialised.
pub fn global_dict() -> Option<&'static StressDict> {
    DICT.get()
}
