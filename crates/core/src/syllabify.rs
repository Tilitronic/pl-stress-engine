use hyphenation::{Hyphenator, Language, Load, Standard};
use once_cell::sync::Lazy;

static HYPHENATOR: Lazy<Standard> = Lazy::new(|| {
    Standard::from_embedded(Language::Polish)
        .expect("Polish hyphenation patterns not found; ensure 'embed_all' feature is enabled")
});

// ---------------------------------------------------------------------------
// Vowel-nucleus detection
// ---------------------------------------------------------------------------

/// Vowels that always form a syllable nucleus in Polish.
const STRONG_VOWELS: &[char] = &['a', 'e', 'o', 'u', 'ó', 'ą', 'ę', 'y'];

fn is_strong_vowel(c: char) -> bool {
    STRONG_VOWELS.contains(&c)
}

fn is_vowel(c: char) -> bool {
    c == 'i' || is_strong_vowel(c)
}

/// Return char-indices of vowel nuclei within `chars`.
///
/// 'i' is treated as a palatalisation marker (non-nucleus) when it is
/// immediately preceded by a consonant AND immediately followed by a strong
/// vowel — the pattern underlying nia/sia/cia/mia/wia/bia/pia … etc.
fn find_nuclei(chars: &[char]) -> Vec<usize> {
    let mut nuclei = Vec::new();
    for (pos, &c) in chars.iter().enumerate() {
        if is_strong_vowel(c) {
            nuclei.push(pos);
        } else if c == 'i' {
            let prev_consonant = pos > 0 && !is_vowel(chars[pos - 1]);
            let next_strong = pos + 1 < chars.len() && is_strong_vowel(chars[pos + 1]);
            if !(prev_consonant && next_strong) {
                nuclei.push(pos);
            }
        }
    }
    nuclei
}

// ---------------------------------------------------------------------------
// Onset table & coda length
// ---------------------------------------------------------------------------

/// 2-character consonant sequences that can open a Polish syllable.
const VALID_2_CHAR_ONSETS: &[&str] = &[
    // liquid / nasal + obstruent
    "bl", "br", "dl", "dr", "fl", "fr", "gl", "gr", "kl", "kn", "kr",
    "mn", "pl", "pr", "tr", "wr",
    // nasal clusters
    "gn", "pn", "sn", "zn", "zm",
    // sibilant + consonant
    "sk", "sl", "sm", "sp", "st", "sw",
    "śl", "śm", "śn", "śp", "śr", "śt", "św",
    "zb", "zd", "zg", "zl", "zr", "zw",
    // labiovelar
    "kw", "tw", "gw",
    // digraphs acting as single phonemes
    "ch", "cz", "dz", "rz", "sz",
    // misc
    "ps", "ćm",
];

/// How many consonants from the *left* of `cluster` belong to the coda.
/// The remainder form the onset of the following syllable.
fn coda_len(cluster: &[char]) -> usize {
    match cluster.len() {
        0 => 0,
        1 => 0, // single consonant always goes with the next nucleus
        2 => {
            let s: String = cluster.iter().collect();
            if VALID_2_CHAR_ONSETS.contains(&s.as_str()) { 0 } else { 1 }
        }
        n => {
            let last2: String = cluster[n - 2..].iter().collect();
            if VALID_2_CHAR_ONSETS.contains(&last2.as_str()) { n - 2 } else { n - 1 }
        }
    }
}

// ---------------------------------------------------------------------------
// Post-processor
// ---------------------------------------------------------------------------

