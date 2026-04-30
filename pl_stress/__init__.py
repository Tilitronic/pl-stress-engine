"""
pl_stress — Polish word stress resolver (Python reference implementation).

Package layout:
  pl_stress/
    __init__.py
    syllabify.py      — vowel-based syllable counter
    parsers/
      wiktionary.py   — streams plwiktionary XML dump
      polimorf.py     — reads PoliMorf .tab file
    rules.py          — productive grammatical rules (from akcentowanie.md)
    resolver.py       — integrates dict + rules
    builder.py        — offline pipeline: generates exceptions dict
"""
from .resolver import StressResolver, StressResult, Confidence

__all__ = ["StressResolver", "StressResult", "Confidence"]
