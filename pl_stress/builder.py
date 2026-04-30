"""
Offline pipeline CLI.

Steps
-----
1. ingest-wiktionary   Parse plwiktionary bz2 dump → master.db `words` table
2. ingest-polimorf     Parse PoliMorf .tab → `morphology` table, then propagate
3. export              master.db → data/processed/exceptions.json
4. (optional) compile  Run `cargo run -p builder` to produce exceptions.bin

Usage examples:

    # Full pipeline:
    python -m pl_stress.builder \\
        --dump    data/raw/plwiktionary-latest-pages-articles.xml.bz2 \\
        --polimorf data/raw/PoliMorf-0.6.7.tab \\
        --db      data/master.db \\
        --out     data/processed/exceptions.json

    # Wiktionary only (no PoliMorf):
    python -m pl_stress.builder --dump data/raw/...xml.bz2
"""

from __future__ import annotations

import sys
from pathlib import Path

from .db import connect
from .ingest.wiktionary import ingest as ingest_wiktionary
from .ingest.polimorf import ingest_morphology, propagate_stress
from .export import export_json


def build(
    dump_path: Path,
    polimorf_path: Path | None = None,
    db_path: Path = Path("data/master.db"),
    out_path: Path = Path("data/processed/exceptions.json"),
    verbose: bool = True,
) -> int:
    """Run the full pipeline.  Returns number of entries exported."""

    def log(msg: str) -> None:
        if verbose:
            print(msg, file=sys.stderr)

    conn = connect(db_path)

    log("[1/3] Ingesting Wiktionary dump…")
    n_wiki = ingest_wiktionary(conn, dump_path, verbose=verbose)
    log(f"  → {n_wiki:,} entries")

    if polimorf_path and Path(polimorf_path).exists():
        log("[2/3] Ingesting PoliMorf…")
        ingest_morphology(conn, polimorf_path, verbose=verbose)
        log("  Propagating stress to inflected forms…")
        n_prop = propagate_stress(conn, verbose=verbose)
        log(f"  → {n_prop:,} new forms added")
    else:
        log("[2/3] PoliMorf not found — skipping (download from https://zil.ipipan.waw.pl/PoliMorf)")

    log("[3/3] Exporting to JSON…")
    n_out = export_json(conn, out_path, verbose=verbose)
    log(f"  → {n_out:,} entries written to {out_path}")

    conn.close()
    return n_out


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Build Polish stress master DB + export")
    parser.add_argument("--dump", required=True, help="plwiktionary bz2 dump path")
    parser.add_argument("--polimorf", default=None, help="PoliMorf .tab file path")
    parser.add_argument("--db", default="data/master.db", help="Master SQLite DB path")
    parser.add_argument("--out", default="data/processed/exceptions.json",
                        help="Output JSON path (consumed by cargo run -p builder)")
    args = parser.parse_args()

    build(
        dump_path=Path(args.dump),
        polimorf_path=Path(args.polimorf) if args.polimorf else None,
        db_path=Path(args.db),
        out_path=Path(args.out),
    )


if __name__ == "__main__":
    main()

