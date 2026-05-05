use once_cell::sync::Lazy;
use pl_stress_core::{StressDict, WordLookupResult, StressReading, MorphReading};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

static EXCEPTIONS_BIN: &[u8] = include_bytes!("../../../data/processed/exceptions.bin");

static DICT: Lazy<StressDict> = Lazy::new(|| {
    StressDict::from_bytes(EXCEPTIONS_BIN).expect("Failed to load embedded stress dictionary")
});

// ── Serialisation helpers ────────────────────────────────────────────────────

fn morph_to_py<'py>(py: Python<'py>, m: &MorphReading) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("pos", PyList::new_bound(py, &m.pos))?;
    let feats_d = PyDict::new_bound(py);
    for (k, vs) in &m.feats {
        feats_d.set_item(k, PyList::new_bound(py, vs))?;
    }
    d.set_item("feats", feats_d)?;
    d.set_item("lemma", m.lemma.as_deref())?;
    d.set_item("definition", m.definition.as_deref())?;
    Ok(d)
}

fn reading_to_py<'py>(py: Python<'py>, r: &StressReading) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("syllable_index", r.syllable_index)?;
    d.set_item("stress_from_end", r.stress_from_end)?;
    d.set_item("syllable_count", r.syllable_count)?;
    d.set_item("form", &r.form)?;
    d.set_item("stressed_form", &r.stressed_form)?;
    d.set_item("word_syllables", PyList::new_bound(py, &r.word_syllables))?;
    d.set_item("ipa", &r.ipa)?;
    d.set_item("ipa_syllables", PyList::new_bound(py, &r.ipa_syllables))?;
    // Token-level detail — empty for Polish
    d.set_item("tokens", PyList::empty_bound(py))?;
    // Morphology — empty for Polish
    let morph_list = PyList::empty_bound(py);
    for m in &r.morph {
        morph_list.append(morph_to_py(py, m)?)?;
    }
    d.set_item("morph", morph_list)?;
    d.set_item("confidence", r.confidence.as_deref())?;
    Ok(d)
}

fn lookup_result_to_py<'py>(
    py: Python<'py>,
    result: &WordLookupResult,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("form", &result.form)?;
    let readings_list = PyList::empty_bound(py);
    for r in &result.readings {
        readings_list.append(reading_to_py(py, r)?)?;
    }
    d.set_item("readings", readings_list)?;
    Ok(d)
}

// ── Public functions ──────────────────────────────────────────────────────────

/// Look up a Polish word and return its stress analysis.
///
/// Returns a dict matching the unified cross-engine format:
///
/// .. code-block:: python
///
///     {
///       "form": "matematyka",
///       "readings": [
///         {
///           "syllable_index": 2,
///           "stress_from_end": 2,
///           "syllable_count": 5,
///           "form": "matematyka",
///           "stressed_form": "matema\u0301tyka",
///           "word_syllables": ["ma", "te", "ma", "ty", "ka"],
///           "ipa": "matɛmatɨka",
///           "ipa_syllables": ["ma", "tɛ", "ˈma", "tɨ", "ka"],
///           "tokens": [],
///           "morph": [],
///           "confidence": "rule"
///         }
///       ]
///     }
#[pyfunction]
fn lookup(py: Python<'_>, word: &str) -> PyResult<Py<PyDict>> {
    let result = DICT.lookup(word);
    Ok(lookup_result_to_py(py, &result)?.into())
}

/// Return the word with a combining acute U+0301 after the stressed vowel.
///
/// Convenience wrapper around ``lookup(word)["readings"][0]["stressed_form"]``.
#[pyfunction]
fn mark(word: &str) -> String {
    let result = DICT.lookup(word);
    match result.readings.first() {
        Some(r) => r.stressed_form.clone(),
        None => word.to_string(),
    }
}

/// Return the 0-based syllable index of the stressed syllable.
///
/// Convenience wrapper for callers that only need the stress position.
#[pyfunction]
fn stress(word: &str) -> usize {
    DICT.stress_index(word)
}

/// Total number of entries in the embedded exception dictionary.
#[pyfunction]
fn word_count() -> usize {
    DICT.exceptions_len()
}

#[pymodule]
fn polish_stress(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(lookup, m)?)?;
    m.add_function(wrap_pyfunction!(mark, m)?)?;
    m.add_function(wrap_pyfunction!(stress, m)?)?;
    m.add_function(wrap_pyfunction!(word_count, m)?)?;
    Ok(())
}


