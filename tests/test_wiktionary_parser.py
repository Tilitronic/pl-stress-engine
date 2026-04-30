"""
Tests for Wiktionary dump parser.

Uses small in-memory XML fragments that mimic the real plwiktionary format
discovered by probing the actual dump. Does NOT require the dump file.
"""

import bz2
import io
import tempfile
from pathlib import Path

import pytest

from pl_stress.parsers.wiktionary import (
    _find_polish_section,
    _stress_from_akc_template,
    _stress_from_ipa,
    _process_page,
    extract_stress_exceptions,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_page(title: str, wikitext: str) -> str:
    """Build a minimal Wiktionary XML page element."""
    return (
        f"  <page>\n"
        f"    <title>{title}</title>\n"
        f"    <ns>0</ns>\n"
        f"    <id>1</id>\n"
        f"    <revision><id>1</id><text xml:space=\"preserve\">"
        f"{wikitext}"
        f"</text></revision>\n"
        f"  </page>\n"
    )


def _make_dump(*pages: str) -> bytes:
    """Wrap pages in a minimal Wiktionary XML dump and bz2-compress it."""
    xml = (
        '<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/">\n'
        "  <siteinfo><sitename>Wikisłownik</sitename></siteinfo>\n"
        + "".join(pages)
        + "</mediawiki>\n"
    )
    return bz2.compress(xml.encode("utf-8"))


def _write_dump(pages: list[str]) -> Path:
    """Write a tiny bz2 dump to a temp file and return the path."""
    tmp = tempfile.NamedTemporaryFile(suffix=".xml.bz2", delete=False)
    tmp.write(_make_dump(*pages))
    tmp.close()
    return Path(tmp.name)


# ---------------------------------------------------------------------------
# Actual wikitext sections (structure matches real dump, discovered by probing)
# ---------------------------------------------------------------------------

MATEMATYKA_SECTION = """\
== matematyka ({{język polski}}) ==
{{wymowa}}
: {{IPA3|ˌmatɛ̃ˈmatɨka}}, {{AS3|mãt'''e'''m'''a'''tyka}}, {{objaśnienie wymowy|NAZAL|AKC3|AKCP}} {{audio|Pl-matematyka.ogg}}
{{znaczenia}}
''rzeczownik, rodzaj żeński''
: (1.1) nauka zajmująca się badaniem struktur abstrakcyjnych
"""

MUZYKA_SECTION = """\
== muzyka ({{język polski}}) ==
{{wymowa}}
: {{IPA3|ˈmuzɨka}}, {{AS3|m'''u'''zyka}}, {{objaśnienie wymowy|AKC3}} {{audio|Pl-muzyka.ogg}}
{{znaczenia}}
"""

PREZYDENT_SECTION = """\
== prezydent ({{język polski}}) ==
{{wymowa}} {{audio|Pl-prezydent.ogg}}, {{IPA3|ˈprɛzɨdɛ̃nt}}, {{AS3|pr'''e'''zydẽnt}}, {{objaśnienie wymowy|NAZAL|AKC3}}
{{znaczenia}}
"""

KOBIETA_SECTION = """\
== kobieta ({{język polski}}) ==
{{wymowa}}
: {{IPA3|kɔˈbʲɛta}}, {{AS3|kob'''e'''ta}}, {{objaśnienie wymowy|}}
{{znaczenia}}
"""

# Penultimate (default) — no AKC template, IPA stress on 2nd from end
RECEPTA_SECTION = """\
== recepta ({{język polski}}) ==
{{wymowa}}
: {{IPA3|rɛˈtsɛpta}}, {{objaśnienie wymowy|}}
{{znaczenia}}
"""

# German section mixed in — should NOT be extracted
LOGIK_SECTION = """\
== Logik ({{język niemiecki}}) ==
{{wymowa}}
: {{IPA3|ˈloːɡɪk}}
{{znaczenia}}
== logika ({{język polski}}) ==
{{wymowa}}
: {{IPA3|ˈlɔɟika}}, {{objaśnienie wymowy|ZM|AKC3}}
{{znaczenia}}
"""

ZROBILIBYSMY_SECTION = """\
== zrobilibyśmy ({{język polski}}) ==
{{wymowa}}
: {{objaśnienie wymowy|AKC4}}
{{znaczenia}}
"""


# ---------------------------------------------------------------------------
# Unit: _find_polish_section
# ---------------------------------------------------------------------------

class TestFindPolishSection:
    def test_finds_standard_format(self):
        section = _find_polish_section(MATEMATYKA_SECTION)
        assert section is not None
        assert "IPA3" in section
        assert "AKC3" in section

    def test_finds_among_other_sections(self):
        section = _find_polish_section(LOGIK_SECTION)
        assert section is not None
        assert "lɔɟika" in section         # Polish IPA
        assert "loːɡɪk" not in section    # German IPA must not appear

    def test_returns_none_for_non_polish_page(self):
        non_polish = "== Logik ({{język niemiecki}}) ==\n{{wymowa}}: {{IPA3|ˈloːɡɪk}}"
        assert _find_polish_section(non_polish) is None

    def test_section_ends_at_next_heading(self):
        multi = """\
== alfa ({{język polski}}) ==
sekcja polska
== alfa ({{język grecki}}) ==
sekcja grecka
"""
        section = _find_polish_section(multi)
        assert section is not None
        assert "sekcja polska" in section
        assert "sekcja grecka" not in section


# ---------------------------------------------------------------------------
# Unit: _stress_from_akc_template
# ---------------------------------------------------------------------------

class TestStressFromAkcTemplate:
    def test_akc3(self):
        assert _stress_from_akc_template("{{objaśnienie wymowy|AKC3}}") == 3

    def test_akc3_with_other_params(self):
        assert _stress_from_akc_template("{{objaśnienie wymowy|NAZAL|AKC3|AKCP}}") == 3

    def test_akc4(self):
        assert _stress_from_akc_template("{{objaśnienie wymowy|AKC4}}") == 4

    def test_akco_oxytone(self):
        assert _stress_from_akc_template("{{objaśnienie wymowy|AKCO}}") == 1

    def test_no_akc_template(self):
        assert _stress_from_akc_template("{{wymowa}}: {{IPA3|kɔˈbʲɛta}}") is None

    def test_empty_akc(self):
        # {{objaśnienie wymowy|}} without AKC → None
        assert _stress_from_akc_template("{{objaśnienie wymowy|}}") is None


# ---------------------------------------------------------------------------
# Unit: _stress_from_ipa
# ---------------------------------------------------------------------------

class TestStressFromIpa:
    def test_matematyka(self):
        # ˌmatɛ̃ˈmatɨka: 2 vowels before ˈ (a, ɛ̃) → index 2
        result = _stress_from_ipa("{{IPA3|ˌmatɛ̃ˈmatɨka}}")
        assert result == 2

    def test_muzyka(self):
        # ˈmuzɨka: 0 vowels before ˈ → index 0
        result = _stress_from_ipa("{{IPA3|ˈmuzɨka}}")
        assert result == 0

    def test_kobieta(self):
        # kɔˈbʲɛta: 1 vowel before ˈ (ɔ) → index 1
        result = _stress_from_ipa("{{IPA3|kɔˈbʲɛta}}")
        assert result == 1

    def test_no_ipa_template(self):
        assert _stress_from_ipa("brak wymowy") is None

    def test_no_stress_marker(self):
        # IPA without ˈ → None (can't determine stress)
        assert _stress_from_ipa("{{IPA3|muzɨka}}") is None


# ---------------------------------------------------------------------------
# Unit: _process_page (integrated section + template parsing)
# ---------------------------------------------------------------------------

class TestProcessPage:
    def test_matematyka_gives_akc3(self):
        results = list(_process_page("matematyka", MATEMATYKA_SECTION))
        assert len(results) == 1
        word, from_end, source = results[0]
        assert word == "matematyka"
        assert from_end == 3
        assert source == "akc"

    def test_muzyka_gives_akc3(self):
        results = list(_process_page("muzyka", MUZYKA_SECTION))
        assert len(results) == 1
        assert results[0][1] == 3

    def test_prezydent_gives_akc3(self):
        results = list(_process_page("prezydent", PREZYDENT_SECTION))
        assert len(results) == 1
        assert results[0][1] == 3

    def test_zrobilibysmy_gives_akc4(self):
        results = list(_process_page("zrobilibyśmy", ZROBILIBYSMY_SECTION))
        assert len(results) == 1
        assert results[0][1] == 4

    def test_kobieta_not_yielded(self):
        # kobieta has empty AKC → falls to IPA → ˈ on 2nd-from-end → penultimate
        # penultimate stress (=2) should NOT be yielded (it's the default)
        results = list(_process_page("kobieta", KOBIETA_SECTION))
        assert len(results) == 0

    def test_skip_namespaced_titles(self):
        results = list(_process_page("Wikisłownik:Strona", MATEMATYKA_SECTION))
        assert len(results) == 0

    def test_logik_page_extracts_polish_only(self):
        # "Logik" title but page has both German and Polish sections
        results = list(_process_page("logika", LOGIK_SECTION))
        assert any(w == "logika" and fe == 3 for w, fe, _ in results)


# ---------------------------------------------------------------------------
# Integration: extract_stress_exceptions (tiny synthetic dump)
# ---------------------------------------------------------------------------

class TestExtractStressExceptions:
    def test_extracts_akc3_words(self):
        dump_path = _write_dump([
            _make_page("matematyka", MATEMATYKA_SECTION),
            _make_page("muzyka", MUZYKA_SECTION),
            _make_page("kobieta", KOBIETA_SECTION),  # should be excluded (default)
        ])
        try:
            exc = extract_stress_exceptions(dump_path)
            assert exc.get("matematyka") == 3
            assert exc.get("muzyka") == 3
            assert "kobieta" not in exc
        finally:
            dump_path.unlink()

    def test_akc4_word(self):
        dump_path = _write_dump([
            _make_page("zrobilibyśmy", ZROBILIBYSMY_SECTION),
        ])
        try:
            exc = extract_stress_exceptions(dump_path)
            assert exc.get("zrobilibyśmy") == 4
        finally:
            dump_path.unlink()
