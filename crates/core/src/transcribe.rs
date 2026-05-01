//! Polish grapheme-to-IPA transcription engine — explicit-pass pipeline.
//!
//! Rules sourced from:
//!   [A1] Wągiel, M. – "Międzynarodowy alfabet fonetyczny (IPA)
//!        w transkrypcji fonetycznej języka polskiego"
//!   [A2] Nagórko, A. – "Podręczna gramatyka języka polskiego"
//!        (chapter: Fonetyka i sylaby)
//!
//! Architecture (Ukrainian pipeline style) — four explicit, named passes:
//!
//!   tokenize → palatalize → nasals → voice → compose
//!
//! Each pass operates on a `Vec<Token>` intermediate representation.
//! The public API `pub fn transcribe(syllables, stress_idx)` is unchanged.

// ── token type ────────────────────────────────────────────────────────────────

mod token {
    /// A single orthographic unit (char, digraph, or trigraph) with a slot for
    /// the IPA string filled in progressively by each pass.
    #[derive(Debug, Clone)]
    pub(crate) struct Token {
        /// Original orthographic chars for this unit (may be 2–3 chars for digraphs/trigraphs).
        pub ortho: String,
        /// Resolved IPA string. `None` = not yet resolved by any pass.
        pub ipa: Option<String>,
        /// Set by the palatalize pass when this token is a consumed softening `i`
        /// (not a nucleus). Skipped by all subsequent passes and by compose.
        pub is_skip: bool,
    }

    impl Token {
        /// Ordinary token from a grapheme unit (ipa will be filled by a later pass).
        pub fn new(ortho: impl Into<String>) -> Self {
            Token { ortho: ortho.into(), ipa: None, is_skip: false }
        }
        /// Synthetic token (e.g. the async [j] glide inserted by palatalize).
        /// Has no ortho — it is ignored by context-lookahead helpers.
        pub fn synthetic(ipa: impl Into<String>) -> Self {
            Token { ortho: String::new(), ipa: Some(ipa.into()), is_skip: false }
        }
        /// First ortho char, or `'\0'` for empty / synthetic tokens.
        pub fn first(&self) -> char {
            self.ortho.chars().next().unwrap_or('\0')
        }
    }

    /// First ortho char of the next non-skip, non-synthetic token after `idx`,
    /// falling back to the first char of the next syllable (`fallback`).
    ///
    /// Used by all passes for cross-token context (voicing, nasal assimilation…).
    pub(crate) fn next_ortho(tokens: &[Token], idx: usize, fallback: Option<char>) -> Option<char> {
        tokens[idx + 1..]
            .iter()
            .filter(|t| !t.is_skip && !t.ortho.is_empty())
            .next()
            .and_then(|t| t.ortho.chars().next())
            .or(fallback)
    }
}

// ── shared phoneme helpers ────────────────────────────────────────────────────

fn is_strong_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'o' | 'u' | 'y' | 'ą' | 'ę' | 'ó')
}

fn is_voiceless_obstruent_start(c: char) -> bool {
    matches!(c, 'p' | 't' | 'k' | 'f' | 's' | 'c' | 'h' | 'ś' | 'ć')
}

fn is_voiced_obstruent_start(c: char) -> bool {
    matches!(c, 'b' | 'd' | 'g' | 'w' | 'z' | 'ż' | 'ź')
}

fn should_devoice(next: Option<char>) -> bool {
    match next {
        None => true,
        Some(c) => is_voiceless_obstruent_start(c),
    }
}

fn should_voice(next: Option<char>) -> bool {
    next.is_some_and(is_voiced_obstruent_start)
}

// ── palatalization helper ─────────────────────────────────────────────────────

