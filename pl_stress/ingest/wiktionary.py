"""
Ingest plwiktionary dump into the master SQLite database.

Streams the bz2 XML dump using the existing low-level parser and batch-inserts
every Polish entry (word, IPA, stress_from_end, source) into the `words` table.
Existing rows are kept intact (INSERT OR IGNORE), so the ingest is safe to
re-run — it only adds new words, never overwrites manual corrections.
"""

import sqlite3
from pathlib import Path

from pl_stress.parsers.wiktionary import iter_polish_stress
from pl_stress.syllabify import count_syllables

_BATCH = 2_000


def ingest(conn: sqlite3.Connection, dump_path: Path, verbose: bool = True) -> int:
    """
    Parse ``dump_path`` and insert Polish entries into ``words``.

    Returns the number of new rows inserted.
    """
    dump_path = Path(dump_path)
    if not dump_path.exists():
        raise FileNotFoundError(f"Wiktionary dump not found: {dump_path}")

    batch: list[tuple] = []
    inserted = 0

    def flush() -> None:
        nonlocal inserted
        conn.executemany(
            """
            INSERT OR IGNORE INTO words
                (word, ipa, stress_from_end, stress_source, syllable_count)
            VALUES (?, ?, ?, ?, ?)
            """,
            batch,
        )
        conn.commit()
        inserted += len(batch)
        batch.clear()

    for word, stress_from_end, ipa, source in iter_polish_stress(dump_path):
        n = count_syllables(word)
        batch.append((word, ipa, stress_from_end, source, n))
        if len(batch) >= _BATCH:
            flush()
            if verbose:
                print(f"  wiktionary: {inserted:,} rows …", end="\r", flush=True)

    if batch:
        flush()

    if verbose:
        print(f"  wiktionary: {inserted:,} Polish entries ingested.    ")

    return inserted
