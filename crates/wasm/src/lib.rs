use pl_stress_core::{StressDict, StressReading, MorphReading};
use wasm_bindgen::prelude::*;

static EXCEPTIONS_BIN: &[u8] = include_bytes!("../../../data/processed/exceptions.bin");

thread_local! {
    static DICT: StressDict = StressDict::from_bytes(EXCEPTIONS_BIN)
        .expect("Failed to load embedded stress dictionary");
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

macro_rules! set {
    ($obj:expr, $key:expr, $val:expr) => {
        js_sys::Reflect::set(&$obj, &JsValue::from_str($key), &$val).unwrap();
    };
}

fn str_array(strings: &[String]) -> JsValue {
    let arr = js_sys::Array::new();
    for s in strings {
        arr.push(&JsValue::from_str(s));
    }
    arr.into()
}

fn morph_to_js(m: &MorphReading) -> JsValue {
    let md = js_sys::Object::new();
    set!(md, "pos", str_array(&m.pos));
    let feats_obj = js_sys::Object::new();
    for (k, vs) in &m.feats {
        set!(feats_obj, k.as_str(), str_array(vs));
    }
    set!(md, "feats", feats_obj.into());
    set!(md, "lemma", m.lemma.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL));
    set!(md, "definition", m.definition.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL));
    md.into()
}

fn reading_to_js(r: &StressReading) -> JsValue {
    let rd = js_sys::Object::new();
    set!(rd, "syllableIndex",  JsValue::from(r.syllable_index as u32));
    set!(rd, "stressFromEnd",  JsValue::from(r.stress_from_end as u32));
    set!(rd, "syllableCount",  JsValue::from(r.syllable_count as u32));
    set!(rd, "form",           JsValue::from_str(&r.form));
    set!(rd, "stressedForm",   JsValue::from_str(&r.stressed_form));
    set!(rd, "wordSyllables",  str_array(&r.word_syllables));
    set!(rd, "ipa",            JsValue::from_str(&r.ipa));
    set!(rd, "ipaSyllables",   str_array(&r.ipa_syllables));
    // tokens — empty for Polish
    set!(rd, "tokens", js_sys::Array::new().into());
    // morph — empty for Polish
    let morph_arr = js_sys::Array::new();
    for m in &r.morph {
        morph_arr.push(&morph_to_js(m));
    }
    set!(rd, "morph", morph_arr.into());
    set!(rd, "confidence", r.confidence.as_deref().map(JsValue::from_str).unwrap_or(JsValue::NULL));
    rd.into()
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
pub fn lookup(word: &str) -> js_sys::Object {
    DICT.with(|d| {
        let result = d.lookup(word);
        let obj = js_sys::Object::new();
        set!(obj, "form", JsValue::from_str(&result.form));
        let readings_arr = js_sys::Array::new();
        for r in &result.readings {
            readings_arr.push(&reading_to_js(r));
        }
        set!(obj, "readings", readings_arr.into());
        obj
    })
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
pub fn lookup_batch(words: &js_sys::Array) -> js_sys::Array {
    DICT.with(|d| {
        let out = js_sys::Array::new();
        for word_val in words.iter() {
            let word = word_val.as_string().unwrap_or_default();
            let result = d.lookup(&word);
            let obj = js_sys::Object::new();
            set!(obj, "form", JsValue::from_str(&result.form));
            let readings_arr = js_sys::Array::new();
            for r in &result.readings {
                readings_arr.push(&reading_to_js(r));
            }
            set!(obj, "readings", readings_arr.into());
            out.push(&obj.into());
        }
        out
    })
}

/// Total number of entries in the embedded exception dictionary.
#[wasm_bindgen(js_name = wordCount)]
pub fn word_count() -> usize {
    DICT.with(|d| d.exceptions_len())
}


