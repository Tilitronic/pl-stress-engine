/* eslint-disable */
/* tslint:disable */
// Generated from crates/wasm/generated/word-lookup-result.schema.json

/**
 * Top-level result of `lookup()`.
 *
 * For Polish this always contains exactly one `StressReading` (Polish stress is near-deterministic).  For Ukrainian there may be multiple readings (heteronyms and variative stress).  `readings` is empty only for words completely absent from all sources.
 */
export interface WordLookupResult {
  /**
   * Normalized query form.
   */
  form: string;
  /**
   * All stress variants.  Always one element for Polish.
   */
  readings: StressReading[];
  [k: string]: unknown;
}
/**
 * One stress variant of a word form.
 *
 * Part of [`WordLookupResult`].  Mirrors `StressReading` in the Ukrainian engine so both engines can be consumed with the same client code.
 */
export interface StressReading {
  /**
   * How the stress was determined: `"exact"` | `"rule"` | `"default"`. `None` for Ukrainian (all entries are confirmed dictionary forms).
   */
  confidence?: string | null;
  /**
   * Normalized (lowercased) input form.
   */
  form: string;
  /**
   * Full IPA string from the G2P pipeline.
   */
  ipa: string;
  /**
   * IPA per syllable.  The stressed syllable is prefixed with `ˈ`. Empty for zero-syllable words.
   */
  ipaSyllables: string[];
  /**
   * Morphological analyses sharing this stress position.  Empty for Polish.
   */
  morph: MorphReading[];
  /**
   * 1-based position from the end (2 = penultimate, 3 = antepenultimate). `0` for zero-syllable words.
   */
  stressFromEnd: number;
  /**
   * Form with a combining acute U+0301 after the stressed vowel. Equal to `form` when there is no vowel to mark.
   */
  stressedForm: string;
  /**
   * Total number of syllables.  `0` for purely consonantal words.
   */
  syllableCount: number;
  /**
   * 0-based index of the stressed syllable from the start of the word. `0` for zero-syllable (purely consonantal) words like "z" or "w".
   */
  syllableIndex: number;
  /**
   * Token-level phonetic detail.  Empty for Polish.
   */
  tokens: string[];
  /**
   * Grapheme syllables, positionally aligned with `ipa_syllables`. Empty for zero-syllable words.
   */
  wordSyllables: string[];
  [k: string]: unknown;
}
/**
 * One morphological reading of a word form.
 *
 * Follows [Universal Dependencies](https://universaldependencies.org/) naming. Polish currently provides no morphological data, so `pos`, `feats`, and `lemma` are always empty / `None` here.  The type is shared with the Ukrainian engine (`ua-stress-engine`) for cross-engine API parity.
 */
export interface MorphReading {
  /**
   * Short sense label from Wiktionary used to disambiguate homographs (e.g. "castle" vs "lock" for Ukrainian «замок»).  Empty for Polish.
   */
  definition?: string | null;
  /**
   * UD feature map, e.g. `{"Case": ["Nom"], "Number": ["Sing"]}`.  Empty for Polish.
   */
  feats: {
    [k: string]: string[];
  };
  /**
   * Base form (lemma).  `None` for Polish.
   */
  lemma?: string | null;
  /**
   * UD POS tags, e.g. `["NOUN"]`.  Empty for Polish.
   */
  pos: string[];
  [k: string]: unknown;
}
