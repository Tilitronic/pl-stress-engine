//! G2P-based syllabification for Polish.
//!
//! Replaces the old TeX hyphenation approach. Pipeline:
//!   1. Tokenize + palatalize: identify which `i` tokens are softening markers.
//!   2. Mark nuclei: every non-skipped, non-empty vowel token is a syllable nucleus.
//!   3. Split on nuclei using onset maximization (Sonority Sequencing + Maximal Onset).
//!
//! Reference: Sle dzinski (2018) "Wielowarstwowy model podzialnu wyrazow ortograficznych
//! jezyka polskiego na sylaby", POLONICA XXXVIII.

use crate::transcribe::tokenize_and_palatalize;
use once_cell::sync::Lazy;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Onset maximization: valid 2- and 3-consonant onset clusters for Polish.
// Source: Szpyra-Kozlowska sonority scale + Maximal Onset Principle.
// These are built from *token* ortho strings (digraphs already fused by tokenizer).
// ---------------------------------------------------------------------------

const VALID_ONSETS_2: &[&str] = &[
    // obstruent + liquid / nasal
    "bl", "br", "dl", "dr", "fl", "fr", "gl", "gr", "kl", "kn", "kr",
    "mn", "pl", "pr", "tr", "wr", "gn", "pn", "sn", "zn", "zm",
    // fricative clusters
    "sk", "sl", "sm", "sp", "st", "sw",
    "zb", "zd", "zg", "zl", "zr", "zw",
    // labiovelars
    "kw", "tw", "gw", "dw",
    // digraph tokens that happen to span two ortho chars — handled as single tokens
    // by the tokenizer, but listed here defensively in case they appear split.
    "ch", "cz", "dz", "rz", "sz",
    // palatalized fricative clusters (soft consonants + stops/liquids)
    "śc", "źd", "ść", "śr", "śl", "śm", "śn", "śp", "śt", "św",
    "ps", "cm",
    // zj cluster (zjeść, zjadać, amnezja)
    "zj",
    // cj cluster: /tsj/ onset in loanwords (akcja, lekcja, administracja)
    "cj",
    // ł-bearing clusters (same validity as l-bearing ones)
    "bł", "dł", "gł", "mł", "pł", "sł", "tł", "wł", "zł",
];

const VALID_ONSETS_3: &[&str] = &[
    "str", "skr", "spr", "zdr", "zgr", "zbr",
    "trz", "drz", "krz",
    "szk",
    // zwr: valid onset cluster (zrobiliście etc.) — zw+r onset preserved
    "zwr",
];

/// Given the non-skip consonant tokens *between* two nuclei, return how many
/// belong to the coda of the left syllable (the rest form the onset of the right).
fn coda_len(cluster: &[&str]) -> usize {
    let n = cluster.len();
    match n {
        0 | 1 => 0,
        2 => {
            let s = format!("{}{}", cluster[0], cluster[1]);
            if VALID_ONSETS_2.contains(&s.as_str()) { 0 } else { 1 }
        }
        _ => {
            let s3 = format!("{}{}{}", cluster[n-3], cluster[n-2], cluster[n-1]);
            if VALID_ONSETS_3.contains(&s3.as_str()) { return n - 3; }
            let s2 = format!("{}{}", cluster[n-2], cluster[n-1]);
            if VALID_ONSETS_2.contains(&s2.as_str()) { return n - 2; }
            n - 1
        }
    }
}

// ---------------------------------------------------------------------------
// Morphological prefix layer (Śledziński 2018 §3.1–3.2, layer 1).
// These prefixes force a syllable boundary BEFORE the phonological split.
// Ordered longest-first so the first match wins.
// ---------------------------------------------------------------------------

/// Known Polish prefixes that create a hard syllable boundary.
/// Ordered longest-first for greedy matching.
const PREFIXES: &[&str] = &[
    // 5-char
    "przed", "między", "współ",
    // 4-char
    "prze", "przy", "niez", "bezs", "bezw",
    // 3-char
    "roz", "nad", "pod", "bez", "nie", "wsp",
    // 2-char
    "ob", "od", "do", "po", "wy", "za", "na",
];

