"""
Ingest PoliMorf into the master SQLite database.

Two-phase process:

Phase 1 – morphology table
    Stream the .tab file and insert every (word, lemma, tag) row verbatim.
    This preserves the full PoliMorf dataset for future queries.

Phase 2 – stress propagation
    For each lemma already in `words` with a known stress, insert its
    inflected forms (from `morphology`) into `words` with source='propagated'.
    The stress is propagated from-end (not from-start) so it stays correct
    even when inflection changes the syllable count at the end of the word.

Both phases are idempotent (INSERT OR IGNORE / INSERT OR IGNORE).
"""

import sqlite3
from pathlib import Path

from pl_stress.parsers.polimorf import iter_entries
from pl_stress.syllabify import count_syllables

_BATCH = 5_000


# ── Phase 1: raw morphology ────────────────────────────────────────────────────

def ingest_morphology(
    conn: sqlite3.Connection, polimorf_path: Path, verbose: bool = True
) -> int:
    """
    Insert all (word, lemma, tag) rows from the PoliMorf .tab file.

    Returns the number of rows inserted.
    """
    polimorf_path = Path(polimorf_path)
    if not polimorf_path.exists():
        raise FileNotFoundError(f"PoliMorf file not found: {polimorf_path}")

    batch: list[tuple] = []
    total = 0

    def flush() -> None:
        nonlocal total
        conn.executemany(
            "INSERT INTO morphology (word, lemma, tag) VALUES (?, ?, ?)",
            batch,
        )
        conn.commit()
        total += len(batch)
        batch.clear()

    for word, lemma, tag in iter_entries(polimorf_path):
        batch.append((word, lemma, tag))
        if len(batch) >= _BATCH:
            flush()
            if verbose:
                print(f"  polimorf morphology: {total:,} rows …", end="\r", flush=True)

    if batch:
        flush()

    if verbose:
        print(f"  polimorf morphology: {total:,} rows inserted.    ")

    return total


# ── Phase 2: stress propagation ────────────────────────────────────────────────

def propagate_stress(conn: sqlite3.Connection, verbose: bool = True) -> int:
    """
    Propagate stress from known lemmas in `words` to their inflected forms
    in `morphology`, inserting new rows into `words` with source='propagated'.

    Stress is kept in *from-end* coordinates so adjustment across different
    syllable counts is straightforward.

    Returns the number of new `words` rows inserted.
    """
    # Fetch all lemmas that have a known stress entry
    lemmas = conn.execute(
        "SELECT word, stress_from_end, syllable_count FROM words WHERE stress_from_end IS NOT NULL"
    ).fetchall()

    batch: list[tuple] = []
    inserted = 0

    def flush() -> None:
        nonlocal inserted
        conn.executemany(
            """
            INSERT OR IGNORE INTO words
                (word, lemma, stress_from_end, stress_source, syllable_count)
            VALUES (?, ?, ?, 'propagated', ?)
            """,
            batch,
        )
        conn.commit()
        inserted += len(batch)
        batch.clear()

    for row in lemmas:
        lemma = row["word"]
        stress_from_end: int = row["stress_from_end"]
        lemma_n: int = row["syllable_count"] or count_syllables(lemma)

        # Get all inflected forms of this lemma from PoliMorf
        forms = conn.execute(
            "SELECT DISTINCT word FROM morphology WHERE lemma = ?", (lemma,)
        ).fetchall()

        for form_row in forms:
            form = form_row["word"]
            if form == lemma:
                continue  # lemma itself already present
            form_n = count_syllables(form)
            if form_n == 0:
                continue

            if form_n == lemma_n:
                adjusted = stress_from_end
            else:
                # Keep stress position relative to end; clamp to [1, form_n]
                adjusted = max(1, min(stress_from_end, form_n))

            batch.append((form, lemma, adjusted, form_n))
            if len(batch) >= _BATCH:
                flush()
                if verbose:
                    print(f"  propagated: {inserted:,} new forms …", end="\r", flush=True)

    if batch:
        flush()

    if verbose:
        print(f"  propagated: {inserted:,} new word forms added.    ")

    return inserted
