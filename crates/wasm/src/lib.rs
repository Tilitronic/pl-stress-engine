use pl_stress_core::StressDict;
use wasm_bindgen::prelude::*;

static EXCEPTIONS_BIN: &[u8] = include_bytes!("../../../data/processed/exceptions.bin");

thread_local! {
    static DICT: StressDict = StressDict::from_bytes(EXCEPTIONS_BIN)
        .expect("Failed to load embedded stress dictionary");
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn to_js_value<T: serde::Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).expect("failed to serialize value for wasm")
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Look up a Polish word and return its stress analysis.
///
/// Returns a JS object matching the unified cross-engine format:
/// ```json
/// {
///   "form": "matematyka",
///   "readings": [{
///     "syllableIndex": 2,
///     "stressFromEnd": 2,
///     "syllableCount": 5,
///     "form": "matematyka",
///     "stressedForm": "matemátyka",
///     "wordSyllables": ["ma","te","ma","ty","ka"],
///     "ipa": "matɛmatɨka",
///     "ipaSyllables": ["ma","tɛ","ˈma","tɨ","ka"],
///     "tokens": [],
///     "morph": [],
///     "confidence": "rule"
///   }]
/// }
/// ```
#[wasm_bindgen]
pub fn lookup(word: &str) -> JsValue {
    DICT.with(|d| to_js_value(&d.lookup(word)))
}

/// Return the word with a combining acute U+0301 on the stressed vowel.
///
/// Convenience wrapper around `lookup(word).readings[0].stressedForm`.
#[wasm_bindgen]
pub fn mark(word: &str) -> String {
    DICT.with(|d| {
        let result = d.lookup(word);
        match result.readings.first() {
            Some(r) => r.stressed_form.clone(),
            None => word.to_string(),
        }
    })
}

/// Return the 0-based syllable index of the stressed syllable.
///
/// Convenience wrapper for callers that only need the stress position.
#[wasm_bindgen]
pub fn stress(word: &str) -> usize {
    DICT.with(|d| d.stress_index(word))
}

/// Batch variant of `mark`. Takes a JS Array of words and returns a JS Array
/// of stress-marked strings.
///
/// Polish stress is always deterministic — every word is returned marked
/// (either from the exception dictionary or by rule).  Significantly faster
/// than calling `mark()` in a loop because the dictionary lookup overhead is
/// amortised over the whole batch.
///
/// ```js
/// markBatch(['matematyka', 'chodziliście', 'GPS']);
/// // → ['matemátyka', 'chódziliście', 'GPS']
/// ```
#[wasm_bindgen(js_name = markBatch)]
pub fn mark_batch(words: &js_sys::Array) -> js_sys::Array {
    DICT.with(|d| {
        let out = js_sys::Array::new();
        for word_val in words.iter() {
            let word = word_val.as_string().unwrap_or_default();
            let result = d.lookup(&word);
            let marked = match result.readings.first() {
                Some(r) => r.stressed_form.clone(),
                None => word,
            };
            out.push(&JsValue::from_str(&marked));
        }
        out
    })
}

/// Batch variant of `stress`. Takes a JS Array of words and returns an
/// `Int32Array` of 0-based stressed-syllable indices (one per word).
///
/// Always ≥ 0 — Polish stress is deterministic so every word resolves to a
/// position.  Significantly faster than calling `stress()` in a loop.
///
/// ```js
/// stressBatch(['matematyka', 'chodziliście']);
/// // → Int32Array [2, 1]
/// ```
#[wasm_bindgen(js_name = stressBatch)]
pub fn stress_batch(words: &js_sys::Array) -> Box<[i32]> {
    DICT.with(|d| {
        words
            .iter()
            .map(|v| {
                let word = v.as_string().unwrap_or_default();
                d.stress_index(&word) as i32
            })
            .collect::<Vec<i32>>()
            .into_boxed_slice()
    })
}

/// Batch variant of `lookup`. Takes a JS Array of words and returns a JS Array
/// of full lookup result objects (same shape as `lookup()` for each word).
///
/// Significantly faster than calling `lookup()` in a loop.
///
/// ```js
/// const results = lookupBatch(['matematyka', 'chodziliście']);
/// results[0].readings[0].stressedForm; // → 'matemátyka'
/// results[1].readings[0].wordSyllables; // → ['cho','dzi','li','ście']
/// ```
#[wasm_bindgen(js_name = lookupBatch)]
pub fn lookup_batch(words: &js_sys::Array) -> JsValue {
    DICT.with(|d| {
        let results: Vec<_> = words
            .iter()
            .map(|word_val| d.lookup(&word_val.as_string().unwrap_or_default()))
            .collect();
        to_js_value(&results)
    })
}

/// Total number of entries in the embedded exception dictionary.
#[wasm_bindgen(js_name = wordCount)]
pub fn word_count() -> usize {
    DICT.with(|d| d.exceptions_len())
}


