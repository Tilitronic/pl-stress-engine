"""
Offline pipeline CLI.

Steps
-----
1. ingest-wiktionary   Parse plwiktionary bz2 dump → master.db `words` table
2. ingest-polimorf     Parse PoliMorf .tab → `morphology` table, then propagate
3. export              master.db → data/processed/exceptions.json
4. (optional) compile  Run `cargo run -p builder` to produce exceptions.bin

If source files are missing, this script can auto-download them.

Usage examples:

    # Full pipeline (auto-download sources if missing):
    python -m pl_stress.builder \\
        --db      data/master.db \\
        --out     data/processed/exceptions.json

    # Wiktionary only (skip PoliMorf):
    python -m pl_stress.builder --no-polimorf
"""

from __future__ import annotations

import gzip
import re
import shutil
import sys
from pathlib import Path
from urllib.request import urlopen, urlretrieve

from .db import connect
from .ingest.wiktionary import ingest as ingest_wiktionary
from .ingest.polimorf import ingest_morphology, propagate_stress
from .export import export_json


WIKTIONARY_URL = (
    "https://dumps.wikimedia.org/plwiktionary/latest/"
    "plwiktionary-latest-pages-articles.xml.bz2"
)
MORFEUSZ_INDEX_URL = "https://download.sgjp.pl/morfeusz/current/"

DEFAULT_DUMP_PATH = Path("data/raw/plwiktionary-latest-pages-articles.xml.bz2")
DEFAULT_POLIMORF_PATH = Path("data/raw/polimorf-current.tab")

README_STATS_BEGIN = "<!-- EXCEPTIONS_STATS:BEGIN -->"
README_STATS_END = "<!-- EXCEPTIONS_STATS:END -->"


def _download(url: str, dst: Path, log, verbose: bool) -> None:
    dst.parent.mkdir(parents=True, exist_ok=True)
    if verbose:
        log(f"  downloading {url}")
        log(f"  → {dst}")
    urlretrieve(url, dst)


def _discover_polimorf_url() -> str:
    with urlopen(MORFEUSZ_INDEX_URL) as resp:
        html = resp.read().decode("utf-8", errors="replace")
    matches = re.findall(r'href="(polimorf-[0-9]{8}\.tab\.gz)"', html)
    if not matches:
        raise RuntimeError(
            f"Could not discover PoliMorf .tab.gz URL from {MORFEUSZ_INDEX_URL}"
        )
    latest = sorted(matches)[-1]
    return MORFEUSZ_INDEX_URL + latest


def _gunzip(src_gz: Path, dst_tab: Path, log, verbose: bool) -> None:
    dst_tab.parent.mkdir(parents=True, exist_ok=True)
    if verbose:
        log(f"  decompressing {src_gz.name}")
        log(f"  → {dst_tab}")
    with gzip.open(src_gz, "rb") as f_in, open(dst_tab, "wb") as f_out:
        shutil.copyfileobj(f_in, f_out)


def _count_exceptions_entries(out_path: Path) -> tuple[int, int]:
    """Return `(total_entries, with_ipa_entries)` from exceptions.json text.

    We intentionally count by key-pattern occurrences instead of full JSON parse,
    because upstream dumps can occasionally produce duplicate object keys that some
    strict parsers reject.
    """
    text = out_path.read_text(encoding="utf-8")
    total = len(re.findall(r'"stress_idx"\s*:', text))
    null_ipa = len(re.findall(r'"ipa"\s*:\s*null', text))
    with_ipa = max(0, total - null_ipa)
    return total, with_ipa


