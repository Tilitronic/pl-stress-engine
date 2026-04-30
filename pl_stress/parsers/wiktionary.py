"""
Wiktionary dump parser for Polish stress extraction.

plwiktionary (Polish Wiktionary) encodes stress in two complementary ways:

1. {{objaśnienie wymowy|AKC3}}  →  stress on 3rd syllable from end (proparoksytona)
   {{objaśnienie wymowy|AKC4}}  →  stress on 4th syllable from end
   {{objaśnienie wymowy|AKCO}}  →  oxytone (stress on last syllable)
   Absence of AKC template      →  penultimate (default, not stored as exception)

2. {{IPA3|ˈfʲizɨka}}  →  primary stress marker ˈ precedes the stressed vowel;
   count IPA vowel tokens before ˈ to find syllable index.
   ˌ = secondary stress (ignored).

Strategy: prefer AKC template (explicit, reliable), fall back to IPA parsing.
IPA string is always captured when present, regardless of stress source.
"""

import bz2
import re
from pathlib import Path
from typing import Dict, Generator, Optional, Tuple

# Template patterns
_AKC_RE = re.compile(r"\{\{objaśnienie wymowy\|([^}]+)\}\}")
_IPA3_RE = re.compile(r"\{\{IPA3\|([^}]+)\}\}")

# IPA vowel characters used in Polish transcriptions
_IPA_VOWELS = frozenset("aeiouɛɨɔɑæʊʌɪəɐɜɞɶɑɒɵɤɯɪʏ")


def _find_polish_section(text: str) -> Optional[str]:
    """
    Return the wymowa (pronunciation) slice of a plwiktionary page.

    plwiktionary page format:
      == word ({{język polski}}) ==
      {{wymowa}}
      : {{IPA3|...}}, {{objaśnienie wymowy|AKC3}}
      {{znaczenia}}
      ...

    We extract from the first {{wymowa}} to the next section-ending template.
    This is simpler and more robust than matching the section header.
    """
    # Guard: must be a Polish word page
    if "język polski" not in text and "{{wymowa}}" not in text:
        return None

    idx = text.find("{{wymowa}}")
    if idx == -1:
        return None

    rest = text[idx:]
    # End at the start of the semantic content section
    end_markers = [
        "{{znaczenia}}", "{{odmiana}}", "{{synonimy}}",
        "{{przykłady}}", "{{kolokacje}}", "{{składnia}}",
    ]
    end = len(rest)
    for marker in end_markers:
        pos = rest.find(marker)
        if 0 < pos < end:
            end = pos
    return rest[:end]


def _stress_from_akc_template(section: str) -> Optional[int]:
    """
    Return stress position as syllables-from-end (1-based) from AKC template.
    Returns None if no AKC template found (caller should assume default/penultimate).
    """
    for m in _AKC_RE.finditer(section):
        params = m.group(1).split("|")
        for p in params:
            p = p.strip()
            if p == "AKC3":
                return 3
            if p == "AKC4":
                return 4
            if p == "AKCO":
                return 1  # oxytone = last syllable = 1 from end
    return None


def _ipa_string(section: str) -> Optional[str]:
    """Return the raw IPA transcription from {{IPA3|...}}, if present."""
    m = _IPA3_RE.search(section)
    return m.group(1).strip() if m else None


def _stress_from_ipa(ipa: str) -> Optional[int]:
    """
    Return stressed syllable index (0-based from start) using the ˈ marker.
    """
    primary = ipa.find("ˈ")
    if primary == -1:
        return None
    n_vowels_before = sum(1 for ch in ipa[:primary] if ch in _IPA_VOWELS)
    return n_vowels_before


# ──────────────────────────────────────────────────────────────────────────────
# Public entry point
# ──────────────────────────────────────────────────────────────────────────────

def iter_polish_stress(
    dump_path: Path,
) -> Generator[Tuple[str, int, Optional[str], str], None, None]:
    """
    Stream the Wiktionary bz2 dump and yield
    ``(word, stress_from_end, ipa, source)`` for every Polish entry where
    stress can be determined.

    ``stress_from_end``: 1-based count from end — 1=oxytone, 2=penultimate, …
    ``ipa``: raw IPA string (e.g. ``"zrɔˈbʲiliɕmɨ"``), or ``None``.
    ``source``: ``'wiktionary_akc'`` (explicit template) or ``'wiktionary_ipa'``
                (derived from IPA transcription).

    Both exceptions *and* regular penultimate words are yielded so IPA is
    captured for all entries; the consumer filters as needed.
    """
    opener = bz2.open if str(dump_path).endswith(".bz2") else open
    with opener(dump_path, "rt", encoding="utf-8") as fh:
        title = ""
        in_text = False
        text_buf: list[str] = []
        for raw_line in fh:
            line = raw_line

            if "<title>" in line:
                m = re.search(r"<title>(.*?)</title>", line)
                if m:
                    title = m.group(1).strip()
                continue

            if "<text" in line:
                in_text = True
                text_buf = []
                after = re.sub(r"<text[^>]*>", "", line)
                text_buf.append(after)
                if "</text>" in after:
                    in_text = False
                    full = "".join(text_buf).replace("</text>", "")
                    yield from _process_page(title, full)
                continue

            if in_text:
                if "</text>" in line:
                    in_text = False
                    text_buf.append(line.replace("</text>", ""))
                    full = "".join(text_buf)
                    yield from _process_page(title, full)
                    text_buf = []
                else:
                    text_buf.append(line)


def _process_page(
    title: str, text: str
) -> Generator[Tuple[str, int, Optional[str], str], None, None]:
    # Skip non-article namespaces
    if ":" in title:
        return

    section = _find_polish_section(text)
    if section is None:
        return

    word = title.lower().strip()
    if not word:
        return

    ipa = _ipa_string(section)

    # Strategy 1: AKC template (explicit, highest confidence)
    from_end = _stress_from_akc_template(section)
    if from_end is not None:
        yield (word, from_end, ipa, "wiktionary_akc")
        return

    # Strategy 2: IPA parsing (good confidence)
    if ipa is not None:
        idx_from_start = _stress_from_ipa(ipa)
        if idx_from_start is not None:
            from pl_stress.syllabify import count_syllables
            n = count_syllables(word)
            fe = max(1, min(n - idx_from_start, n))
            yield (word, fe, ipa, "wiktionary_ipa")


def extract_stress_exceptions(
    dump_path: Path,
) -> Dict[str, dict]:
    """
    Parse the full dump and return::

        {
          "word": {"stress_from_end": N, "ipa": "..." | None, "source": "..."},
          ...
        }

    All entries are returned (including penultimate); the caller prunes as needed.
    """
    result: Dict[str, dict] = {}
    for word, from_end, ipa, source in iter_polish_stress(dump_path):
        if word not in result:
            result[word] = {"stress_from_end": from_end, "ipa": ipa, "source": source}
    return result


