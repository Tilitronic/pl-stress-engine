"""
Syllabification — count vowel nuclei to determine syllable count.

Polish syllables always have exactly one vowel nucleus.

Key rule: 'i' immediately before another vowel is a palatalizing glide [j],
not a vowel nucleus. Examples:
  - 'ie' in 'siedemset' = [jɛ] → one nucleus, not two
  - 'ia' in 'ciasto'    = [ja] → one nucleus
  - 'ie' in 'kobieta'  = [jɛ] → one nucleus
But 'i' before a consonant is a proper vowel: 'fizyka' fi-zy-ka = 3 syllables.
"""

VOWELS: frozenset = frozenset("aeiouąęóy")


def count_syllables(word: str) -> int:
    """Return the number of syllables in a Polish word."""
    chars = list(word.lower())
    n = len(chars)
    count = 0
    for idx, ch in enumerate(chars):
        if ch in VOWELS:
            # 'i' before another vowel acts as glide [j], not a vowel nucleus
            if ch == 'i' and idx + 1 < n and chars[idx + 1] in VOWELS:
                continue
            count += 1
    return max(1, count)


def stress_syllable_index_from_end(word: str, n_from_end: int) -> int:
    """
    Convert 'n-th syllable from end' (1-based, Polish convention) to
    0-based syllable index from the start.

    n_from_end=2 → penultimate (default Polish stress)
    n_from_end=3 → antepenultimate
    n_from_end=4 → pre-antepenultimate
    """
    n = count_syllables(word)
    return max(0, n - n_from_end)
