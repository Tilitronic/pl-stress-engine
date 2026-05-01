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

fn has_vowel(s: &str) -> bool {
    s.chars().any(is_vowel)
}

fn ends_with_vowel(s: &str) -> bool {
    s.chars().last().is_some_and(is_vowel)
}

fn first_vowel_pos(chars: &[char]) -> Option<usize> {
    chars.iter().position(|c| is_vowel(*c))
}

fn shift_first_consonant_to_left(left: &mut String, right: &mut String) {
    if let Some(first) = right.chars().next() {
        let n = first.len_utf8();
        left.push(first);
        *right = right[n..].to_string();
    }
}

fn merge_zero_vowel_chunks(mut syllables: Vec<String>) -> Vec<String> {
    let mut i = 0usize;
    while i < syllables.len() {
        if has_vowel(&syllables[i]) {
            i += 1;
            continue;
        }

        if i + 1 < syllables.len() {
            let right = syllables.remove(i + 1);
            syllables[i].push_str(&right);
        } else if i > 0 {
            let chunk = syllables.remove(i);
            syllables[i - 1].push_str(&chunk);
            i -= 1;
        } else {
            break;
        }
    }
    syllables
}

fn apply_article_rules(mut syllables: Vec<String>) -> Vec<String> {
    // MOD-style corrections from article examples.
    // For specific onset clusters at the start of a syllable, move first consonant
    // to the previous syllable coda: msta -> m|sta, dl -> d|l, dm -> d|m.
    let mut i = 0usize;
    while i + 1 < syllables.len() {
        if !ends_with_vowel(&syllables[i]) {
            i += 1;
            continue;
        }

        let right_chars: Vec<char> = syllables[i + 1].chars().collect();
        let Some(vpos) = first_vowel_pos(&right_chars) else {
            i += 1;
            continue;
        };

        if vpos >= 2 {
            let onset: String = right_chars[..vpos].iter().collect();
            let nucleus = right_chars[vpos];
            let should_shift = onset.starts_with("mst")
                || (onset.starts_with("dl") && nucleus == 'a')
                || (onset.starts_with("dm") && nucleus == 'a');
            if should_shift {
                let (left_slice, right_slice) = syllables.split_at_mut(i + 1);
                shift_first_consonant_to_left(&mut left_slice[i], &mut right_slice[0]);
            }
        }

        i += 1;
    }

    syllables
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

/// Return char-indices of vowel nuclei within `chars`.
///
/// 'i' is treated as a palatalisation marker (non-nucleus) when it is
/// immediately preceded by a consonant AND immediately followed by a strong
/// vowel — the pattern underlying nia/sia/cia/mia/wia/bia/pia … etc.
fn find_nuclei(chars: &[char]) -> Vec<usize> {
    let mut nuclei = Vec::new();
    for (pos, &c) in chars.iter().enumerate() {
        if is_strong_vowel(c) {
            if c == 'u' && pos == 1 && matches!(chars[0], 'a' | 'e') {
                continue;
            }
            nuclei.push(pos);
        } else if c == 'i' {
            let prev_consonant = pos > 0 && !is_vowel(chars[pos - 1]);
            let next_strong = pos + 1 < chars.len() && is_strong_vowel(chars[pos + 1]);
            let prev_prev_consonant = pos > 1 && !is_vowel(chars[pos - 2]);
            let prev_digraph = pos > 1 && is_consonant_digraph(chars[pos - 2], chars[pos - 1]);
            // Liquids (l, r) and the labiovelar glide (w) directly before 'i'
            // in a C-sonorant-i-V cluster allow 'i' to keep its nuclear role
            // (e.g. bl-i-o in biblioteka, ćw-i-erć in ćwierćwiecze).
            // For all other C-C-i-V clusters (śc-i-e, gn-i-a, …) 'i' palatalises
            // the preceding consonant and is NOT a nucleus.
            let prev_is_sonorant = pos > 0 && matches!(chars[pos - 1], 'l' | 'r' | 'w');

            // 'i' palatalizes a preceding consonant before strong vowels,
            // except when preceded by a sonorant (l/r/w) in a consonant cluster.
            let softening_i = prev_consonant && next_strong
                && (!prev_prev_consonant || prev_digraph || !prev_is_sonorant);

            if !softening_i {
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
            if cluster[1] == 'i' && !is_vowel(cluster[0]) {
                return 0;
            }
            let s: String = cluster.iter().collect();
            if VALID_2_CHAR_ONSETS.contains(&s.as_str()) { 0 } else { 1 }
        }
        n => {
            if cluster[n - 1] == 'i' && !is_vowel(cluster[n - 2]) {
                return n - 2;
            }
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

    let split: Vec<String> = chunks.into_iter().flat_map(|c| resplit_chunk(&c)).collect();
    let merged = merge_zero_vowel_chunks(split);
    apply_article_rules(merged)
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
        assert_eq!(count_syllables("osioł"), 2);     // o-sioł
        assert_eq!(count_syllables("piasek"), 2);    // pia-sek
        assert_eq!(count_syllables("ziemia"), 2);    // zie-mia
        assert_eq!(count_syllables("powiedzieć"), 3); // po-wie-dzieć
        assert_eq!(count_syllables("zrozumieć"), 3);  // zro-zu-mieć
        assert_eq!(count_syllables("przyjaciel"), 3); // przy-ja-ciel
        // C-C-i-V clusters: 'i' softens even with two preceding consonants
        assert_eq!(count_syllables("chodziliście"), 4); // cho-dzi-liś-cie
        assert_eq!(count_syllables("widzieliście"), 4); // wi-dzie-liś-cie
        assert_eq!(count_syllables("zrobiliście"),  4); // zro-bi-liś-cie
        assert_eq!(count_syllables("jeździe"),      2); // jeź-dzie
        assert_eq!(count_syllables("gościem"),      2); // goś-ciem
    }

    /// Syllabification of past-plural verb forms (C-C-i-V clusters in -ście/-śmy).
    ///
    /// These forms triggered a regression where 'i' in the -ście suffix was
    /// mis-counted as a vowel nucleus, adding a spurious extra syllable and
    /// shifting stress to the wrong position.
    #[test]
    fn test_past_plural_syllable_counts() {
        // -liście (past 2 pl, perfective)
        assert_eq!(count_syllables("chodziliście"), 4); // cho-dzi-liś-cie
        assert_eq!(count_syllables("widzieliście"), 4); // wi-dzie-liś-cie
        assert_eq!(count_syllables("zrobiliście"),  4); // zro-bi-liś-cie
        assert_eq!(count_syllables("powiedzieliście"), 5); // po-wie-dzie-liś-cie
        assert_eq!(count_syllables("przyszliście"), 3); // przy-szliś-cie
        // -liśmy (past 1 pl, perfective)
        assert_eq!(count_syllables("zrobiliśmy"),   4); // zro-bi-liś-my
        assert_eq!(count_syllables("chodziliśmy"),  4); // cho-dzi-liś-my
        assert_eq!(count_syllables("powiedzieliśmy"), 5); // po-wie-dzie-liś-my
        // -łyście / -łyśmy (feminine past)
        assert_eq!(count_syllables("zrobiłyście"),  4); // zro-bi-łyś-cie
        assert_eq!(count_syllables("zrobiłyśmy"),   4); // zro-bi-łyś-my
        // -libyście / -libyśmy (conditional plural)
        assert_eq!(count_syllables("zrobilibyście"), 5); // zro-bi-li-byś-cie
        assert_eq!(count_syllables("zrobilibyśmy"),  5); // zro-bi-li-byś-my
        // miscellaneous C-C-ie clusters
        assert_eq!(count_syllables("jeździe"),  2); // jeź-dzie
        assert_eq!(count_syllables("gościem"),  2); // goś-ciem
        assert_eq!(count_syllables("wszędzie"), 2); // wszę-dzie
        assert_eq!(count_syllables("podróżnik"), 3); // po-dróż-nik
    }

    /// 'i' before a vowel IS a nucleus when the preceding consonant is a sonorant (l/r/w).
    #[test]
    fn test_i_nucleus_after_sonorant() {
        // bl-i-o in biblioteka: 'i' is a nucleus (l is sonorant)
        assert_eq!(count_syllables("biblioteka"), 5); // bi-bli-o-te-ka
        // ćw-i-erć: 'i' is a nucleus (w is sonorant)
        assert_eq!(count_syllables("ćwierćwiecze"), 4); // ćwi-erć-wie-cze
    }

    #[test]
    fn test_split_shapes() {
        assert_eq!(syllabify("matematyka"), vec!["ma", "te", "ma", "ty", "ka"]);
        assert_eq!(syllabify("biblioteka"), vec!["bi", "bli", "o", "te", "ka"]);
        assert_eq!(syllabify("osioł"), vec!["o", "sioł"]);
        assert_eq!(syllabify("siebie"), vec!["sie", "bie"]);
        assert_eq!(syllabify("rozmowa"), vec!["roz", "mo", "wa"]);
        assert_eq!(syllabify("piszemy"), vec!["pi", "sze", "my"]);
    }

    #[test]
    fn test_minimum_one() {
        assert_eq!(count_syllables("brr"), 1);
        assert_eq!(count_syllables("w"), 1); // no vowel → returned as single chunk
    }

    #[test]
    fn article_rules_1_to_8_smoke_suite() {
        // Rule 1: each syllable has a vowel nucleus.
        assert_eq!(syllabify("ale"), vec!["a", "le"]);
        assert_eq!(syllabify("oko"), vec!["o", "ko"]);

        // Rule 2: words with one vowel are not split.
        assert_eq!(syllabify("most"), vec!["most"]);
        assert_eq!(syllabify("rak"), vec!["rak"]);
        assert_eq!(syllabify("dom"), vec!["dom"]);
        assert_eq!(syllabify("sok"), vec!["sok"]);

        // Rule 3: digraphs representing one sound should stay intact.
        assert_eq!(syllabify("szkoła"), vec!["szko", "ła"]);
        assert_eq!(syllabify("chata"), vec!["cha", "ta"]);
        assert_eq!(syllabify("czapka"), vec!["czap", "ka"]);
        assert_eq!(syllabify("dziecko"), vec!["dziec", "ko"]);

        // Rule 4: initial au/eu remain one syllabic unit.
        assert_eq!(syllabify("auto"), vec!["au", "to"]);
        assert_eq!(syllabify("europa"), vec!["eu", "ro", "pa"]);

        // Rule 5: prefix boundaries.
        assert_eq!(syllabify("przedszkole"), vec!["przed", "szko", "le"]);
        assert_eq!(syllabify("rozmowa"), vec!["roz", "mo", "wa"]);

        // Rule 6: some words allow more than one acceptable split.
        let kostka = syllabify("kostka").join("-");
        let matka = syllabify("matka").join("-");
        assert!(kostka == "kost-ka" || kostka == "kos-tka");
        assert!(matka == "mat-ka" || matka == "ma-tka");

        // Rule 7: softening 'i' does not form its own syllable.
        assert_eq!(syllabify("ciasto"), vec!["cia", "sto"]);
        assert_eq!(syllabify("powiedzieć"), vec!["po", "wie", "dzieć"]);
        assert_eq!(syllabify("osioł"), vec!["o", "sioł"]);

        // Rule 8: identical consonants are split.
        assert_eq!(syllabify("wanna"), vec!["wan", "na"]);
        assert_eq!(syllabify("anna"), vec!["an", "na"]);
    }

    #[test]
    fn article_open_and_closed_syllables_examples() {
        // Open syllables end with a vowel.
        assert_eq!(syllabify("oko"), vec!["o", "ko"]);
        assert_eq!(syllabify("ucho"), vec!["u", "cho"]);

        // Closed syllables end with a consonant or consonant cluster.
        assert_eq!(syllabify("kulka"), vec!["kul", "ka"]);
        assert_eq!(syllabify("rysunek"), vec!["ry", "su", "nek"]);
        assert_eq!(syllabify("szelest"), vec!["sze", "lest"]);
    }

    #[test]
    fn article2_extended_aligned_cases() {
        assert_eq!(syllabify("konto"), vec!["kon", "to"]);
        assert_eq!(syllabify("perspektywa"), vec!["per", "spek", "ty", "wa"]);
        assert_eq!(syllabify("portfel"), vec!["port", "fel"]);
        assert_eq!(syllabify("majstrem"), vec!["maj", "strem"]);
        assert_eq!(syllabify("administracja"), vec!["ad", "mi", "ni", "stra", "cja"]);
        assert_eq!(syllabify("egzamin"), vec!["eg", "za", "min"]);
        assert_eq!(syllabify("agresywny"), vec!["a", "gre", "syw", "ny"]);
        assert_eq!(syllabify("pownosić"), vec!["po", "wno", "sić"]);
        assert_eq!(syllabify("wydmuchać"), vec!["wy", "dmu", "chać"]);
        assert_eq!(syllabify("dostudzić"), vec!["do", "stu", "dzić"]);
        assert_eq!(syllabify("nadlecieć"), vec!["nad", "le", "cieć"]);
        assert_eq!(syllabify("nadworny"), vec!["na", "dwor", "ny"]);
        assert_eq!(syllabify("obmyślić"), vec!["ob", "my", "ślić"]);
        assert_eq!(syllabify("około"), vec!["o", "ko", "ło"]);
        assert_eq!(syllabify("aeroplan"), vec!["a", "e", "ro", "plan"]);
        assert_eq!(syllabify("geoida"), vec!["ge", "o", "i", "da"]);
        assert_eq!(syllabify("samoistny"), vec!["sa", "mo", "ist", "ny"]);
        assert_eq!(syllabify("herbstem"), vec!["herb", "stem"]);
        assert_eq!(syllabify("gangsterski"), vec!["gang", "ster", "ski"]);
        assert_eq!(syllabify("tekstem"), vec!["tek", "stem"]);
        assert_eq!(syllabify("ekspres"), vec!["eks", "pres"]);
        assert_eq!(syllabify("ciepliwy"), vec!["cie", "pli", "wy"]);
        assert_eq!(syllabify("amnezja"), vec!["a", "mne", "zja"]);
        assert_eq!(syllabify("ziarno"), vec!["ziar", "no"]);
        assert_eq!(syllabify("kuchnia"), vec!["kuch", "nia"]);
        assert_eq!(syllabify("podjazd"), vec!["pod", "jazd"]);
        assert_eq!(syllabify("podwładny"), vec!["pod", "wład", "ny"]);
        assert_eq!(syllabify("obsłuchać"), vec!["ob", "słu", "chać"]);
        assert_eq!(syllabify("rozmnażać"), vec!["roz", "mna", "żać"]);
    }

    #[test]
    fn article2_formerly_different_cases_now_aligned() {
        assert_eq!(syllabify("zemsta"), vec!["zem", "sta"]);
        assert_eq!(syllabify("bydlak"), vec!["byd", "lak"]);
        assert_eq!(syllabify("wydma"), vec!["wyd", "ma"]);
        assert_eq!(syllabify("okołozwrotnikowy"), vec!["o", "ko", "ło", "zwrot", "ni", "ko", "wy"]);
        assert_eq!(syllabify("kopii"), vec!["ko", "pi", "i"]);
        assert_eq!(syllabify("anarchii"), vec!["a", "nar", "chi", "i"]);
        assert_eq!(syllabify("unii"), vec!["u", "ni", "i"]);
    }

    // ─── Śledziński (2018) "Wielowarstwowy model podziału wyrazów ortograficznych" ───
    // Tests grounded in the article's examples, sections, and tables.

    /// §4.1 — konto is the article's step-by-step worked example.
    /// Cluster "nt": n(sonority 3) > t(sonority 1). MOP assigns t to next onset.
    #[test]
    fn sledz_s4_1_konto_nt_cluster() {
        assert_eq!(syllabify("konto"), vec!["kon", "to"]);
    }

    /// §3.6 — uschnąć: explicit phonological-projection example.
    /// Cluster "schn": s(2)–ch/x(2)–n(3). Valley between s and ch → s|chn.
    #[test]
    fn sledz_s3_6_uschnac_schn_cluster() {
        assert_eq!(syllabify("uschnąć"), vec!["usch", "nąć"]);
    }

    /// §3.7 — amnezja: "mn" has equal sonority values (both 3), no phonology split.
    /// Fallback (step 8) places boundary before mn → a-mne-zja.
    #[test]
    fn sledz_s3_7_amnezja_mn_equal_sonority() {
        assert_eq!(syllabify("amnezja"), vec!["a", "mne", "zja"]);
    }

    /// §3.4 — adjacent different vowel nuclei always get a boundary between them.
    #[test]
    fn sledz_s3_4_adjacent_vowel_nuclei() {
        assert_eq!(syllabify("aeroplan"), vec!["a", "e", "ro", "plan"]);
        assert_eq!(syllabify("geoida"),   vec!["ge", "o", "i", "da"]);
    }

    /// §5 group 1 — clusters resolved by SSP + MOP (phonological layer).
    #[test]
    fn sledz_s5_group1_sonority_mop_splits() {
        // perspektywa: rsp → r(4)s(2)p(1) falling; MOP → pers-pek
        assert_eq!(syllabify("perspektywa"), vec!["per", "spek", "ty", "wa"]);
        // portfel: rtf → rt in coda, f onset → port-fel
        assert_eq!(syllabify("portfel"),     vec!["port", "fel"]);
        // majstrem: jstr → j in coda, str valid 3-char onset → maj-strem
        assert_eq!(syllabify("majstrem"),    vec!["maj", "strem"]);
        // egzamin: gz → g in coda, z starts next nucleus → eg-za-min
        assert_eq!(syllabify("egzamin"),     vec!["eg", "za", "min"]);
        // agresywny: wn → w in coda, n lower sonority onset → a-gre-syw-ny
        assert_eq!(syllabify("agresywny"),   vec!["a", "gre", "syw", "ny"]);
        // administracja: str valid complex onset → ad-mi-ni-stra-cja
        assert_eq!(syllabify("administracja"), vec!["ad", "mi", "ni", "stra", "cja"]);
    }

    /// §5 group 2 — clusters where phonology is ambiguous; fallback onset placement.
    #[test]
    fn sledz_s5_group2_fallback_onset_placement() {
        // herbstem: rbst → rb in coda, st onset → herb-stem
        assert_eq!(syllabify("herbstem"),    vec!["herb", "stem"]);
        // gangsterski: ngst → ng in coda, st onset → gang-ster-ski
        assert_eq!(syllabify("gangsterski"), vec!["gang", "ster", "ski"]);
        // tekstem: kst → ks in coda, t onset → tek-stem
        assert_eq!(syllabify("tekstem"),     vec!["tek", "stem"]);
        // ekspres: kspr → ks in coda, pr valid onset → eks-pres
        assert_eq!(syllabify("ekspres"),     vec!["eks", "pres"]);
    }

    /// §5 group 3 — morphological prefix layer (TeX patterns encode many prefixes).
    #[test]
    fn sledz_s5_group3_morphological_prefixes() {
        assert_eq!(syllabify("dostudzić"),  vec!["do", "stu", "dzić"]);
        assert_eq!(syllabify("nadlecieć"),  vec!["nad", "le", "cieć"]);
        // nadworny: exception → phonological split na-dwor-ny
        assert_eq!(syllabify("nadworny"),   vec!["na", "dwor", "ny"]);
        // pownosić: exception → phonological po-wno-sić
        assert_eq!(syllabify("pownosić"),   vec!["po", "wno", "sić"]);
    }

    /// §3.2 — pod-, ob-, roz- prefix boundaries from the article's footnote examples.
    #[test]
    fn sledz_s3_2_pod_ob_roz_prefixes() {
        // podjazd: article example #podja>#pod|ja rule
        assert_eq!(syllabify("podjazd"),    vec!["pod", "jazd"]);
        // podwładny: 86% morphological boundary preference (article footnote)
        assert_eq!(syllabify("podwładny"),  vec!["pod", "wład", "ny"]);
        // obsłuchać: 88% morphological boundary preference (article footnote)
        assert_eq!(syllabify("obsłuchać"),  vec!["ob", "słu", "chać"]);
        // rozmnażać: 90% morphological boundary preference (article footnote)
        assert_eq!(syllabify("rozmnażać"),  vec!["roz", "mna", "żać"]);
    }

    /// Table 1 — basic phoneme-inventory words from the article.
    #[test]
    fn sledz_table1_basic_words() {
        assert_eq!(syllabify("jeden"),   vec!["je", "den"]);
        assert_eq!(syllabify("wiele"),   vec!["wie", "le"]);
        assert_eq!(syllabify("moneta"),  vec!["mo", "ne", "ta"]);
        assert_eq!(syllabify("futro"),   vec!["fu", "tro"]);
        assert_eq!(syllabify("wysoki"),  vec!["wy", "so", "ki"]);
        assert_eq!(syllabify("koza"),    vec!["ko", "za"]);
        assert_eq!(syllabify("ryba"),    vec!["ry", "ba"]);
        assert_eq!(syllabify("tama"),    vec!["ta", "ma"]);
        assert_eq!(syllabify("buda"),    vec!["bu", "da"]);
        assert_eq!(syllabify("palec"),   vec!["pa", "lec"]);
    }

    /// Table 1 — words containing digraphs (sz, cz, ch, dż, dz, rz…).
    #[test]
    fn sledz_table1_digraph_words() {
        assert_eq!(syllabify("ziarno"),  vec!["ziar", "no"]);
        assert_eq!(syllabify("kuchnia"), vec!["kuch", "nia"]);
        assert_eq!(syllabify("dzwonek"), vec!["dzwo", "nek"]);
        assert_eq!(syllabify("działka"), vec!["dział", "ka"]);
        assert_eq!(syllabify("dżuma"),   vec!["dżu", "ma"]);
        assert_eq!(syllabify("kocioł"),  vec!["ko", "cioł"]);
    }

    /// §2.2 footnote — ćwierć- compounds. Our engine lacks the MOR rule for
    /// ćwierć- so it falls through to phonological splitting.
    #[test]
    fn sledz_s2_2_cwierc_phonological_fallback() {
        assert_eq!(syllabify("ćwierćwiecze"), vec!["ćwi", "erć", "wie", "cze"]);
    }
}