/// If `word` starts with a known prefix and the remainder has at least one
/// vowel (i.e. it is a real stem, not just inflectional noise), return the
/// split index (byte position of the boundary).
///
/// Also validates that the stem's initial consonant cluster is a valid Polish
/// onset — this prevents false matches like "po" + "rtfel" (portfel).
fn find_prefix_split(word: &str) -> Option<usize> {
    for prefix in PREFIXES {
        if word.starts_with(prefix) {
            let stem = &word[prefix.len()..];
            // Stem must be non-empty and contain at least one vowel.
            if !stem.is_empty() && stem.chars().any(|c| is_vowel_ortho_char(c)) {
                // Validate that the stem starts with a valid Polish onset cluster.
                if has_valid_stem_onset(stem) {
                    return Some(prefix.len());
                }
            }
        }
    }
    None
}

/// Returns `true` when the consonant cluster at the START of `stem` (all chars
/// before the first vowel) forms a valid Polish syllable onset.
///
/// Used to prevent short prefixes like "po" from firing on loanwords like
/// "portfel" where the stem's initial cluster "rtf" is not a Polish onset.
fn has_valid_stem_onset(stem: &str) -> bool {
    // Collect chars before the first vowel into a raw string.
    let onset: String = stem.chars()
        .take_while(|c| !is_vowel_ortho_char(*c))
        .collect();
    match onset.chars().count() {
        0 | 1 => true,                                   // vowel-initial or single consonant
        2 => VALID_ONSETS_2.contains(&onset.as_str()),   // 2-char cluster
        3 => VALID_ONSETS_3.contains(&onset.as_str()),   // 3-char cluster
        _ => false,                                      // 4+ consonants before vowel
    }
}

// ---------------------------------------------------------------------------
// Syllabification exception dictionary.
// These words cannot be handled correctly by the G2P rules and are listed
// explicitly.  The keys are lowercase.  Add entries sparingly — only for
// forms where the phonological or morphological rules genuinely cannot
// produce the correct split.
// ---------------------------------------------------------------------------

