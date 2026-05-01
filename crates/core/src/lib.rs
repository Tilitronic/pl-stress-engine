mod syllabify;
mod dict;
mod rules;
mod transcribe;

pub use dict::{init_global_dict, global_dict, DictEntry, StressDict};
pub use syllabify::{count_syllables, syllabify};
pub use transcribe::transcribe;

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

/// Full stress resolution result.
#[derive(Debug, Clone, PartialEq)]
pub struct StressResult {
    /// Syllables of the word (lowercase), split by the hyphenation engine.
    pub syllables: Vec<String>,
    /// 0-based index of the stressed syllable from the start.
    pub syllable_index: usize,
    /// IPA transcription, if available from the exception dictionary.
    pub ipa: Option<String>,
    pub confidence: Confidence,
}

impl StressResult {
    /// 1-based syllable count from the end (Polish convention).
    /// penultimate = 2, antepenultimate = 3, …
    pub fn stress_from_end(&self) -> usize {
        self.syllables.len().saturating_sub(self.syllable_index)
    }

    /// The text of the stressed syllable, or `None` for empty words.
    pub fn stressed_syllable(&self) -> Option<&str> {
        self.syllables.get(self.syllable_index).map(String::as_str)
    }

    /// Compute IPA transcription from orthographic rules.
    /// Falls back to the exception-dictionary `ipa` field if present.
    pub fn ipa_transcribed(&self) -> String {
        if let Some(dict_ipa) = &self.ipa {
            return dict_ipa.clone();
        }
        transcribe::transcribe(&self.syllables, self.syllable_index)
    }
}
