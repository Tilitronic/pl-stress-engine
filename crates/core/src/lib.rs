mod syllabify;
mod dict;
mod rules;
mod transcribe;

pub use dict::{init_global_dict, global_dict, DictEntry, StressDict};
pub use syllabify::{count_syllables, syllabify};
pub use transcribe::transcribe;

use std::collections::HashMap;

// ── Public result types ───────────────────────────────────────────────────────

/// How the stress was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Found in the exception dictionary (Wiktionary / PoliMorf derived).
    Exact,
    /// Matched a productive grammatical rule (past plural, conditional, etc.).
    Rule,
    /// Fell back to the default: penultimate syllable.
    Default,
}

/// One morphological reading of a word form.
///
/// Follows [Universal Dependencies](https://universaldependencies.org/) naming.
/// Polish currently provides no morphological data, so `pos`, `feats`, and
/// `lemma` are always empty / `None` here.  The type is shared with the
/// Ukrainian engine (`ua-stress-engine`) for cross-engine API parity.
#[derive(Debug, Clone)]
pub struct MorphReading {
    /// UD POS tags, e.g. `["NOUN"]`.  Empty for Polish.
    pub pos: Vec<String>,
    /// UD feature map, e.g. `{"Case": ["Nom"], "Number": ["Sing"]}`.  Empty for Polish.
    pub feats: HashMap<String, Vec<String>>,
    /// Base form (lemma).  `None` for Polish.
    pub lemma: Option<String>,
    /// Short sense label from Wiktionary used to disambiguate homographs
    /// (e.g. "castle" vs "lock" for Ukrainian «замок»).  Empty for Polish.
    pub definition: Option<String>,
}

/// One stress variant of a word form.
///
/// Part of [`WordLookupResult`].  Mirrors `StressReading` in the Ukrainian
/// engine so both engines can be consumed with the same client code.
#[derive(Debug, Clone)]
pub struct StressReading {
    // ── Stress position ──────────────────────────────────────────────────────
    /// 0-based index of the stressed syllable from the start of the word.
    /// `0` for zero-syllable (purely consonantal) words like "z" or "w".
    pub syllable_index: usize,
    /// 1-based position from the end (2 = penultimate, 3 = antepenultimate).
    /// `0` for zero-syllable words.
    pub stress_from_end: usize,
    /// Total number of syllables.  `0` for purely consonantal words.
    pub syllable_count: usize,
    // ── Written representation ───────────────────────────────────────────────
    /// Normalized (lowercased) input form.
    pub form: String,
    /// Form with a combining acute U+0301 after the stressed vowel.
    /// Equal to `form` when there is no vowel to mark.
    pub stressed_form: String,
    /// Grapheme syllables, positionally aligned with `ipa_syllables`.
    /// Empty for zero-syllable words.
    pub word_syllables: Vec<String>,
    // ── Phonetic representation ──────────────────────────────────────────────
    /// Full IPA string from the G2P pipeline.
    pub ipa: String,
    /// IPA per syllable.  The stressed syllable is prefixed with `ˈ`.
    /// Empty for zero-syllable words.
    pub ipa_syllables: Vec<String>,
    // ── Morphology (UD) ──────────────────────────────────────────────────────
    /// Morphological analyses sharing this stress position.  Empty for Polish.
    pub morph: Vec<MorphReading>,
    // ── Source quality ───────────────────────────────────────────────────────
    /// How the stress was determined: `"exact"` | `"rule"` | `"default"`.
    /// `None` for Ukrainian (all entries are confirmed dictionary forms).
    pub confidence: Option<String>,
}

/// Top-level result of `lookup()`.
///
/// For Polish this always contains exactly one `StressReading` (Polish stress
/// is near-deterministic).  For Ukrainian there may be multiple readings
/// (heteronyms and variative stress).  `readings` is empty only for words
/// completely absent from all sources.
#[derive(Debug, Clone)]
pub struct WordLookupResult {
    /// Normalized query form.
    pub form: String,
    /// All stress variants.  Always one element for Polish.
    pub readings: Vec<StressReading>,
}

// ── Internal StressResult (used by dict + rules machinery) ───────────────────

/// Internal resolution result — used by `dict.rs` and `rules.rs`.
/// Not part of the public API; callers receive [`WordLookupResult`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct StressResult {
    pub syllables: Vec<String>,
    pub syllable_index: usize,
    pub ipa: Option<String>,
    pub confidence: Confidence,
}

impl StressResult {
    /// 1-based position from the end (penultimate = 2).
    /// Returns 0 for zero-syllable words.
    pub fn stress_from_end(&self) -> usize {
        let n = self.syllables.len();
        if n == 0 { 0 } else { n - self.syllable_index }
    }

    pub fn stressed_syllable(&self) -> Option<&str> {
        self.syllables.get(self.syllable_index).map(String::as_str)
    }

    /// Full IPA string: prefer dict entry, fall back to G2P pipeline.
    pub fn ipa_transcribed(&self) -> String {
        if let Some(dict_ipa) = &self.ipa {
            return dict_ipa.clone();
        }
        transcribe::transcribe(&self.syllables, self.syllable_index)
    }

    /// Per-syllable IPA from the G2P pipeline (always computed, never split from dict IPA).
    pub fn ipa_transcribed_syllables(&self) -> Vec<String> {
        transcribe::transcribe_syllables(&self.syllables)
    }

    /// Convert into a public [`StressReading`].
    pub fn into_reading(self, form: String) -> StressReading {
        let n = self.syllables.len();
        let conf = match self.confidence {
            Confidence::Exact   => "exact",
            Confidence::Rule    => "rule",
            Confidence::Default => "default",
        };
        let ipa = self.ipa_transcribed();
        let ipa_syls = self.ipa_transcribed_syllables();
        let stressed_form = apply_stress_mark(&form, &self.syllables, self.syllable_index);
        let ipa_syllables: Vec<String> = ipa_syls
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if i == self.syllable_index && n > 0 {
                    format!("\u{02c8}{}", s)
                } else {
                    s.clone()
                }
            })
            .collect();
        StressReading {
            syllable_index: self.syllable_index,
            stress_from_end: self.stress_from_end(),
            syllable_count: n,
            form: form.clone(),
            stressed_form,
            word_syllables: self.syllables,
            ipa,
            ipa_syllables,
            morph: Vec::new(),
            confidence: Some(conf.to_string()),
        }
    }
}

/// Insert a combining acute U+0301 after the stressed vowel.
///
/// Counts the vowels in `syllables[..syllable_index]` to determine which
/// vowel in `form` is stressed, then inserts U+0301 after it.
/// Returns `form` unchanged when there is no vowel (zero-syllable words).
pub fn apply_stress_mark(form: &str, syllables: &[String], syllable_index: usize) -> String {
    const VOWELS: &str = "aeiouąęóy";
    if syllables.is_empty() {
        return form.to_string();
    }
    let vowels_before: usize = syllables[..syllable_index.min(syllables.len())]
        .iter()
        .flat_map(|s| s.chars())
        .filter(|c| VOWELS.contains(*c))
        .count();
    let mut count = 0usize;
    let mut out = String::with_capacity(form.len() + 2);
    for c in form.chars() {
        out.push(c);
        if VOWELS.contains(c) {
            if count == vowels_before {
                out.push('\u{0301}');
            }
            count += 1;
        }
    }
    out
}