/// `(ipa_string, needs_async_j_glide)` for a grapheme unit (ortho) softened by `i + StrongVowel`.
///
/// `needs_async_j_glide = false`: inherently palatal/alveopalatal output — absorbs the [j] element.
/// `needs_async_j_glide = true`: carries `ʲ` superscript + an extra asynchronous [j] glide [A1 §3.3].
fn soften(ortho: &str) -> Option<(&'static str, bool)> {
    match ortho {
        // Inherent alveopalatal — [j] absorbed into the articulation:
        "n"        => Some(("ɲ",   false)),
        "s"        => Some(("ɕ",   false)),
        "z"        => Some(("ʑ",   false)),
        "c"        => Some(("t͡ɕ",  false)),
        // Asynchronous labials — separate [j] glide required [A1 §3.3]:
        "p"        => Some(("pʲ",  true)),
        "b"        => Some(("bʲ",  true)),
        "m"        => Some(("mʲ",  true)),
        "f"        => Some(("fʲ",  true)),
        "w"        => Some(("vʲ",  true)),  // w → v (labiodental), then palatalized
        // Post-palatal (velar) — [A1 §3.3: post-palatal velars produce [kj], [ɡj], [xj]]:
        "h" | "ch" => Some(("xʲ",  true)),
        "k"        => Some(("kʲ",  true)),
        "g"        => Some(("ɡʲ",  true)),
        // Asynchronous alveolars:
        "t"        => Some(("tʲ",  true)),
        "d"        => Some(("dʲ",  true)),
        "r"        => Some(("rʲ",  true)),
        "l"        => Some(("lʲ",  true)),
        _          => None,
    }
}

// ── Pass 1 — tokenize ─────────────────────────────────────────────────────────
//
// Splits a lowercased syllable chunk into `Token`s.
// Trigraphs are matched before digraphs; digraphs before single chars.
// No IPA is emitted here — tokens carry only their ortho strings.

mod pass_tokenize {
    use super::token::Token;

    pub(super) fn run(chunk: &str) -> Vec<Token> {
        let chars: Vec<char> = chunk.chars().collect();
        let n = chars.len();
        let mut tokens = Vec::with_capacity(n);
        let mut i = 0;
        while i < n {
            // 3-char trigraphs (checked before digraphs):
            if i + 2 < n {
                match (chars[i], chars[i + 1], chars[i + 2]) {
                    ('t', 'r', 'z') => { tokens.push(Token::new("trz")); i += 3; continue; }
                    ('d', 'r', 'z') => { tokens.push(Token::new("drz")); i += 3; continue; }
                    _ => {}
                }
            }
            // 2-char digraphs:
            if i + 1 < n {
                match (chars[i], chars[i + 1]) {
                    ('s', 'z') => { tokens.push(Token::new("sz"));  i += 2; continue; }
                    ('c', 'z') => { tokens.push(Token::new("cz"));  i += 2; continue; }
                    ('c', 'h') => { tokens.push(Token::new("ch"));  i += 2; continue; }
                    ('r', 'z') => { tokens.push(Token::new("rz"));  i += 2; continue; }
                    ('d', 'ż') => { tokens.push(Token::new("dż"));  i += 2; continue; }
                    ('d', 'ź') => { tokens.push(Token::new("dź"));  i += 2; continue; }
                    ('d', 'z') => { tokens.push(Token::new("dz"));  i += 2; continue; }
                    ('ś', 'ć') => { tokens.push(Token::new("ść")); i += 2; continue; }
                    _ => {}
                }
            }
            // Single character:
            tokens.push(Token::new(chars[i].to_string()));
            i += 1;
        }
        tokens
    }
}

// ── Pass 2 — palatalize ───────────────────────────────────────────────────────
//
// Detects C + i + StrongVowel (Case A) and C + i + Nucleus (Case B) patterns.
//
// Case A: 'i' is a palatalizing marker (not a nucleus).
//   - The consonant token receives its palatalized IPA.
//   - The 'i' token is marked is_skip = true.
//   - If the consonant articulation is asynchronous (needs_j_glide), a synthetic
//     [j] token is inserted immediately after the skipped 'i'.
//
// Case B: 'i' is a nucleus (no following strong vowel).
//   - The consonant receives its palatalized IPA (no j glide).
//   - The 'i' token stays as a regular nucleus to be resolved by the voice pass.
//
// Special case: `dz + i + StrongVowel` → inherently palatalized [d͡ʑ] (no j).

