"""
PoliMorf parser.

PoliMorf-0.6.7.tab format (tab-separated, no header):
  <word_form>  <lemma>  <grammatical_tag>

Example lines:
  matematyka	matematyka	subst:sg:nom:f
  matematyki	matematyka	subst:sg:gen:f
  matematyką	matematyka	subst:sg:inst:f

Tag grammar (partial, enough for stress propagation):
  subst    = noun
  verb     = verb (but inflected forms use praet, inf, etc.)
  praet    = past tense
  cond     = conditional mood
  inf      = infinitive

We use PoliMorf to:
  1. Build lemma → [inflected forms] index.
  2. Propagate known stress from lemma to all its forms (if syllable count allows).
"""

from pathlib import Path
from typing import Dict, Iterator, List, Optional, Tuple


def iter_entries(polimorf_path: Path) -> Iterator[Tuple[str, str, str]]:
    """Yield (word_form, lemma, tag) triples from a PoliMorf .tab file."""
    with open(polimorf_path, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            parts = line.split("\t")
            if len(parts) >= 3:
                yield parts[0].lower(), parts[1].lower(), parts[2]
            elif len(parts) == 2:
                yield parts[0].lower(), parts[1].lower(), ""


def build_lemma_index(
    polimorf_path: Path,
) -> Dict[str, List[Tuple[str, str]]]:
    """
    Return {lemma: [(word_form, tag), ...]} from the full PoliMorf file.
    """
    index: Dict[str, List[Tuple[str, str]]] = {}
    for form, lemma, tag in iter_entries(polimorf_path):
        index.setdefault(lemma, []).append((form, tag))
    return index


def propagate_stress(
    lemma_index: Dict[str, List[Tuple[str, str]]],
    known_stress: Dict[str, dict],
) -> Dict[str, dict]:
    """
    For each lemma with known stress, propagate to all inflected forms.

    ``known_stress`` maps word → ``{"stress_from_end": N, "ipa": str|None}``.

    IPA is **not** propagated to inflected forms (it would be wrong).
    Returns a dict of NEW entries to merge (does not override explicit ones).
    """
    from ..syllabify import count_syllables

    new_entries: Dict[str, dict] = {}

    for lemma, forms in lemma_index.items():
        if lemma not in known_stress:
            continue
        stress_from_end = known_stress[lemma]["stress_from_end"]
        lemma_syllables = count_syllables(lemma)

        for form, _tag in forms:
            if form in known_stress:
                continue  # explicit data wins
            form_syllables = count_syllables(form)

            if form_syllables == lemma_syllables:
                new_entries.setdefault(form, {"stress_from_end": stress_from_end, "ipa": None})
            elif form_syllables >= 2:
                adjusted = min(stress_from_end, form_syllables - 1)
                adjusted = max(adjusted, 1)
                new_entries.setdefault(form, {"stress_from_end": adjusted, "ipa": None})

    return new_entries
