"""
StressResolver — integrates exception dictionary + rule engine.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
from pathlib import Path
from typing import Dict, Optional

from .rules import Confidence, RuleMatch, apply_rules
from .syllabify import count_syllables


@dataclass(frozen=True)
class StressResult:
    """
    Result of a stress query.

    stress_from_end: int  — 1-based count from word end
      1 = oxytone (last syllable)
      2 = penultimate  ← default Polish
      3 = antepenultimate
      4 = pre-antepenultimate

    syllable_count: int   — total syllables in the word
    confidence: Confidence
    rule_name: str        — which rule/source matched ('' for default)
    """
    stress_from_end: int
    syllable_count: int
    confidence: Confidence
    rule_name: str = ""

    @property
    def syllable_index(self) -> int:
        """0-based syllable index from start."""
        return max(0, self.syllable_count - self.stress_from_end)

    def stressed_syllable_number(self) -> int:
        """1-based syllable number from start."""
        return self.syllable_index + 1


class StressResolver:
    """
    Polish word stress resolver.

    Usage:
        resolver = StressResolver()
        resolver.load_exceptions({"matematyka": 3, "muzyka": 3})
        result = resolver.resolve("matematyka")
        print(result.stress_from_end)  # 3
    """

    def __init__(self) -> None:
        self._exceptions: Dict[str, int] = {}

    def load_exceptions(self, exceptions: Dict[str, int]) -> None:
        """
        Load a pre-built exception dictionary.
        Keys are lowercase words, values are stress_from_end (1-based).
        """
        self._exceptions = {k.lower(): v for k, v in exceptions.items()}

    def load_exceptions_json(self, path: Path) -> None:
        import json
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
        self.load_exceptions(data)

    def resolve(self, word: str) -> StressResult:
        """Resolve stress for a single word."""
        lower = word.lower().strip()
        n = count_syllables(lower)

        # 1. Exact exception dictionary lookup
        if lower in self._exceptions:
            from_end = self._exceptions[lower]
            return StressResult(
                stress_from_end=from_end,
                syllable_count=n,
                confidence=Confidence.EXACT,
                rule_name="exception_dict",
            )

        # 2. Productive grammatical rules
        match = apply_rules(lower)
        if match is not None:
            # Clamp: stress_from_end cannot exceed syllable count
            from_end = min(match.stress_from_end, n)
            return StressResult(
                stress_from_end=from_end,
                syllable_count=n,
                confidence=Confidence.RULE,
                rule_name=match.rule_name,
            )

        # 3. Default: penultimate
        from_end = min(2, n)  # monosyllables: stress "from end 1"
        return StressResult(
            stress_from_end=from_end,
            syllable_count=n,
            confidence=Confidence.DEFAULT,
            rule_name="penultimate",
        )

    def resolve_many(self, words: list[str]) -> list[StressResult]:
        return [self.resolve(w) for w in words]