mod pass_palatalize {
    use super::{token::Token, is_strong_vowel, soften};

    pub(super) fn run(tokens: &mut Vec<Token>, fallback_next: Option<char>) {
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i].is_skip || tokens[i].ipa.is_some() {
                i += 1;
                continue;
            }
            // Is the immediately next token the softening 'i'?
            if i + 1 < tokens.len() && tokens[i + 1].ortho == "i" && !tokens[i + 1].is_skip {
                let after_i: Option<char> = tokens
                    .get(i + 2)
                    .map(|t| t.first())
                    .filter(|&c| c != '\0')
                    .or(fallback_next);

                if after_i.is_some_and(is_strong_vowel) {
                    // ── Case A: C + i + StrongVowel ──────────────────────
                    // Special: dz + i + V → inherently palatalized [d͡ʑ] (no j glide).
                    if tokens[i].ortho == "dz" {
                        tokens[i].ipa = Some("d͡ʑ".to_string());
                        tokens[i + 1].is_skip = true;
                        i += 1;
                        continue;
                    }
                    if let Some((pal, needs_j)) = soften(&tokens[i].ortho) {
                        tokens[i].ipa = Some(pal.to_string());
                        tokens[i + 1].is_skip = true;
                        if needs_j {
                            // Insert async [j] glide right after the skipped 'i'.
                            tokens.insert(i + 2, Token::synthetic("j"));
                        }
                        i += 1;
                        continue;
                    }
                } else {
                    // ── Case B: C + i (nucleus) ───────────────────────────
                    // Consonant is palatalized; 'i' remains as the nucleus.
                    if let Some((pal, _)) = soften(&tokens[i].ortho) {
                        tokens[i].ipa = Some(pal.to_string());
                        // Do NOT skip the 'i' token — it is the syllable nucleus.
                        i += 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
    }
}

// ── Pass 3 — nasals ───────────────────────────────────────────────────────────
//
// Resolves nasal vowels (ą, ę) and the nasal consonant n in context.
// Context is supplied by `next_ortho` which looks past is_skip / synthetic tokens.
//
// Accuracy note: word-final ę (next_c = None) is realized as plain [ɛ]
// in contemporary Polish (the labio-velar glide component is lost).
// Reference: Biedrzycki 1963, Wierzchowska 1971 (cited in [A1 §3.2]).

mod pass_nasals {
    use super::token::{Token, next_ortho};

    pub(super) fn run(tokens: &mut Vec<Token>, fallback_next: Option<char>) {
        for i in 0..tokens.len() {
            if tokens[i].is_skip || tokens[i].ipa.is_some() {
                continue;
            }
            let next_c = next_ortho(tokens, i, fallback_next);
            let ipa = match tokens[i].ortho.as_str() {
                "ą" => nasal_a(next_c),
                "ę" => nasal_e(next_c),
                "n" => {
                    // Velar nasal assimilation before k/g [A2: bank→bäŋk].
                    // Works across syllable boundaries via fallback_next.
                    if matches!(next_c, Some('k') | Some('g')) { "ŋ" } else { continue }
                }
                _ => continue,
            };
            tokens[i].ipa = Some(ipa.to_string());
        }
    }

    /// Allophone of ą based on following context [A1 §3.2, A2].
    fn nasal_a(next: Option<char>) -> &'static str {
        match next {
            // Before palatal Cs → palatal nasal glide [ɔj̃]:
            Some('ć') | Some('ś') | Some('ź') | Some('ń') | Some('j') => "ɔj̃",
            // Before labial stops → bilabial nasal assimilation [ɔm]:
            Some('p') | Some('b') => "ɔm",
            // Before alveolar/dental stops → alveolar nasal [ɔn]:
            Some('t') | Some('d') | Some('c') | Some('n') => "ɔn",
            // Before velar stops → velar nasal [ɔŋ]:
            Some('k') | Some('g') => "ɔŋ",
            // Default (fricatives, ł, finally) → labio-velar diphthong [ɔw̃]:
            _ => "ɔw̃",
        }
    }

    /// Allophone of ę based on following context [A1 §3.2, A2].
    fn nasal_e(next: Option<char>) -> &'static str {
        match next {
            Some('ć') | Some('ś') | Some('ź') | Some('ń') | Some('j') => "ɛj̃",
            Some('p') | Some('b') => "ɛm",
            Some('t') | Some('d') | Some('c') | Some('n') => "ɛn",
            Some('k') | Some('g') => "ɛŋ",
            // Default (fricatives, ł, finally): labio-velar nasal diphthong [ɛw̃].
            // ę is NEVER plain [ɛ] — it always carries the nasal component [A1 §3.2].
            _ => "ɛw̃",
        }
    }
}

