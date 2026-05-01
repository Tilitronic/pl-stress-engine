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

fn is_vowel_char(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'ą' | 'ę' | 'ó')
}

fn is_strong_vowel_char(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'o' | 'u' | 'y' | 'ą' | 'ę' | 'ó')
}

fn is_consonant_digraph(prev: char, curr: char) -> bool {
    matches!((prev, curr),
        ('c', 'h') |
        ('c', 'z') |
        ('d', 'z') |
        ('d', 'ź') |
        ('d', 'ż') |
        ('r', 'z') |
        ('s', 'z')
    )
}

fn should_legacy_merge_ci_v(left: &str, right: &str) -> bool {
    let right_chars: Vec<char> = right.chars().collect();
    if right_chars.len() != 1 || !is_strong_vowel_char(right_chars[0]) {
        return false;
    }

    let left_chars: Vec<char> = left.chars().collect();
    if left_chars.len() < 2 || *left_chars.last().unwrap_or(&' ') != 'i' {
        return false;
    }

    let prev = left_chars[left_chars.len() - 2];
    if is_vowel_char(prev) {
        return false;
    }

    if left_chars.len() == 2 {
        return false;
    }

    let prev_prev = left_chars[left_chars.len() - 3];
    !is_vowel_char(prev_prev) && !is_consonant_digraph(prev_prev, prev)
}

