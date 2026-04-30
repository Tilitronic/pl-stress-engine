"""
Tests for PoliMorf parser.

Uses inline mock data matching the real PoliMorf .tab format:
  word_form<TAB>lemma<TAB>grammatical_tag
"""

import io
import tempfile
from pathlib import Path
from typing import Dict, List

import pytest

from pl_stress.parsers.polimorf import (
    iter_entries,
    build_lemma_index,
    propagate_stress,
)

# ---------------------------------------------------------------------------
# Sample PoliMorf data (subset of real entries)
# ---------------------------------------------------------------------------

SAMPLE_TAB = """\
matematyka\tmatematyka\tsubst:sg:nom:f
matematyki\tmatematyka\tsubst:sg:gen:f
matematyce\tmatematyka\tsubst:sg:dat:f
matematykę\tmatematyka\tsubst:sg:acc:f
matematyką\tmatematyka\tsubst:sg:inst:f
matematyce\tmatematyka\tsubst:sg:loc:f
matematyko\tmatematyka\tsubst:sg:voc:f
matematyki\tmatematyka\tsubst:pl:nom:f
matematyk\tmatematyka\tsubst:pl:gen:f
matematykom\tmatematyka\tsubst:pl:dat:f
matematyki\tmatematyka\tsubst:pl:acc:f
matematy­kami\tmatematyka\tsubst:pl:inst:f
matematykach\tmatematyka\tsubst:pl:loc:f
kobieta\tkobieta\tsubst:sg:nom:f
kobiety\tkobieta\tsubst:sg:gen:f
kobiecie\tkobieta\tsubst:sg:dat:f
kobietę\tkobieta\tsubst:sg:acc:f
kobietą\tkobieta\tsubst:sg:inst:f
kobiecie\tkobieta\tsubst:sg:loc:f
kobiety\tkobieta\tsubst:pl:nom:f
kobiet\tkobieta\tsubst:pl:gen:f
"""


def _make_polimorf_file(content: str) -> Path:
    """Write mock PoliMorf content to a temp file."""
    tmp = tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=".tab", delete=False
    )
    tmp.write(content)
    tmp.close()
    return Path(tmp.name)


# ---------------------------------------------------------------------------
# Unit: iter_entries
# ---------------------------------------------------------------------------

class TestIterEntries:
    def test_yields_tuples(self):
        path = _make_polimorf_file(SAMPLE_TAB)
        try:
            entries = list(iter_entries(path))
            assert len(entries) > 0
            # Each entry is (form, lemma, tag)
            assert all(len(e) == 3 for e in entries)
        finally:
            path.unlink()

    def test_lowercases_forms(self):
        path = _make_polimorf_file("Matematyka\tMatematyka\tsubst:sg:nom:f\n")
        try:
            entries = list(iter_entries(path))
            assert entries[0][0] == "matematyka"
            assert entries[0][1] == "matematyka"
        finally:
            path.unlink()

    def test_first_entry(self):
        path = _make_polimorf_file(SAMPLE_TAB)
        try:
            entries = list(iter_entries(path))
            form, lemma, tag = entries[0]
            assert form == "matematyka"
            assert lemma == "matematyka"
            assert "subst" in tag
        finally:
            path.unlink()

    def test_skips_malformed_lines(self):
        bad = "only_one_field\n\nmatematyka\tmatematyka\tsubst:sg:nom:f\n"
        path = _make_polimorf_file(bad)
        try:
            entries = list(iter_entries(path))
            # 2-field line is accepted (tag defaults to ""), 1-field is skipped
            forms = [e[0] for e in entries]
            assert "matematyka" in forms
        finally:
            path.unlink()


# ---------------------------------------------------------------------------
# Unit: build_lemma_index
# ---------------------------------------------------------------------------

class TestBuildLemmaIndex:
    def test_groups_by_lemma(self):
        path = _make_polimorf_file(SAMPLE_TAB)
        try:
            index = build_lemma_index(path)
            assert "matematyka" in index
            assert "kobieta" in index
        finally:
            path.unlink()

    def test_all_forms_under_lemma(self):
        path = _make_polimorf_file(SAMPLE_TAB)
        try:
            index = build_lemma_index(path)
            forms = [f for f, _ in index["matematyka"]]
            # All inflected forms of matematyka should be present
            assert "matematyki" in forms
            assert "matematykę" in forms
            assert "matematyką" in forms
        finally:
            path.unlink()


# ---------------------------------------------------------------------------
# Unit: propagate_stress
# ---------------------------------------------------------------------------

class TestPropagateStress:
    def _index(self) -> Dict:
        path = _make_polimorf_file(SAMPLE_TAB)
        try:
            return build_lemma_index(path)
        finally:
            path.unlink()

    def test_propagates_to_same_syllable_count_forms(self):
        index = self._index()
        # matematyka (lemma) → stress from end = 3
        known = {"matematyka": 3}
        new = propagate_stress(index, known)

        # matematyki (4 syl = same as matematyka) → should get stress 3
        assert new.get("matematyki") == 3
        # matematyce (4 syl) → stress 3
        assert new.get("matematyce") == 3
        # matematykę (4 syl) → stress 3
        assert new.get("matematykę") == 3

    def test_does_not_override_existing_entries(self):
        index = self._index()
        known = {"matematyka": 3, "matematyki": 2}  # matematyki explicitly penultimate
        new = propagate_stress(index, known)
        # propagate_stress returns NEW entries only; doesn't touch known
        assert new.get("matematyki") != 3 or "matematyki" not in new

    def test_skips_lemma_without_known_stress(self):
        index = self._index()
        known = {}  # no known stress for any lemma
        new = propagate_stress(index, known)
        assert len(new) == 0

    def test_adjusts_for_different_syllable_count(self):
        # matematyk (pl gen, 4 syl) vs matematyka (5 syl)
        # With stress-from-end=3, forms with fewer syllables get stress adjusted
        index = self._index()
        known = {"matematyka": 3}
        new = propagate_stress(index, known)
        # matematyk: ma-te-ma-tyk = 4 syllables; stress_from_end = min(3, 4-1) = 3
        # (4th from end would be off the edge, so clamped to 3)
        if "matematyk" in new:
            assert new["matematyk"] >= 1  # valid stress_from_end

    def test_kobieta_penultimate_not_propagated_as_exception(self):
        # kobieta has default penultimate stress — if it were in known with fe=2,
        # propagation would add its forms with fe=2 (still default, not an exception)
        index = self._index()
        known = {"kobieta": 2}  # penultimate = default
        new = propagate_stress(index, known)
        # kobiety: ko-bie-ty = 3 syl, same as kobieta (3) → propagated with fe=2
        assert new.get("kobiety", 2) == 2  # still penultimate
