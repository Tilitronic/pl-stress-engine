"""
Tests for syllabify module.
TDD: these tests define the expected behaviour before implementation details.

Key insight: in Polish, 'i' before another vowel is a palatalizing glide [j],
NOT a vowel nucleus:
  - 'ie' in 'siedemset' → [jɛ] → 1 nucleus → sie-dem-set = 3 syllables
  - 'ie' in 'kobieta'   → [jɛ] → 1 nucleus → ko-bie-ta  = 3 syllables
  - 'i'  in 'fizyka'    → [i]  → 1 nucleus → fi-zy-ka   = 3 syllables (before consonant)
"""

import pytest
from pl_stress.syllabify import count_syllables, stress_syllable_index_from_end


class TestCountSyllables:
    def test_simple_words(self):
        assert count_syllables("kot") == 1       # k-o-t → 1 vowel
        assert count_syllables("kota") == 2      # ko-ta → 2 vowels
        assert count_syllables("kobra") == 2     # kob-ra → 2 vowels

    def test_i_before_vowel_is_glide(self):
        # 'ie', 'ia', 'io', 'iu' — 'i' is glide, not nucleus
        assert count_syllables("kobieta") == 3   # ko-bie-ta  (o, e, a)
        assert count_syllables("siedemset") == 3 # sie-dem-set (e, e, e)
        assert count_syllables("niebo") == 2     # nie-bo      (e, o)
        assert count_syllables("biuro") == 2     # biu-ro      (u, o)

    def test_i_before_consonant_is_vowel(self):
        # 'i' before a consonant is a proper vowel nucleus
        assert count_syllables("fizyka") == 3      # fi-zy-ka
        assert count_syllables("polityka") == 4    # po-li-ty-ka
        assert count_syllables("zrobiłbym") == 3   # zro-biłbym  (o, i, y)
        assert count_syllables("chodziliśmy") == 4 # cho-dzi-li-śmy (o, i, i, y)

    def test_multisyllable(self):
        assert count_syllables("matematyka") == 5   # ma-te-ma-ty-ka
        assert count_syllables("muzyka") == 3        # mu-zy-ka
        assert count_syllables("prezydent") == 3     # pre-zy-dent
        assert count_syllables("uniwersytet") == 5   # u-ni-wer-sy-tet

    def test_numerals(self):
        assert count_syllables("czterysta") == 3    # czte-ry-sta (e, y, a)
        assert count_syllables("siedemset") == 3    # sie-dem-set (e, e, e)
        assert count_syllables("zrobilibyśmy") == 5 # zro-bi-li-byś-my

    def test_minimum_one(self):
        assert count_syllables("brr") == 1
        assert count_syllables("PKP") == 1   # no vowels → 1

    def test_case_insensitive(self):
        assert count_syllables("MATEMATYKA") == count_syllables("matematyka")

    def test_polish_special_vowels(self):
        # ą, ę, ó count as vowel nuclei
        assert count_syllables("wąż") == 1        # wąż
        assert count_syllables("zębów") == 2      # zę-bów


class TestStressSyllableIndexFromEnd:
    def test_penultimate(self):
        # muzyka: 3 syllables, penultimate = index 1 from start (0-based)
        assert stress_syllable_index_from_end("muzyka", 2) == 1   # 3-2=1

    def test_antepenultimate(self):
        # matematyka: 5 syllables, antepenultimate = index 2 from start
        assert stress_syllable_index_from_end("matematyka", 3) == 2  # 5-3=2

    def test_preantepenultimate(self):
        # zrobilibyśmy: 5 syllables, 4th from end = index 1 from start
        assert stress_syllable_index_from_end("zrobilibyśmy", 4) == 1  # 5-4=1

    def test_oxytone(self):
        # kobieta: 3 syllables, oxytone = last = index 2
        assert stress_syllable_index_from_end("kobieta", 1) == 2   # 3-1=2

    def test_verified_examples_from_sources(self):
        # chodziliśmy: cho-dzi-li-śmy = 4 syl, antepenult = index 1
        # (stress on "dzi" → choDZIliśmy)
        assert stress_syllable_index_from_end("chodziliśmy", 3) == 1  # 4-3=1
        # poszlibyśmy: po-szli-byś-my = 4 syl, 4th from end = index 0
        # (stress on "po" → POszlibyśmy)
        assert stress_syllable_index_from_end("poszlibyśmy", 4) == 0  # 4-4=0
