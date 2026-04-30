"""
Export the master SQLite DB to the JSON format consumed by the Rust builder.

Output format (exceptions.json)
--------------------------------
{
  "word": {
    "stress_idx": <int>,   // 0-based syllable index from start
    "ipa": <str|null>      // IPA transcription, or null
  },
  ...
}

Only entries worth storing in the runtime binary are exported:
  - Non-penultimate stress  (rules/default handle penultimate)
  - OR  penultimate stress with IPA  (so IPA is available at runtime)
"""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path

from .syllabify import count_syllables


def export_json(conn: sqlite3.Connection, out_path: Path, verbose: bool = True) -> int:
    """
    Write ``exceptions.json`` from the master DB.

    Returns the number of entries written.
    """
    rows = conn.execute(
        """
        SELECT word, stress_from_end, ipa, syllable_count
        FROM   words
        WHERE  stress_from_end IS NOT NULL
          AND  (
               stress_from_end != 2          -- non-penultimate stress
            OR ipa IS NOT NULL               -- or has IPA worth storing
          )
        ORDER BY word
        """
    ).fetchall()

    out: dict = {}
    for row in rows:
        word: str = row["word"]
        stress_from_end: int = row["stress_from_end"]
        ipa = row["ipa"]
        n: int = row["syllable_count"] or count_syllables(word)

        stress_idx = max(0, n - stress_from_end)
        out[word] = {"stress_idx": stress_idx, "ipa": ipa}

    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(out, fh, ensure_ascii=False, indent=2)

    if verbose:
        print(f"  exported {len(out):,} entries → {out_path}")

    return len(out)