/// If a hyphenation chunk has more than one vowel nucleus, resplit it.
///
/// This corrects cases where TeX line-break patterns leave multiple nuclei in
/// one chunk — e.g. "okno" (returned as a single chunk by hyphenation) has
/// nuclei [o, o] and becomes ["o", "kno"].
fn resplit_chunk(chunk: &str) -> Vec<String> {
    let chars: Vec<char> = chunk.chars().collect();
    let nuclei = find_nuclei(&chars);
    if nuclei.len() <= 1 {
        return vec![chunk.to_string()];
    }

    let mut result: Vec<String> = Vec::new();
    let mut start = 0usize;

    for i in 0..nuclei.len() - 1 {
        let n1 = nuclei[i];
        let n2 = nuclei[i + 1];
        let cluster = &chars[n1 + 1..n2];
        let split = n1 + 1 + coda_len(cluster);
        result.push(chars[start..split].iter().collect());
        start = split;
    }
    result.push(chars[start..].iter().collect());
    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Split a Polish word into syllables.
///
/// Uses embedded Polish TeX/LibreOffice hyphenation patterns as a first pass,
/// then applies a vowel-nucleus post-processor that fixes chunks where the
/// typographic break rules under-split (e.g. "okno" → ["o","kno"]).
pub fn syllabify(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let hyphenated = HYPHENATOR.hyphenate(&lower);
    let breaks = &hyphenated.breaks;

    let chunks: Vec<String> = if breaks.is_empty() {
        vec![lower]
    } else {
        let mut v = Vec::with_capacity(breaks.len() + 1);
        let mut prev = 0;
        for &b in breaks.iter() {
            v.push(lower[prev..b].to_string());
            prev = b;
        }
        v.push(lower[prev..].to_string());
        v
    };

    chunks.into_iter().flat_map(|c| resplit_chunk(&c)).collect()
}

/// Count the syllables in a Polish word (≥ 1).
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
    fn test_basic_counts() {
        assert_eq!(count_syllables("kot"), 1);
        assert_eq!(count_syllables("kota"), 2);
        assert_eq!(count_syllables("muzyka"), 3);
        assert_eq!(count_syllables("polityka"), 4);
        assert_eq!(count_syllables("prezydent"), 3);
        assert_eq!(count_syllables("czterysta"), 3);
        assert_eq!(count_syllables("siedemset"), 3);
        assert_eq!(count_syllables("fizyka"), 3);
        assert_eq!(count_syllables("matematyka"), 5);
        assert_eq!(count_syllables("informatyka"), 5);
        assert_eq!(count_syllables("gramatyka"), 4);
    }

    /// Words that hyphenation under-splits; the post-processor must fix them.
    #[test]
    fn test_postprocessor_fixes() {
        // "okno" returned as single chunk by hyphenation; must become ["o","kno"]
        assert_eq!(count_syllables("okno"), 2);
        assert_eq!(syllabify("okno"), vec!["o", "kno"]);

        // "aryt" chunk in arytmetyka must resplit to ["a","ryt"]
        assert_eq!(count_syllables("arytmetyka"), 5);
        assert_eq!(syllabify("arytmetyka"), vec!["a", "ryt", "me", "ty", "ka"]);

        // "uni" chunk in uniwersytet must resplit to ["u","ni"]
        assert_eq!(count_syllables("uniwersytet"), 5);
        assert_eq!(syllabify("uniwersytet"), vec!["u", "ni", "wer", "sy", "tet"]);
    }

    /// 'i' as palatalisation marker must NOT be counted as a nucleus.
    /// These all have fewer syllables than naive vowel-counting would suggest.
    #[test]
    fn test_i_softener_not_a_nucleus() {
        assert_eq!(count_syllables("siebie"), 2);    // sie-bie
        assert_eq!(count_syllables("dzieci"), 2);    // dzie-ci
        assert_eq!(count_syllables("ciasto"), 2);    // cia-sto
        assert_eq!(count_syllables("miasto"), 2);    // mia-sto
        assert_eq!(count_syllables("niebo"), 2);     // nie-bo
        assert_eq!(count_syllables("piasek"), 2);    // pia-sek
        assert_eq!(count_syllables("ziemia"), 2);    // zie-mia
        assert_eq!(count_syllables("powiedzieć"), 3); // po-wie-dzieć
        assert_eq!(count_syllables("zrozumieć"), 3);  // zro-zu-mieć
        assert_eq!(count_syllables("przyjaciel"), 3); // przy-ja-ciel
    }

    #[test]
    fn test_split_shapes() {
        assert_eq!(syllabify("matematyka"), vec!["ma", "te", "ma", "ty", "ka"]);
        assert_eq!(syllabify("siebie"), vec!["sie", "bie"]);
        assert_eq!(syllabify("rozmowa"), vec!["roz", "mo", "wa"]);
        assert_eq!(syllabify("piszemy"), vec!["pi", "sze", "my"]);
    }

    #[test]
    fn test_minimum_one() {
        assert_eq!(count_syllables("brr"), 1);
        assert_eq!(count_syllables("w"), 1); // no vowel → returned as single chunk
    }
}