// ── Pass 4 — voice ────────────────────────────────────────────────────────────
//
// Resolves every token that still has ipa = None (i.e., not touched by palatalize
// or nasals).  Applies regressive voicing/devoicing assimilation using the ortho
// char of the next non-skip, non-synthetic token (or the first char of the next
// syllable via fallback_next).

mod pass_voice {
    use super::token::{Token, next_ortho};
    use super::{should_devoice, should_voice};

    pub(super) fn run(tokens: &mut Vec<Token>, fallback_next: Option<char>) {
        for i in 0..tokens.len() {
            if tokens[i].is_skip || tokens[i].ipa.is_some() {
                continue;
            }
            let next_c = next_ortho(tokens, i, fallback_next);
            tokens[i].ipa = Some(resolve(&tokens[i].ortho, next_c).to_string());
        }
    }

    fn resolve(ortho: &str, next: Option<char>) -> &'static str {
        match ortho {
            // ── Trigraphs ──────────────────────────────────────────────
            "trz" => "ʈʂ",  // retroflex stop + fricative, no tie bar [A1 §3.3]
            "drz" => if should_devoice(next) { "ʈʂ" } else { "ɖʐ" },

            // ── Digraphs — retroflex fricatives [A1 §3.3] ─────────────
            "sz"  => if should_voice(next)   { "ʐ"  } else { "ʂ" },
            "rz"  => if should_devoice(next) { "ʂ"  } else { "ʐ" },

            // ── Digraphs — retroflex affricates ───────────────────────
            "cz"  => "t͡ʂ",
            "dż"  => if should_devoice(next) { "t͡ʂ" } else { "d͡ʐ" },

            // ── Digraphs — alveopalatal affricates ────────────────────
            "dź"  => if should_devoice(next) { "t͡ɕ" } else { "d͡ʑ" },

            // ── Digraphs — alveolar affricate ─────────────────────────
            "dz"  => if should_devoice(next) { "t͡s" } else { "d͡z" },

            // ── Digraphs — velar fricative ─────────────────────────────
            "ch"  => if should_voice(next)   { "ɣ"  } else { "x" },

            // ── Digraph cluster ────────────────────────────────────────
            "ść"  => "ɕt͡ɕ",

            // ── Vowels [A1 §3.1] ───────────────────────────────────────
            "a"   => "ä",   // central open [ä], NOT front [a]
            "e"   => "ɛ",
            "i"   => "i",
            "o"   => "ɔ",
            "ó"   => "u",   // historical merger with u
            "u"   => "u",
            "y"   => "ɨ",   // close central unrounded

            // ── Consonants with voicing alternation ───────────────────
            "b"   => if should_devoice(next) { "p"  } else { "b" },
            "c"   => if should_voice(next)   { "d͡z" } else { "t͡s" },
            "d"   => if should_devoice(next) { "t"  } else { "d" },
            "f"   => if should_voice(next)   { "v"  } else { "f" },
            "g"   => if should_devoice(next) { "k"  } else { "ɡ" },
            "h"   => if should_voice(next)   { "ɣ"  } else { "x" },
            "k"   => if should_voice(next)   { "ɡ"  } else { "k" },
            "p"   => if should_voice(next)   { "b"  } else { "p" },
            "s"   => if should_voice(next)   { "z"  } else { "s" },
            "t"   => if should_voice(next)   { "d"  } else { "t" },
            "w"   => if should_devoice(next) { "f"  } else { "v" },
            "z"   => if should_devoice(next) { "s"  } else { "z" },
            "ć"   => if should_voice(next)   { "d͡ʑ" } else { "t͡ɕ" },
            "ś"   => if should_voice(next)   { "ʑ"  } else { "ɕ" },
            "ź"   => if should_devoice(next) { "ɕ"  } else { "ʑ" },
            "ż"   => if should_devoice(next) { "ʂ"  } else { "ʐ" },

            // ── Consonants without voicing alternation ────────────────
            "j"   => "j",
            "l"   => "l",
            "ł"   => "w",   // labio-velar approximant [A1 table]
            "m"   => "m",
            "n"   => "n",
            "ń"   => "ɲ",
            "r"   => "r",   // alveolar trill

            // Unknown (foreign letters, punctuation, etc.)
            _     => "",
        }
    }
}

