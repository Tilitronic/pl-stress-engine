"""
Rule-based Polish stress resolver.

Rules encoded from:
  - akcentowanie.md (Wikipedia article + Kutnyj + Polszczyzna.pl + academic sources)
  - Rada Języka Polskiego guidelines

Polish stress is PENULTIMATE by default. Rules below handle exceptions.

Return value convention throughout this module:
  stress_from_end: int  — 1-based, counting from word end
    1 = last syllable (oxytone)
    2 = penultimate (DEFAULT — rules only return this explicitly for clitic compounds)
    3 = antepenultimate (proparoksytona)
    4 = pre-antepenultimate
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

from .syllabify import count_syllables


class Confidence(Enum):
    EXACT = auto()   # found in exception dictionary
    RULE = auto()    # matched a productive grammatical rule
    DEFAULT = auto() # penultimate fallback


@dataclass(frozen=True)
class RuleMatch:
    stress_from_end: int
    rule_name: str


# ---------------------------------------------------------------------------
# Rule predicates
# ---------------------------------------------------------------------------

def _is_acronym(word: str) -> bool:
    """
    All-uppercase ASCII letter strings are acronyms → oxytone (stress on last syllable).
    E.g.: PKP, ONZ, KGB, TVP, GPS
    Mixed-case words with digits/special chars are excluded.
    """
    return len(word) >= 2 and all(ch.isupper() and ch.isascii() for ch in word)


# Prefixes that produce oxytone when attached to a monosyllabic noun:
_OXYTONE_PREFIXES = ("arcy", "eks", "wice", "mini", "super", "ekstra", "euro")


def _is_prefix_plus_monosyllable(word: str) -> bool:
    """
    Words like arcymistrz, eksmąż, wicekról, minibar → oxytone.
    Heuristic: known prefix + the rest has exactly ONE vowel.
    """
    lower = word.lower()
    for prefix in _OXYTONE_PREFIXES:
        if lower.startswith(prefix) and len(lower) > len(prefix):
            rest = lower[len(prefix):]
            if count_syllables(rest) == 1:
                return True
    return False


# ---------------------------------------------------------------------------
# Past tense plural (1st/2nd person) → antepenultimate (3rd from end)
# Endings: -liśmy, -łyśmy, -liście, -łyście
# Source: all references, consistently
# ---------------------------------------------------------------------------
_PAST_PLURAL_ENDINGS = ("liśmy", "łyśmy", "liście", "łyście")


def _is_past_plural(word: str) -> bool:
    lower = word.lower()
    return any(lower.endswith(e) for e in _PAST_PLURAL_ENDINGS)


# ---------------------------------------------------------------------------
# Conditional mood singular + 3rd plural → antepenultimate
# Endings: -łbym, -łbyś, -łby, -łaby, -łabyś, -łabym, -łoby,
#          -liby, -łyby, -łoby, -nęłby, etc.
# Pattern: contains -by- or ends with -by after a consonant cluster
# ---------------------------------------------------------------------------
_COND_SG_ENDINGS = (
    "łbym", "łbyś", "łby",
    "łabym", "łabyś", "łaby",
    "łobym", "łobyś", "łoby",
    "liby", "łyby",
    # rarer forms
    "nęłbym", "nęłbyś", "nęłby",
    "słbym", "słbyś", "słby",
)


def _is_conditional_singular(word: str) -> bool:
    lower = word.lower()
    return any(lower.endswith(e) for e in _COND_SG_ENDINGS)


# ---------------------------------------------------------------------------
# Conditional mood plural (1st/2nd) → pre-antepenultimate (4th from end)
# Endings: -libyśmy, -libyście, -łybyśmy, -łybyście, -łobyśmy, -łobyście
# ---------------------------------------------------------------------------
_COND_PL_ENDINGS = (
    "libyśmy", "libyście",
    "łybyśmy", "łybyście",
    "łobyśmy", "łobyście",
    "słibyśmy", "słibyście",
)


def _is_conditional_plural(word: str) -> bool:
    lower = word.lower()
    return any(lower.endswith(e) for e in _COND_PL_ENDINGS)


# ---------------------------------------------------------------------------
# Nouns ending in -ika/-yka (Latin loans) → antepenultimate
# But only when nominative singular OR case with SAME syllable count.
# We can't know the case here, so we apply broadly and note as Rule
# (PoliMorf propagation will correct specific forms).
#
# Also applies to person-nouns in genitive singular with same syllable count:
# cybernetykiem (same syllable count as cybernetyk)
# ---------------------------------------------------------------------------
def _is_ika_yka(word: str) -> bool:
    lower = word.lower()
    return lower.endswith(("ika", "yka")) and count_syllables(lower) >= 3


# ---------------------------------------------------------------------------
# Numerals: 400–900 with -sta/-set/-kroć → antepenultimate
# Specific words, hard-coded (the productive pattern is very limited)
# ---------------------------------------------------------------------------
_NUMERAL_ANTEPENULT: frozenset[str] = frozenset({
    "czterysta", "czterystu",
    "pięćset", "pięciuset", "pięciuset",
    "sześćset", "sześciuset",
    "siedemset", "siedmiuset",
    "osiemset", "ośmiuset",
    "dziewięćset", "dziewięciuset",
    "osiemkroć", "siedemkroć", "sześćkroć", "pięciokroć",
    "czterokroć", "tysiąckroć",
})
# czterystoma is PENULTIMATE (exception to the exception)
_NUMERAL_PENULT_EXCEPTION: frozenset[str] = frozenset({"czterystoma"})


def _is_numeral_antepenult(word: str) -> bool:
    lower = word.lower()
    return lower in _NUMERAL_ANTEPENULT and lower not in _NUMERAL_PENULT_EXCEPTION


# ---------------------------------------------------------------------------
# Conjunctions with personal suffixes / conditional morphemes → antepenultimate
# abyśmy, żebyście, jeśliby, ponieważby, etc.
# ---------------------------------------------------------------------------
_CONJ_ROOTS = ("aby", "żeby", "ażeby", "jeśliby", "jeżeliby",
               "iżby", "ponieważby", "gdyby")
_CONJ_ANTEPENULT_ENDINGS = ("śmy", "ście", "by", "bym", "byś", "byście", "byśmy")


def _is_conjunction_antepenult(word: str) -> bool:
    lower = word.lower()
    # Exact compound check
    for root in _CONJ_ROOTS:
        if lower.startswith(root) and len(lower) > len(root):
            suffix = lower[len(root):]
            if any(suffix == e or suffix.endswith(e) for e in _CONJ_ANTEPENULT_ENDINGS):
                return count_syllables(lower) >= 3
    return False


# ---------------------------------------------------------------------------
# Traditional foreign words with antepenultimate stress (normative list)
# ---------------------------------------------------------------------------
_TRADITIONAL_ANTEPENULT: frozenset[str] = frozenset({
    "komitet", "uniwersytet", "prezydent",
    "maksimum", "minimum", "optimum",
    "muzeum", "liceum", "technikum",
    "audytorium", "laboratorium", "gymnasium", "seminarium",
    "centrum", "collegium",
})


def _is_traditional_antepenult(word: str) -> bool:
    return word.lower() in _TRADITIONAL_ANTEPENULT


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def apply_rules(word: str) -> Optional[RuleMatch]:
    """
    Apply productive stress rules to a word.
    Returns RuleMatch or None (caller should fall back to penultimate default).

    Rules are checked in priority order (most specific first).
    """
    lower = word.lower()
    n = count_syllables(lower)

    # 1. Acronyms → oxytone (last syllable)
    if _is_acronym(word):  # use original case
        return RuleMatch(stress_from_end=1, rule_name="acronym")

    # 2. Prefix + monosyllable → oxytone
    if _is_prefix_plus_monosyllable(lower):
        return RuleMatch(stress_from_end=1, rule_name="prefix_monosyllable")

    if n < 3:
        # All remaining rules require ≥ 3 syllables
        return None

    # 3. Conditional plural (1st/2nd person) → 4th from end
    if n >= 4 and _is_conditional_plural(lower):
        return RuleMatch(stress_from_end=4, rule_name="conditional_plural")

    # 4. Past plural (1st/2nd person) → antepenultimate
    if _is_past_plural(lower):
        return RuleMatch(stress_from_end=3, rule_name="past_plural")

    # 5. Conditional singular + 3rd plural → antepenultimate
    if _is_conditional_singular(lower):
        return RuleMatch(stress_from_end=3, rule_name="conditional_singular")

    # 6. Conjunctions with conditional/personal morphemes → antepenultimate
    if _is_conjunction_antepenult(lower):
        return RuleMatch(stress_from_end=3, rule_name="conjunction")

    # 7. Numerals 400–900 → antepenultimate
    if _is_numeral_antepenult(lower):
        return RuleMatch(stress_from_end=3, rule_name="numeral")

    # 8. Latin-loan -ika/-yka nouns → antepenultimate
    if _is_ika_yka(lower):
        return RuleMatch(stress_from_end=3, rule_name="ika_yka")

    # 9. Traditional foreign words → antepenultimate
    if _is_traditional_antepenult(lower):
        return RuleMatch(stress_from_end=3, rule_name="traditional_foreign")

    return None
