use crate::syllabify::count_syllables;
use crate::Confidence;

/// Productive grammatical rules for Polish stress exceptions.
///
/// Returns `Some((syllable_index, confidence))` if the word matches a rule,
/// where `syllable_index` is 0-based from the start and `n` is the total
/// syllable count (pre-computed by the caller from `syllabify`).
pub fn apply_rules(word: &str, n: usize) -> Option<(usize, Confidence)> {
    let lower = word.to_lowercase();

    // --- Acronyms / abbreviations: oxytone (last syllable) ---
    // Heuristic: all characters are ASCII uppercase letters.
    if word.len() >= 2 && word.chars().all(|c| c.is_ascii_uppercase()) {
        return Some((n.saturating_sub(1), Confidence::Rule));
    }

    // --- One-syllable stems with arcy-/eks-/wice- prefix: oxytone ---
    let oxytone_prefixes = ["arcy", "eks", "wice"];
    for prefix in &oxytone_prefixes {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if count_syllables(rest) == 1 {
                return Some((n.saturating_sub(1), Confidence::Rule));
            }
        }
    }

    if n < 3 {
        return None; // rules below only apply to 3+ syllable words
    }

    // --- 1st/2nd person plural past tense: antepenultimate (3rd from end) ---
    // Endings: -liśmy, -łyśmy, -liście, -łyście
    let past_plural = ["liśmy", "łyśmy", "liście", "łyście"];
    for ending in &past_plural {
        if lower.ends_with(ending) {
            return Some((n - 3, Confidence::Rule));
        }
    }

    // --- Conditional mood, singular + 3rd plural: antepenultimate ---
    // Endings: -łbym, -łbyś, -łby, -łaby, -łabyś, -łabym, -liby, -łyby
    let cond_singular = [
        "łbym", "łbyś", "łby", "łaby", "łabyś", "łabym",
        "liby", "łyby", "łoby",
    ];
    for ending in &cond_singular {
        if lower.ends_with(ending) {
            return Some((n - 3, Confidence::Rule));
        }
    }

    // --- Conditional mood, 1st/2nd plural: pre-antepenultimate (4th from end) ---
    // Endings: -libyśmy, -libyście, -łybyśmy, -łybyście
    if n >= 4 {
        let cond_plural = ["libyśmy", "libyście", "łybyśmy", "łybyście"];
        for ending in &cond_plural {
            if lower.ends_with(ending) {
                return Some((n - 4, Confidence::Rule));
            }
        }
    }

    // --- Words ending in -ika/-yka (Latin loans): antepenultimate ---
    if lower.ends_with("ika") || lower.ends_with("yka") {
        return Some((n - 3, Confidence::Rule));
    }

    // --- Conjunctions with conditional/personal morphemes: antepenultimate ---
    // abyśmy, żebyście, jeśliby, etc.
    let conj_prefixes = ["aby", "żeby", "ażeby", "jeśli", "jeżeli", "iżby", "ponieważ"];
    for prefix in &conj_prefixes {
        if lower.starts_with(prefix) && lower.len() > prefix.len() {
            let rest = &lower[prefix.len()..];
            if matches!(rest, "śmy" | "ście" | "by" | "bym" | "byś") {
                return Some((n - 3, Confidence::Rule));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acronym_oxytone() {
        // PKP: 1 syllable → index 0 (last = only)
        let n = crate::count_syllables("PKP");
        assert_eq!(apply_rules("PKP", n), Some((n.saturating_sub(1), Confidence::Rule)));
    }

    #[test]
    fn test_past_plural_antepenultimate() {
        // zrobiliśmy: 5 syllables → index 2
        let n = crate::count_syllables("zrobiliśmy");
        let r = apply_rules("zrobiliśmy", n);
        assert!(r.is_some());
        let (idx, conf) = r.unwrap();
        assert_eq!(conf, Confidence::Rule);
        assert_eq!(idx, n - 3);
    }

    #[test]
    fn test_conditional_plural_preantepenultimate() {
        // zrobilibyśmy: 6 syllables → index 2
        let n = crate::count_syllables("zrobilibyśmy");
        let r = apply_rules("zrobilibyśmy", n);
        assert!(r.is_some());
        let (idx, _) = r.unwrap();
        assert_eq!(idx, n - 4);
    }

    #[test]
    fn test_ika_yka_antepenultimate() {
        let n = crate::count_syllables("fizyka");
        assert_eq!(apply_rules("fizyka", n), Some((n - 3, Confidence::Rule)));
        let n = crate::count_syllables("matematyka");
        assert_eq!(apply_rules("matematyka", n), Some((n - 3, Confidence::Rule)));
    }

    #[test]
    fn test_no_match_default_word() {
        let n = crate::count_syllables("kobieta");
        assert_eq!(apply_rules("kobieta", n), None);
    }
}