fn legacy_split_offset(syllables: &[String], old_idx: usize) -> usize {
    let mut old_pos = 0usize;
    let mut extra = 0usize;
    let mut i = 0usize;

    while i < syllables.len() {
        if i + 1 < syllables.len() && should_legacy_merge_ci_v(&syllables[i], &syllables[i + 1]) {
            if old_pos < old_idx {
                extra += 1;
            }
            old_pos += 1;
            i += 2;
            continue;
        }

        old_pos += 1;
        i += 1;
    }

    extra
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
            let old_idx = entry.stress_idx as usize;
            let corrected_idx = old_idx + legacy_split_offset(&syllables, old_idx);
            let idx = corrected_idx.min(n.saturating_sub(1));
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::path::PathBuf;

    fn master_db_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("data")
            .join("master.db")
    }

    fn open_master_db() -> Connection {
        let path = master_db_path();
        Connection::open(&path).unwrap_or_else(|e| {
            panic!(
                "master.db not found at {}: {e}",
                path.display()
            )
        })
    }

    fn normalize_ipa(s: &str) -> String {
        s.chars()
            .filter_map(|c| {
                let mapped = match c {
                    'ˈ' | 'ˌ' | '[' | ']' | '/' | '.' | ' ' => return None,
                    'g' => 'ɡ',
                    'ä' => 'a',
                    _ => c,
                };
                Some(mapped)
            })
            .collect()
    }

    fn is_polish_like_word(word: &str) -> bool {
        word.chars().all(|c| {
            matches!(
                c,
                'a'..='z'
                    | 'ą' | 'ć' | 'ę' | 'ł' | 'ń' | 'ó' | 'ś' | 'ź' | 'ż'
                    | '-' | '\''
            )
        })
    }

    fn is_clean_ipa_entry(ipa: &str) -> bool {
        // Keep only single-word IPA-like strings; drop explanatory/noisy entries.
        // Excludes spaces, punctuation used in long comments, and phrase markers.
        !ipa.is_empty()
            && !ipa.contains(' ')
            && !ipa.contains('\n')
            && !ipa.contains('‿')
            && !ipa.contains(':')
            && !ipa.contains('"')
            && !ipa.contains('=')
    }

    fn is_polish_ipa_inventory_entry(ipa: &str) -> bool {
        // Exclude obvious non-Polish / foreign-phone transcriptions for a fair
        // benchmark against the Polish rule engine.
        // rules.md inventory does not include phones such as θ, ð, β, ɾ, ʌ, ə.
        !ipa.chars().any(|c| {
            matches!(
                c,
                'θ' | 'ð' | 'β' | 'ɾ' | 'ʌ' | 'ə' | 'ɚ' | 'ɜ' | 'ɞ' | 'ʊ' | 'ɯ' | 'ɒ' | 'ø'
                    | 'œ' | 'ɐ' | 'ǰ' | 'ː' | 'ˑ' | '˞' | 'ʔ'
            )
        })
    }

    fn levenshtein(a: &str, b: &str) -> usize {
        let ac: Vec<char> = a.chars().collect();
        let bc: Vec<char> = b.chars().collect();
        let mut prev: Vec<usize> = (0..=bc.len()).collect();
        let mut curr = vec![0usize; bc.len() + 1];

        for (i, ca) in ac.iter().enumerate() {
            curr[0] = i + 1;
            for (j, cb) in bc.iter().enumerate() {
                let cost = if ca == cb { 0 } else { 1 };
                curr[j + 1] = (curr[j] + 1)
                    .min(prev[j + 1] + 1)
                    .min(prev[j] + cost);
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev[bc.len()]
    }

    #[test]
    fn exact_index_is_shifted_for_split_ci_v_sequence() {
        let mut exceptions = AHashMap::new();
        exceptions.insert(
            "biblioteka".to_string(),
            DictEntry {
                stress_idx: 2,
                ipa: Some("ˌbʲiblʲjɔˈtɛka".to_string()),
            },
        );

        let dict = StressDict { exceptions };
        let result = dict.stress("biblioteka");

        assert_eq!(result.syllables, vec!["bi", "bli", "o", "te", "ka"]);
        assert_eq!(result.syllable_index, 3);
        assert_eq!(result.stress_from_end(), 2);
        assert_eq!(result.confidence, Confidence::Exact);
    }

    #[test]
    fn no_shift_for_softening_sequences_like_osiol() {
        let mut exceptions = AHashMap::new();
        exceptions.insert(
            "osioł".to_string(),
            DictEntry {
                stress_idx: 1,
                ipa: Some("ˈɔɕɔw".to_string()),
            },
        );

        let dict = StressDict { exceptions };
        let result = dict.stress("osioł");

        assert_eq!(result.syllables, vec!["o", "sioł"]);
        assert_eq!(result.syllable_index, 1);
        assert_eq!(result.stress_from_end(), 1);
        assert_eq!(result.confidence, Confidence::Exact);
    }

    #[test]
    fn masterdb_polimorf_does_not_provide_ipa() {
        let conn = open_master_db();

        let wiktionary_ipa_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM words WHERE stress_source = 'wiktionary_ipa' AND ipa IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("failed to count wiktionary IPA rows");

        let polimorf_ipa_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM words WHERE stress_source LIKE 'polimorf%' AND ipa IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("failed to count polimorf IPA rows");

        assert!(
            wiktionary_ipa_count > 0,
            "expected Wiktionary IPA rows in master DB"
        );
        assert_eq!(
            polimorf_ipa_count, 0,
            "PoliMorf-derived rows should not carry IPA"
        );
    }

    #[test]
    fn masterdb_all_wiktionary_ipa_roundtrip_in_stressdict() {
        let conn = open_master_db();

        let mut stmt = conn
            .prepare(
                "SELECT word, stress_from_end, ipa, syllable_count \
                 FROM words \
                                 WHERE stress_source LIKE 'wiktionary_%' \
                   AND ipa IS NOT NULL \
                   AND stress_from_end IS NOT NULL \
                                     AND syllable_count IS NOT NULL \
                                     AND EXISTS (SELECT 1 FROM morphology m WHERE m.word = words.word)",
            )
            .expect("failed to prepare Wiktionary IPA select");

        let rows = stmt
            .query_map([], |row| {
                let word: String = row.get(0)?;
                let sfe: i64 = row.get(1)?;
                let ipa: String = row.get(2)?;
                let sc: i64 = row.get(3)?;
                Ok((word, sfe, ipa, sc))
            })
            .expect("failed to query Wiktionary IPA rows");

        let mut exceptions = AHashMap::new();
        let mut expected = AHashMap::new();

        for row in rows {
            let (word, stress_from_end, ipa, syllable_count) = row.expect("row decode failed");
            if syllable_count <= 0 || stress_from_end <= 0 {
                continue;
            }
            let sc = syllable_count as usize;
            let sfe = stress_from_end as usize;
            if sfe > sc {
                continue;
            }

            let stress_idx = (sc - sfe) as u8;
            exceptions.insert(
                word.clone(),
                DictEntry {
                    stress_idx,
                    ipa: Some(ipa.clone()),
                },
            );
            expected.insert(word, ipa);
        }

        assert!(
            expected.len() > 100,
            "expected substantial Polish Wiktionary IPA coverage in master DB"
        );

        let dict = StressDict { exceptions };
        for (word, ipa) in expected {
            let r = dict.stress(&word);
            assert_eq!(r.confidence, Confidence::Exact, "word={word}");
            assert_eq!(r.ipa.as_deref(), Some(ipa.as_str()), "word={word}");
        }
    }

    #[test]
    fn masterdb_transcriber_quality_report() {
        let conn = open_master_db();

        let mut stmt = conn
            .prepare(
                "SELECT word, stress_from_end, ipa, syllable_count \
                 FROM words \
                                 WHERE stress_source LIKE 'wiktionary_%' \
                   AND ipa IS NOT NULL \
                   AND stress_from_end IS NOT NULL \
                                     AND syllable_count IS NOT NULL \
                                     AND EXISTS (SELECT 1 FROM morphology m WHERE m.word = words.word)",
            )
            .expect("failed to prepare quality-report query");

        let rows = stmt
            .query_map([], |row| {
                let word: String = row.get(0)?;
                let sfe: i64 = row.get(1)?;
                let ipa: String = row.get(2)?;
                let sc: i64 = row.get(3)?;
                Ok((word, sfe, ipa, sc))
            })
            .expect("failed to read quality-report rows");

        let mut total = 0usize;
        let mut exact = 0usize;
        let mut norm_exact = 0usize;
        let mut sum_similarity = 0.0f64;
        let mut total_pl = 0usize;
        let mut exact_pl = 0usize;
        let mut norm_exact_pl = 0usize;
        let mut sum_similarity_pl = 0.0f64;
        let mut worst: Vec<(f64, String, String, String)> = Vec::new();

        for row in rows {
            let (word, stress_from_end, expected_ipa, syll_count) = row.expect("row decode failed");
            if syll_count <= 0 || stress_from_end <= 0 {
                continue;
            }
            if !is_clean_ipa_entry(&expected_ipa) {
                continue;
            }
            if !is_polish_ipa_inventory_entry(&expected_ipa) {
                continue;
            }
            let sc = syll_count as usize;
            let sfe = stress_from_end as usize;
            if sfe > sc {
                continue;
            }

            let stress_idx = sc - sfe;
            let syllables = crate::syllabify::syllabify(&word);
            let actual_ipa = crate::transcribe::transcribe(&syllables, stress_idx);

            total += 1;
            if actual_ipa == expected_ipa {
                exact += 1;
            }

            let n_expected = normalize_ipa(&expected_ipa);
            let n_actual = normalize_ipa(&actual_ipa);
            if n_actual == n_expected {
                norm_exact += 1;
            }

            let max_len = n_expected.chars().count().max(n_actual.chars().count());
            let sim = if max_len == 0 {
                1.0
            } else {
                let d = levenshtein(&n_actual, &n_expected);
                1.0 - (d as f64 / max_len as f64)
            };
            sum_similarity += sim;

            if is_polish_like_word(&word) {
                total_pl += 1;
                if actual_ipa == expected_ipa {
                    exact_pl += 1;
                }
                if n_actual == n_expected {
                    norm_exact_pl += 1;
                }
                sum_similarity_pl += sim;
            }

            if worst.len() < 10 {
                worst.push((sim, word, actual_ipa, expected_ipa));
                worst.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            } else if sim < worst[9].0 {
                worst[9] = (sim, word, actual_ipa, expected_ipa);
                worst.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            }
        }

        assert!(total > 100, "too few rows for quality report: {total}");

        let exact_pct = exact as f64 * 100.0 / total as f64;
        let norm_exact_pct = norm_exact as f64 * 100.0 / total as f64;
        let mean_sim = sum_similarity / total as f64;

        let exact_pct_pl = if total_pl == 0 {
            0.0
        } else {
            exact_pl as f64 * 100.0 / total_pl as f64
        };
        let norm_exact_pct_pl = if total_pl == 0 {
            0.0
        } else {
            norm_exact_pl as f64 * 100.0 / total_pl as f64
        };
        let mean_sim_pl = if total_pl == 0 {
            0.0
        } else {
            sum_similarity_pl / total_pl as f64
        };

        println!("\\n=== PL-IPA quality report (vs master.db Polish Wiktionary IPA only) ===");
        println!("rows: {total}");
        println!("exact: {exact} ({exact_pct:.2}%)");
        println!("normalized-exact: {norm_exact} ({norm_exact_pct:.2}%)");
        println!("mean normalized similarity: {mean_sim:.4}");
        println!(
            "polish-like rows: {total_pl} | exact: {exact_pl} ({exact_pct_pl:.2}%) | normalized-exact: {norm_exact_pl} ({norm_exact_pct_pl:.2}%) | mean similarity: {mean_sim_pl:.4}"
        );
        println!("--- worst 10 examples ---");
        for (sim, word, actual, expected) in worst {
            println!("sim={sim:.4} | {word} | actual={actual} | expected={expected}");
        }
    }
}
