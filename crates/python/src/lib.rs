use once_cell::sync::Lazy;
use pl_stress_core::{Confidence, StressDict};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

static EXCEPTIONS_BIN: &[u8] = include_bytes!("../../../data/processed/exceptions.bin");

static DICT: Lazy<StressDict> = Lazy::new(|| {
    StressDict::from_bytes(EXCEPTIONS_BIN).expect("Failed to load embedded stress dictionary")
});

/// Return the 0-based syllable index of the stressed syllable.
///
/// Example::
///
///     >>> import polish_stress
///     >>> polish_stress.stress("matematyka")
///     2
#[pyfunction]
fn stress(word: &str) -> usize {
    DICT.stress_index(word)
}

/// Return full stress information as a dict.
///
/// Keys:
///   ``word``          – the input word
///   ``syllables``     – list of syllable strings
///   ``syllable_index`` – 0-based index of stressed syllable from start
///   ``stress_from_end`` – 1-based from end (2 = penultimate, 3 = antepenult, …)
///   ``ipa``           – IPA transcription string, or ``None``
///   ``confidence``    – ``"exact"`` | ``"rule"`` | ``"default"``
///
/// Example::
///
///     >>> polish_stress.stress_info("zrobiliśmy")
///     {
///         'word': 'zrobiliśmy',
///         'syllables': ['zro', 'bi', 'li', 'śmy'],
///         'syllable_index': 1,
///         'stress_from_end': 3,
///         'ipa': None,
///         'confidence': 'rule'
///     }
#[pyfunction]
fn stress_info(py: Python<'_>, word: &str) -> PyResult<Py<PyDict>> {
    let r = DICT.stress(word);
    let conf = match r.confidence {
        Confidence::Exact   => "exact",
        Confidence::Rule    => "rule",
        Confidence::Default => "default",
    };
    let dict = PyDict::new_bound(py);
    dict.set_item("word", word)?;
    let syllables = PyList::new_bound(py, &r.syllables);
    dict.set_item("syllables", syllables)?;
    dict.set_item("syllable_index", r.syllable_index)?;
    dict.set_item("stress_from_end", r.stress_from_end())?;
    dict.set_item("ipa", r.ipa.as_deref())?;
    dict.set_item("confidence", conf)?;
    Ok(dict.into())
}

#[pymodule]
fn polish_stress(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(stress, m)?)?;
    m.add_function(wrap_pyfunction!(stress_info, m)?)?;
    Ok(())
}