static SYLLABIFICATION_EXCEPTIONS: Lazy<HashMap<&'static str, &'static [&'static str]>> =
    Lazy::new(|| {
        let mut m: HashMap<&'static str, &'static [&'static str]> = HashMap::new();
        // bydlak: yd cluster → byd-lak (policy: phonetic, *not* by-dlak)
        m.insert("bydlak", &["byd", "lak"]);
        // ekspres: ksp cluster — ks in coda, pr onset → eks-pres
        m.insert("ekspres", &["eks", "pres"]);
        // ekspresowy, ekspresja etc. could be added if needed
        m.insert("ekspresja", &["eks", "pres", "ja"]);
        // tekstem: kst — tek-stem
        m.insert("tekstem", &["tek", "stem"]);
        m.insert("tekst", &["tekst"]);
        // portfel: rtf — port-fel
        m.insert("portfel", &["port", "fel"]);
        // biblioteka: bli-o split needed
        m.insert("biblioteka", &["bi", "bli", "o", "te", "ka"]);
        // amnezja: mne-zja (zj is valid onset but mn cluster resolution differs)
        m.insert("amnezja", &["a", "mne", "zja"]);
        // ćwierćwiecze: ćwi-erć-wie-cze
        m.insert("ćwierćwiecze", &["ćwi", "erć", "wie", "cze"]);
        // nadworny: nad- prefix must NOT apply; phonological na-dwor-ny (dw is valid onset)
        m.insert("nadworny", &["na", "dwor", "ny"]);
        // pownosić: po- prefix + wnosić; wn is valid word-initial onset → po-wno-sić
        m.insert("pownosić", &["po", "wno", "sić"]);
        m
    });

// ---------------------------------------------------------------------------
// Diphthong handling: au/eu at the START of a word (or syllable) stay together.
// In Polish loanwords "auto", "europa" the au/eu sequences are treated as
// a single nucleus (phonetically [aw]/[ɛw]).
// ---------------------------------------------------------------------------

/// If `word` begins with one of the Greek/Latin diphthongs au/eu, return the
/// byte length of that diphthong prefix so the caller can treat it as a single
/// block.
fn leading_diphthong(word: &str) -> Option<usize> {
    for dp in &["au", "eu"] {
        if word.starts_with(dp) {
            return Some(dp.len());
        }
    }
    None
}

fn is_vowel_ortho_char(c: char) -> bool {
    matches!(c, 'a'|'e'|'i'|'o'|'u'|'y'|'ą'|'ę'|'ó')
}

// ---------------------------------------------------------------------------
// Token representation
// ---------------------------------------------------------------------------

struct OrthoToken {
    ortho: String,
    is_nucleus: bool,
    is_skip: bool,   // true = softening `i`, belongs to onset of next consonant
}

fn is_vowel_ortho(s: &str) -> bool {
    // Polish vowel letters (including o-acute which may appear as composed char)
    matches!(s,
        "a" | "e" | "i" | "o" | "u" | "y" |
        "\u{f3}" |   // ó
        "\u{105}" |  // ą
        "\u{119}"    // ę
    )
}

/// Tokenize + palatalize, then classify each token.
fn analyze(word: &str) -> Vec<OrthoToken> {
    let lower = word.to_lowercase();
    let raw = tokenize_and_palatalize(&lower);
    raw.into_iter().map(|(ortho, is_skip)| {
        let is_nucleus = !is_skip && !ortho.is_empty() && is_vowel_ortho(&ortho);
        OrthoToken { ortho, is_nucleus, is_skip }
    }).collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Split a Polish word into orthographic syllables using G2P nucleus detection
/// with a morphological prefix layer (Śledziński 2018).
pub fn syllabify(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();

    // Exception dictionary layer: exact-match overrides everything.
    if let Some(&syls) = SYLLABIFICATION_EXCEPTIONS.get(lower.as_str()) {
        return syls.iter().map(|s| s.to_string()).collect();
    }

    // Diphthong layer: au/eu at word start stay in one syllable block.
    if let Some(dp_len) = leading_diphthong(&lower) {
        let rest = &lower[dp_len..];
        if rest.is_empty() {
            return vec![lower];
        }
        let dp = lower[..dp_len].to_string();
        let mut result = vec![dp];
        result.extend(syllabify_inner(rest));
        return result;
    }

    // Morphological layer: if word starts with a known prefix, recursively
    // syllabify prefix and stem independently, then concatenate.
    if let Some(split) = find_prefix_split(&lower) {
        let prefix = &lower[..split];
        let stem   = &lower[split..];
        let mut result = syllabify_raw(prefix);
        result.extend(syllabify_inner(stem));
        return result;
    }

    syllabify_raw(&lower)
}

/// Syllabify `word` (already lowercase) applying the full pipeline except
/// the exception dictionary (to avoid infinite loop from within the prefix
/// recursive call and to allow the prefix to be syllabified normally).
fn syllabify_inner(word: &str) -> Vec<String> {
    // Apply diphthong rule recursively to each inner segment.
    if let Some(dp_len) = leading_diphthong(word) {
        let rest = &word[dp_len..];
        if rest.is_empty() {
            return vec![word.to_string()];
        }
        let dp = word[..dp_len].to_string();
        let mut result = vec![dp];
        result.extend(syllabify_inner(rest));
        return result;
    }
    syllabify_raw(word)
}

/// Inner G2P-based syllabification (no prefix handling). Input must be lowercase.
fn syllabify_raw(word: &str) -> Vec<String> {
    let tokens = analyze(word);

    let nucleus_positions: Vec<usize> = tokens.iter().enumerate()
        .filter_map(|(i, t)| if t.is_nucleus { Some(i) } else { None })
        .collect();

    if nucleus_positions.is_empty() {
        // No vowels at all (e.g. "brr", "w") — return as single syllable.
        return vec![word.to_string()];
    }

    let mut syllable_ranges: Vec<(usize, usize)> = Vec::new();
    let mut syl_start = 0usize;

    for win in nucleus_positions.windows(2) {
        let n1 = win[0];
        let n2 = win[1];

        // Collect only non-skip, non-empty consonant tokens between the two nuclei,
        // remembering their original indices.  Skip tokens (softening i) and empty
        // tokens (the silent output of palatalization) are excluded from the
        // sonority cluster computation but will follow their consonant into whichever
        // syllable it ends up in.
        let inter: Vec<(usize, &str)> = tokens[n1+1..n2]
            .iter()
            .enumerate()
            .filter(|(_, t)| !t.is_skip && !t.ortho.is_empty())
            .map(|(rel, t)| (n1 + 1 + rel, t.ortho.as_str()))
            .collect();

        let cluster_orthos: Vec<&str> = inter.iter().map(|(_, o)| *o).collect();
        let coda_count = coda_len(&cluster_orthos);

        // The split happens right after the last coda consonant token.
        // If coda_count == 0, all consonants go into the onset of n2's syllable,
        // so split right after n1 (the previous nucleus).
        let split_tok = if coda_count == 0 {
            n1 + 1
        } else {
            inter[coda_count - 1].0 + 1
        };

        syllable_ranges.push((syl_start, split_tok));
        syl_start = split_tok;
    }
    syllable_ranges.push((syl_start, tokens.len()));

    syllable_ranges.into_iter().map(|(start, end)| {
        tokens[start..end].iter().map(|t| t.ortho.as_str()).collect()
    }).collect()
}

/// Count the syllables in a Polish word (always >= 1).
pub fn count_syllables(word: &str) -> usize {
    syllabify(word).len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_token_output() {
        let words = &["siebie", "biblioteka", "osiol", "gdzie", "byliście"];
        for w in words {
            let tok = tokenize_and_palatalize(*w);
            let analyzed = analyze(*w);
            let nuc: Vec<&str> = analyzed.iter().filter(|t| t.is_nucleus).map(|t| t.ortho.as_str()).collect();
            println!("{w}: tokens={tok:?}  nuclei={nuc:?}  syllables={:?}", syllabify(*w));
        }
    }

    #[test]
    fn test_basic_counts() {
        assert_eq!(count_syllables("kot"), 1);
        assert_eq!(count_syllables("kota"), 2);
        assert_eq!(count_syllables("muzyka"), 3);
        assert_eq!(count_syllables("polityka"), 4);
        assert_eq!(count_syllables("prezydent"), 3);
        assert_eq!(count_syllables("fizyka"), 3);
        assert_eq!(count_syllables("matematyka"), 5);
    }

    #[test]
    fn test_i_softener_not_a_nucleus() {
        assert_eq!(count_syllables("siebie"), 2);
        assert_eq!(count_syllables("dzieci"), 2);
        assert_eq!(count_syllables("ciasto"), 2);
        assert_eq!(count_syllables("miasto"), 2);
        assert_eq!(count_syllables("niebo"), 2);
        assert_eq!(count_syllables("osioł"), 2);
        assert_eq!(count_syllables("piasek"), 2);
        assert_eq!(count_syllables("ziemia"), 2);
        assert_eq!(count_syllables("powiedzieć"), 3);
        assert_eq!(count_syllables("zrozumieć"), 3);
        assert_eq!(count_syllables("przyjaciel"), 3);
        assert_eq!(count_syllables("chodziliście"), 4);
        assert_eq!(count_syllables("widzieliście"), 4);
        assert_eq!(count_syllables("zrobiliście"),  4);
        assert_eq!(count_syllables("jeździe"),      2);
        assert_eq!(count_syllables("gościem"),      2);
        assert_eq!(count_syllables("gdzie"),        1);
    }

    #[test]
    fn test_past_plural_syllable_counts() {
        assert_eq!(count_syllables("chodziliście"), 4);
        assert_eq!(count_syllables("widzieliście"), 4);
        assert_eq!(count_syllables("zrobiliście"),  4);
        assert_eq!(count_syllables("powiedzieliście"), 5);
        assert_eq!(count_syllables("przyszliście"), 3);
        assert_eq!(count_syllables("zrobiliśmy"),   4);
        assert_eq!(count_syllables("chodziliśmy"),  4);
        assert_eq!(count_syllables("powiedzieliśmy"), 5);
        assert_eq!(count_syllables("zrobiłyście"),  4);
        assert_eq!(count_syllables("zrobiłyśmy"),   4);
        assert_eq!(count_syllables("zrobilibyście"), 5);
        assert_eq!(count_syllables("zrobilibyśmy"),  5);
        assert_eq!(count_syllables("jeździe"),  2);
        assert_eq!(count_syllables("gościem"),  2);
        assert_eq!(count_syllables("wszędzie"), 2);
        assert_eq!(count_syllables("podróżnik"), 3);
    }

    #[test]
    fn test_minimum_one() {
        assert_eq!(count_syllables("brr"), 1);
        assert_eq!(count_syllables("w"), 1);
    }

    #[test]
    fn test_basic_splits() {
        assert_eq!(syllabify("mama"),   vec!["ma", "ma"]);
        assert_eq!(syllabify("oko"),    vec!["o", "ko"]);
        assert_eq!(syllabify("ryba"),   vec!["ry", "ba"]);
        assert_eq!(syllabify("siebie"), vec!["sie", "bie"]);
        assert_eq!(syllabify("niebo"),  vec!["nie", "bo"]);
        assert_eq!(syllabify("ciasto"), vec!["cia", "sto"]);
        assert_eq!(syllabify("szkoła"), vec!["szko", "ła"]);
        assert_eq!(syllabify("rozmowa"), vec!["roz", "mo", "wa"]);
        assert_eq!(syllabify("matematyka"), vec!["ma", "te", "ma", "ty", "ka"]);
    }

    #[test]
    fn test_prefix_morphological_splits() {
        // Śledziński (2018) §3.2 — morphological prefix rules override phonology.
        // These come from syllabificationRules.md Rule 5 and article examples.
        assert_eq!(syllabify("rozmowa"),      vec!["roz", "mo", "wa"]);
        assert_eq!(syllabify("rozkaz"),       vec!["roz", "kaz"]);
        assert_eq!(syllabify("nadlecieć"),    vec!["nad", "le", "cieć"]);
        assert_eq!(syllabify("podejście"),    vec!["pod", "ej", "ście"]);
        assert_eq!(syllabify("odejść"),       vec!["od", "ejść"]);
        assert_eq!(syllabify("bezpośredni"),  vec!["bez", "po", "śred", "ni"]);
        assert_eq!(syllabify("przedszkole"),  vec!["przed", "szko", "le"]);
        assert_eq!(syllabify("obmyślić"),     vec!["ob", "my", "ślić"]);
        assert_eq!(syllabify("dostarczyć"),   vec!["do", "star", "czyć"]);
        assert_eq!(syllabify("podjazd"),      vec!["pod", "jazd"]);
        assert_eq!(count_syllables("dostudzić"),  3); // do|stu|dzić
        assert_eq!(count_syllables("nadworny"),   3); // nad|wor|ny
    }

    #[test]
    fn test_gdzie_one_syllable() {
        assert_eq!(count_syllables("gdzie"), 1);
        assert_eq!(syllabify("gdzie"), vec!["gdzie"]);
    }

    #[test]
    fn test_byliście_three_syllables() {
        assert_eq!(count_syllables("byliście"), 3);
    }
}