def _upsert_readme_exception_stats(out_path: Path, log, verbose: bool) -> None:
    """Insert/update the auto-generated exception stats block in README.md."""
    repo_root = Path(__file__).resolve().parents[1]
    readme_path = repo_root / "README.md"
    if not readme_path.exists() or not out_path.exists():
        return

    total, with_ipa = _count_exceptions_entries(out_path)
    generated_block = (
        f"{README_STATS_BEGIN}\n"
        "- Exception dictionary entries: **{total:,}**\n"
        "- Entries with IPA: **{with_ipa:,}**\n"
        "- Source datasets: Polish Wiktionary dump + PoliMorf morphological dictionary\n"
        "- Generated from: `data/processed/exceptions.json` by `python -m pl_stress.builder`\n"
        f"{README_STATS_END}"
    ).format(total=total, with_ipa=with_ipa)

    readme_text = readme_path.read_text(encoding="utf-8")
    block_re = re.compile(
        rf"{re.escape(README_STATS_BEGIN)}[\\s\\S]*?{re.escape(README_STATS_END)}",
        re.MULTILINE,
    )

    if block_re.search(readme_text):
        updated = block_re.sub(generated_block, readme_text)
    else:
        anchor = "## Data Pipeline"
        insert = (
            "\n## Exception Dictionary (Auto Stats)\n\n"
            f"{generated_block}\n"
        )
        if anchor in readme_text:
            updated = readme_text.replace(anchor, insert + "\n" + anchor, 1)
        else:
            updated = readme_text.rstrip() + "\n\n" + insert

    if updated != readme_text:
        readme_path.write_text(updated, encoding="utf-8")
        if verbose:
            log(f"  → updated README exception stats ({total:,} entries)")


def _ensure_wiktionary_dump(path: Path, auto_download: bool, log, verbose: bool) -> Path:
    if path.exists():
        return path
    if not auto_download:
        raise FileNotFoundError(f"Wiktionary dump not found: {path}")
    _download(WIKTIONARY_URL, path, log, verbose)
    return path


def _ensure_polimorf_tab(path: Path, auto_download: bool, log, verbose: bool) -> Path:
    if path.exists():
        return path

    gz_path = path.with_suffix(path.suffix + ".gz")
    if gz_path.exists():
        _gunzip(gz_path, path, log, verbose)
        return path

    if not auto_download:
        raise FileNotFoundError(f"PoliMorf tab file not found: {path}")

    url = _discover_polimorf_url()
    discovered_name = Path(url).name
    downloaded_gz = path.parent / discovered_name
    _download(url, downloaded_gz, log, verbose)
    _gunzip(downloaded_gz, path, log, verbose)
    return path


def build(
    dump_path: Path,
    polimorf_path: Path | None = None,
    db_path: Path = Path("data/master.db"),
    out_path: Path = Path("data/processed/exceptions.json"),
    auto_download: bool = True,
    verbose: bool = True,
) -> int:
    """Run the full pipeline.  Returns number of entries exported."""

    def log(msg: str) -> None:
        if verbose:
            print(msg, file=sys.stderr)

    dump_path = _ensure_wiktionary_dump(Path(dump_path), auto_download, log, verbose)

    if polimorf_path is not None:
        polimorf_path = _ensure_polimorf_tab(
            Path(polimorf_path), auto_download, log, verbose
        )

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
        log("[2/3] PoliMorf not configured — skipping")

    log("[3/3] Exporting to JSON…")
    n_out = export_json(conn, out_path, verbose=verbose)
    log(f"  → {n_out:,} entries written to {out_path}")
    _upsert_readme_exception_stats(out_path, log, verbose)

    conn.close()
    return n_out


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Build Polish stress master DB + export")
    parser.add_argument(
        "--dump",
        default=str(DEFAULT_DUMP_PATH),
        help=f"plwiktionary bz2 dump path (default: {DEFAULT_DUMP_PATH})",
    )
    parser.add_argument(
        "--polimorf",
        default=str(DEFAULT_POLIMORF_PATH),
        help=f"PoliMorf .tab file path (default: {DEFAULT_POLIMORF_PATH})",
    )
    parser.add_argument(
        "--no-polimorf",
        action="store_true",
        help="Skip PoliMorf ingestion/propagation",
    )
    parser.add_argument(
        "--no-download",
        action="store_true",
        help="Do not auto-download missing source files",
    )
    parser.add_argument("--db", default="data/master.db", help="Master SQLite DB path")
    parser.add_argument("--out", default="data/processed/exceptions.json",
                        help="Output JSON path (consumed by cargo run -p builder)")
    args = parser.parse_args()

    build(
        dump_path=Path(args.dump),
        polimorf_path=None if args.no_polimorf else Path(args.polimorf),
        db_path=Path(args.db),
        out_path=Path(args.out),
        auto_download=not args.no_download,
    )


if __name__ == "__main__":
    main()

