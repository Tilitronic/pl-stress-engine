use pl_stress_core::{Confidence, StressDict};
use wasm_bindgen::prelude::*;

static EXCEPTIONS_BIN: &[u8] = include_bytes!("../../../data/processed/exceptions.bin");

thread_local! {
    static DICT: StressDict = StressDict::from_bytes(EXCEPTIONS_BIN)
        .expect("Failed to load embedded stress dictionary");
}

/// Returns the 0-based syllable index of the stressed syllable.
///
/// ```js
/// stress("matematyka") // → 2  (ma·te·[ma]·ty·ka)
/// ```
#[wasm_bindgen]
pub fn stress(word: &str) -> usize {
    DICT.with(|d| d.stress_index(word))
}

/// Returns a JS object with full stress information:
///
/// ```json
/// {
///   "word":          "zrobiliśmy",
///   "syllables":     ["zro", "bi", "li", "śmy"],
///   "syllableIndex": 2,
///   "stressFromEnd": 2,
///   "ipa":           "zrɔˈbʲiliɕmɨ",   // null if not in dictionary
///   "confidence":    "exact"            // "exact" | "rule" | "default"
/// }
/// ```
#[wasm_bindgen(js_name = stressInfo)]
pub fn stress_info(word: &str) -> js_sys::Object {
    let obj = js_sys::Object::new();
    DICT.with(|d| {
        let r = d.stress(word);
        let conf = match r.confidence {
            Confidence::Exact   => "exact",
            Confidence::Rule    => "rule",
            Confidence::Default => "default",
        };
        let syllables_arr = js_sys::Array::new();
        for s in &r.syllables {
            syllables_arr.push(&JsValue::from_str(s));
        }
        macro_rules! set {
            ($key:expr, $val:expr) => {
                js_sys::Reflect::set(&obj, &$key.into(), &$val).unwrap();
            };
        }
        set!("word",          JsValue::from_str(word));
        set!("syllables",     syllables_arr.into());
        set!("syllableIndex", JsValue::from(r.syllable_index as u32));
        set!("stressFromEnd", JsValue::from(r.stress_from_end() as u32));
        set!("ipa",              r.ipa.as_deref().map(JsValue::from_str)
                                     .unwrap_or(JsValue::NULL));
        set!("ipaTranscribed",   JsValue::from_str(&r.ipa_transcribed()));
        set!("confidence",       JsValue::from_str(conf));
    });
    obj
}

