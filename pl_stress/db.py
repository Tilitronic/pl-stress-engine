"""
SQLite master database for Polish stress, IPA and morphology data.

Schema
------
words(word PK, lemma, ipa, stress_from_end, stress_source, syllable_count)
    One row per unique word form with all known linguistic data.

morphology(id PK, word, lemma, tag)
    Raw PoliMorf (word_form, lemma, grammatical_tag) rows — kept verbatim
    for reference and future re-processing.

stress_source values (ordered by decreasing reliability):
    'wiktionary_akc'   Explicit {{objaśnienie wymowy|AKCn}} template
    'wiktionary_ipa'   Derived from {{IPA3|...}} primary stress marker
    'propagated'       Inherited from lemma via PoliMorf inflection map
    'rule'             Produced by the grammar rule engine

This master DB is the canonical offline store.  The runtime binary
(exceptions.bin) is derived from it by the export step.
"""

import sqlite3
from pathlib import Path

DEFAULT_DB = Path("data/master.db")

_DDL = """
CREATE TABLE IF NOT EXISTS words (
    word            TEXT PRIMARY KEY,
    lemma           TEXT,
    ipa             TEXT,
    stress_from_end INTEGER,
    stress_source   TEXT,
    syllable_count  INTEGER
);

CREATE TABLE IF NOT EXISTS morphology (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    word    TEXT NOT NULL,
    lemma   TEXT NOT NULL,
    tag     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_morphology_lemma ON morphology(lemma);
CREATE INDEX IF NOT EXISTS idx_morphology_word  ON morphology(word);
"""


def connect(db_path: Path = DEFAULT_DB) -> sqlite3.Connection:
    """Open (or create) the master database.  Returns a live connection."""
    db_path = Path(db_path)
    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    # WAL mode: much faster concurrent writes + reads
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA synchronous=NORMAL")
    conn.executescript(_DDL)
    return conn