// ── Pass 5 — compose ──────────────────────────────────────────────────────────
//
// Joins the ipa strings of all non-skip tokens into the final IPA output.

mod pass_compose {
    use super::token::Token;

    pub(super) fn run(tokens: &[Token]) -> String {
        let mut out = String::new();
        for t in tokens {
            if !t.is_skip {
                if let Some(ipa) = &t.ipa {
                    out.push_str(ipa);
                }
            }
        }
        out
    }
}

// ── public API ────────────────────────────────────────────────────────────────

/// Run tokenize + palatalize passes on a whole lowercased word and return
/// `(ortho, is_skip)` pairs for use by the syllabifier.
///
/// This exposes the G2P front-end so the syllabifier can identify which
/// tokens are softening `i` markers (is_skip = true) versus real nuclei.
pub(crate) fn tokenize_and_palatalize(word: &str) -> Vec<(String, bool)> {
    let mut tokens = pass_tokenize::run(word);
    // No fallback_next across word boundary — whole word is the input.
    pass_palatalize::run(&mut tokens, None);
    tokens.into_iter().map(|t| (t.ortho, t.is_skip)).collect()
}

/// Produce an IPA string for the given syllable sequence.
///
/// A primary-stress mark `ˈ` is inserted before `syllables[stress_idx]`.
/// Single-syllable words receive no stress mark.
///
/// Pipeline per syllable: tokenize → palatalize → nasals → voice → compose.
///
/// ```
/// use pl_stress_core::transcribe;
/// let syls = vec!["ma".to_string(), "ma".to_string()];
/// assert_eq!(transcribe(&syls, 0), "ˈmämä");
/// ```
pub fn transcribe(syllables: &[String], stress_idx: usize) -> String {
    if syllables.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, syl) in syllables.iter().enumerate() {
        // Stress mark before the stressed syllable (only for polysyllabic words).
        if syllables.len() > 1 && i == stress_idx {
            out.push('ˈ');
        }
        // First char of the next syllable — used for cross-boundary assimilation
        // (velar nasal n→ŋ before k/g; obstruent voicing; nasal diphthong context).
        let fallback_next: Option<char> = syllables
            .get(i + 1)
            .and_then(|s| s.chars().next());

        let chunk = syl.to_lowercase();
        let mut tokens = pass_tokenize::run(&chunk);
        pass_palatalize::run(&mut tokens, fallback_next);
        pass_nasals::run(&mut tokens, fallback_next);
        pass_voice::run(&mut tokens, fallback_next);
        out.push_str(&pass_compose::run(&tokens));
    }
    out
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build syllable vec and call transcribe.
    fn t(syllables: &[&str], stress_idx: usize) -> String {
        let syls: Vec<String> = syllables.iter().map(|s| s.to_string()).collect();
        transcribe(&syls, stress_idx)
    }

    // ── V-01: basic vowels [A1 §3.1] ─────────────────────────────────────

    #[test]
    fn v01_a_maps_to_central_ä() {
        // Polish 'a' is central [ä], NOT front [a].
        assert_eq!(t(&["tak"], 0), "täk");
        assert_eq!(t(&["ma", "ma"], 0), "ˈmämä");
    }

    #[test]
    fn v01_vowel_mapping_full() {
        assert_eq!(t(&["ry", "ba"], 0), "ˈrɨbä");      // r=[r], y=ɨ, a=ä
        assert_eq!(t(&["o", "ko"], 0), "ˈɔkɔ");        // o=ɔ
        assert_eq!(t(&["bu", "da"], 0), "ˈbudä");       // u=u
        assert_eq!(t(&["ból"], 0), "bul");              // ó=u
    }

    // ── V-03: nasal diphthongs and assimilation [A1 §3.2, A2] ────────────

    #[test]
    fn v03_nasal_before_fricative_is_diphthong() {
        // wąs: ą before fricative s → [ɔw̃]  [A1 §3.2]
        assert_eq!(t(&["wąs"], 0), "vɔw̃s");
    }

    #[test]
    fn v03_nasal_assimilation_before_stops() {
        // kąt: ą before t (alveolar stop) → [ɔn]  [A2: kąt→kont]
        assert_eq!(t(&["kąt"], 0), "kɔnt");
        // ząb: ą before b (labial stop) → [ɔm]  [A2: dąb→domp]
        assert_eq!(t(&["ząb"], 0), "zɔmp");
        // prąd: ą before d (alveolar stop) + final devoicing d→t
        assert_eq!(t(&["prąd"], 0), "prɔnt");
        // ręka: ę before k (velar stop) → [ɛŋ]
        assert_eq!(t(&["rę", "ka"], 0), "ˈrɛŋkä");
    }

    #[test]
    fn v03_nasal_n_assimilation_velar() {
        // n before k/g → ŋ; works across syllable boundary via fallback_next.
        assert_eq!(t(&["bank"], 0), "bäŋk");
        assert_eq!(t(&["tan", "go"], 0), "ˈtäŋɡɔ");
        assert_eq!(t(&["Kon", "go"], 0), "ˈkɔŋɡɔ");
    }

    // ── C-01: retroflexes [A1 §3.3] ──────────────────────────────────────

    #[test]
    fn c01_sz_rz_are_retroflex() {
        // sz → [ʂ], NOT [ʃ]
        assert_eq!(t(&["szko", "ła"], 0), "ˈʂkɔwä");
        // ż → [ʐ], NOT [ʒ]
        assert_eq!(t(&["ża", "ba"], 0), "ˈʐäbä");
        // rz → [ʐ]
        assert_eq!(t(&["rze", "ka"], 0), "ˈʐɛkä");
    }

    #[test]
    fn c01_cz_is_retroflex_affricate_with_tie_bar() {
        // cz → [t͡ʂ] with tie bar (one phonological unit)
        assert_eq!(t(&["czy"], 0), "t͡ʂɨ");
    }

    #[test]
    fn c01_trz_is_sequence_no_tie_bar() {
        // trz → [ʈʂ] without tie bar: retroflex stop + fricative sequence.
        // [A1 §3.3]: "czy"=[t͡ʂɨ] vs "trzy"=[ʈʂɨ]
        assert_eq!(t(&["trzy"], 0), "ʈʂɨ");
    }

    // ── C-02: affricates [A1 §3.3] ───────────────────────────────────────

    #[test]
    fn c02_affricates_with_tie_bar() {
        assert_eq!(t(&["co"], 0), "t͡sɔ");           // c → t͡s
        assert_eq!(t(&["ćma"], 0), "t͡ɕmä");         // ć → t͡ɕ
        assert_eq!(t(&["dźwig"], 0), "d͡ʑvʲik");     // final devoicing: g -> k
        assert_eq!(t(&["dżun", "gla"], 0), "ˈd͡ʐuŋɡlä"); // dż=d͡ʐ; g in "gla" stays [ɡ]
    }

    #[test]
    fn c02_dzi_before_vowel_softens_to_alveopalatal() {
        // dzi + strong vowel → d͡ʑ  [A1 §3.3]
        assert_eq!(t(&["dzia", "ła"], 0), "ˈd͡ʑäwä");
        assert_eq!(t(&["dziec", "ko"], 0), "ˈd͡ʑɛt͡skɔ");
    }

    // ── C-03: fricatives ──────────────────────────────────────────────────

    #[test]
    fn c03_alveolar_fricatives() {
        assert_eq!(t(&["so", "wa"], 0), "ˈsɔvä");
        assert_eq!(t(&["śnieg"], 0), "ɕɲɛk");       // final devoicing: g -> k
        assert_eq!(t(&["cha", "ta"], 0), "ˈxätä");  // ch=x
        assert_eq!(t(&["ho", "nor"], 0), "ˈxɔnɔr"); // h=x
    }

    // ── C-06: approximants and laterals [A1 table p.10] ──────────────────

    #[test]
    fn c06_l_is_alveolar_lateral_not_labio_velar() {
        assert_eq!(t(&["las"], 0), "läs");           // l=[l]
    }

    #[test]
    fn c06_ł_is_labio_velar_approximant() {
        // ł → [w], NOT dark-l  [A1 table]
        assert_eq!(t(&["ław", "ka"], 0), "ˈwäfkä");
    }

    #[test]
    fn c06_w_is_labiodental_fricative() {
        // w → [v] (labiodental fricative)  [A1 table]
        assert_eq!(t(&["wo", "da"], 0), "ˈvɔdä");
    }

    // ── P-01: asynchronous palatalization [A1 §3.3] ──────────────────────

    #[test]
    fn p01_labial_plus_i_plus_vowel_async() {
        // p + ie: softening i detected → pʲ + j + ɛ
        assert_eq!(t(&["pies"], 0), "pʲjɛs");
        // p + ia: piasek → ["pia","sek"]
        assert_eq!(t(&["pia", "sek"], 0), "ˈpʲjäsɛk");
        // m + ia: miał → ["miał"]
        assert_eq!(t(&["miał"], 0), "mʲjäw");
        // w + ie: wiek
        assert_eq!(t(&["wiek"], 0), "vʲjɛk");
        // b + ie: bieg → ["bieg"]
        assert_eq!(t(&["bieg"], 0), "bʲjɛk");        // final devoicing: g -> k
    }

    // ── P-02: softening i in nia, sia, cia etc. ───────────────────────────

    #[test]
    fn p02_n_softened_by_i_before_vowel() {
        // nie-bo: "nie" chunk → n+i+e → ɲɛ
        assert_eq!(t(&["nie", "bo"], 0), "ˈɲɛbɔ");
        // nia-nia
        assert_eq!(t(&["nia", "nia"], 0), "ˈɲäɲä");
    }

    #[test]
    fn p02_s_z_softened_by_i_before_vowel() {
        // sia-no → ɕä
        assert_eq!(t(&["sia", "no"], 0), "ˈɕänɔ");
        // ziar-no → ʑär-no (z+ia)
        assert_eq!(t(&["ziar", "no"], 0), "ˈʑärnɔ");
    }

    #[test]
    fn p02_c_softened_by_i_becomes_alveopalatal() {
        // cia-ło → t͡ɕä
        assert_eq!(t(&["cia", "ło"], 0), "ˈt͡ɕäwɔ");
        // cie-pli-wy: "cie" → t͡ɕɛ
        // l is before nucleus i (w in "wy" is not a strong vowel) → lʲ, no j glide
        assert_eq!(t(&["cie", "pli", "wy"], 0), "ˈt͡ɕɛplʲivɨ");
    }

    // ── ST-01: stress mark ────────────────────────────────────────────────

    #[test]
    fn st01_stress_mark_placement() {
        // Paroxytone (normal Polish): stress on penultimate = stress_idx=0 for 2-syl
        assert_eq!(t(&["ma", "ma"], 0), "ˈmämä");
        // Antepenultimate: stress_idx=1 for 3-syl word
        assert_eq!(t(&["ry", "su", "nek"], 1), "rɨˈsunɛk");    // r=[r]
        // Oxytone (single syllable — no mark):
        assert_eq!(t(&["tak"], 0), "täk");
    }

    // ── Full word tests ───────────────────────────────────────────────────

    #[test]
    fn full_word_szkoła() {
        assert_eq!(t(&["szko", "ła"], 0), "ˈʂkɔwä");
    }

    #[test]
    fn full_word_biblioteka() {
        // bi → bʲi (b before nucleus i); bli+o → blʲjɔ (l softened by i+o across boundary)
        assert_eq!(t(&["bi", "bli", "o", "te", "ka"], 3), "bʲiblʲjɔˈtɛkä");
    }

    #[test]
    fn full_word_kiedy() {
        // k+ie: k palatalized → kʲ+j+ɛ  [A1 §3.3: post-palatal velars]
        assert_eq!(t(&["kie", "dy"], 0), "ˈkʲjɛdɨ");
    }

    #[test]
    fn full_word_hiacynt() {
        // C-04 example from rules.md: ch before i+a -> post-palatal [xj]
        assert_eq!(t(&["hia", "cynt"], 1), "xʲjäˈt͡sɨnt");
    }

    #[test]
    fn full_word_chata() {
        assert_eq!(t(&["cha", "ta"], 0), "ˈxätä");
    }

    #[test]
    fn md_examples_smoke_coverage_transcribe_non_empty() {
        // Comprehensive coverage list from ignored/phonetic-transcription-rules.md.
        // Many A2 examples are given in mixed/slavic notation and allow variants,
        // so this suite ensures the PL-IPA engine can process every listed word.
        let words = [
            "uiścić", "baba", "jaja", "dąb", "dęby", "prąd", "prądy", "chęć", "kęs", "wącha",
            "lubią", "lubię", "comte", "kąt", "wąs", "oni", "idą", "pies", "bies", "plastik",
            "idiota", "trzy", "odczepić", "drzewo", "podrzucić", "kino", "chianti", "gigant",
            "magia", "zoologia", "giaur", "geniusz", "gest", "kefir", "baum", "guten", "morgen",
            "osm", "rytm", "piosnka", "andrzejki", "kongo", "krwawy", "wiatr", "risotto",
            "grizzly", "cieplny", "pomyślcie", "wykreśl", "lipa", "oblicz", "kolia", "chleb",
            "modrzew", "modrzewia", "honor", "noc", "dzień", "próg", "mów", "wróg", "męka",
            "świętobliwość", "piątek", "państwo", "okrucieństwo", "bank", "tango", "ziemia",
            "armia", "historia", "piasek", "piękny", "dobry", "tak", "ryba", "gęś", "szkoła",
            "czy", "pik", "niebo", "ciało", "kiedy", "ławka", "mama", "dżungla", "dziecko",
            "rzeka", "żaba", "siano", "bieg", "miał", "wiek", "sowa", "śnieg",
        ];

        for word in words {
            let syls = crate::syllabify::syllabify(word);
            let stress_idx = if syls.len() >= 2 { syls.len() - 2 } else { 0 };
            let ipa = transcribe(&syls, stress_idx);
            assert!(!ipa.is_empty(), "IPA should not be empty for {word}");
        }
    }
}
