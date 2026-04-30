# pl-stress-engine

Polish word stress engine with:
- Rust core library
- Python bindings (PyO3)
- WebAssembly bindings for browser usage
- Offline data pipeline that builds a stress exceptions dictionary from external lexical sources

## Project License

This repository is licensed under GNU AGPL v3.0 or later.
See [LICENSE](LICENSE).

Important:
- Code license and data licenses are not the same thing.
- AGPL covers this codebase.
- Third-party lexical data keeps its original licenses and attribution requirements.

Compatibility summary used by this project:
- PoliMorf is treated as permissive (BSD-style) data based on local source notes, pending version-level verification.
- Wiktionary data is treated as attribution/share-alike content; generated dictionary artifacts may carry those obligations.
- Therefore, AGPL applies to project code, while data-derived artifacts must also satisfy source-data terms.

## Third-Party Data and Legal Compliance

This project can ingest data from:
- Polish Wiktionary dump
- PoliMorf morphological dictionary

Before using, redistributing, or publishing artifacts built from these sources, read [THIRD_PARTY_DATA.md](THIRD_PARTY_DATA.md).

Compliance model used in this repository:
- Source dumps are treated as third-party content.
- Attribution and downstream license obligations must be preserved.
- Derived dictionary artifacts can inherit obligations from source datasets.

## Contributor Rules for Data

When contributing changes that touch external data sources:

1. Do not commit third-party dumps unless maintainers explicitly approve it.
2. Prefer keeping large raw files local under ignored paths.
3. Always record source URL, version, retrieval date, and license in [THIRD_PARTY_DATA.md](THIRD_PARTY_DATA.md).
4. Keep attribution text and required notices when distributing derived artifacts.
5. If license terms are unclear for a source, do not merge data-derived artifacts from that source until clarified.

## Data Pipeline

The pipeline builds:
- SQLite master DB
- processed JSON exceptions
- compact binary exceptions file embedded into Python and WASM crates

Run from repository root:

    python -m pl_stress.builder \
      --db data/master.db \
      --out data/processed/exceptions.json

Source behavior:
- If source files are missing, the builder auto-downloads:
    - Wiktionary dump from Wikimedia dumps
    - Latest PoliMorf `.tab.gz` from Morfeusz and decompresses to `.tab`
- Default source paths:
    - `data/raw/plwiktionary-latest-pages-articles.xml.bz2`
    - `data/raw/polimorf-current.tab`

Useful flags:
- `--no-polimorf` to skip PoliMorf ingestion
- `--no-download` to fail instead of downloading missing sources

Then compile binary dictionary:

    cargo run -p builder --release

This writes:
- data/processed/exceptions.json
- data/processed/exceptions.bin

## Build

### Rust workspace

    cargo build --workspace --release

### Python package

Use your intended Python interpreter explicitly:

    C:\ProgramData\anaconda3\python.exe -m maturin develop --release

### WASM package

Install wasm target once:

    rustup target add wasm32-unknown-unknown

Build WASM package:

    C:\Users\qualt\.cargo\bin\wasm-pack.exe build crates/wasm --target web --release

Generated package:
- crates/wasm/pkg/pl_stress_wasm.js
- crates/wasm/pkg/pl_stress_wasm_bg.wasm

## Python Usage

    import polish_stress

    info = polish_stress.stress_info("matematyka")
    print(info)

Fields:
- word
- syllables
- syllable_index
- stress_from_end
- ipa
- confidence

## Web Usage

Install package in your web app:

    npm install /absolute/path/to/pl-stress-engine/crates/wasm/pkg

Then in app code:

    import init, { stress, stressInfo } from "pl_stress_wasm";

    await init();
    console.log(stress("matematyka"));
    console.log(stressInfo("prezydent"));

## Polish Web Service (pnpm, TypeScript)

This repository includes a pnpm package:
- `@tilitronic/polish-web-service`

### Build prerequisites

The service uses WASM generated from Rust and requires:
- `data/processed/exceptions.bin` (generated dictionary)
- `wasm-pack` installed

Generate dictionary binary:

    cargo run -p builder --release

Build node-target WASM package:

    pnpm run build:wasm:node

### Local package registration

Register package globally for local testing:

    cd packages/polish-web-service
    pnpm link --global

Or create a local tarball:

    pnpm --filter @tilitronic/polish-web-service pack --pack-destination .local-packages

Install tarball in another local project:

    pnpm add /absolute/path/to/pl-stress-engine/.local-packages/tilitronic-polish-web-service-0.1.0.tgz

### Run service locally

From repository root:

    pnpm run dev:web-service

Default host/port:
- `HOST=0.0.0.0`
- `PORT=8787`

### Service API

Health check:

    GET /health

Stress info (query):

    GET /stress?word=matematyka

Stress info (JSON body):

    POST /stress
    Content-Type: application/json
    { "word": "matematyka" }

Syllable stress index only:

    GET /stress/index?word=matematyka

Example curl:

    curl "http://localhost:8787/stress?word=matematyka"

## Distribution Checklist

Before publishing wheels, wasm packages, or API services that embed generated dictionary artifacts:

1. Confirm source licenses for all ingested datasets are documented in [THIRD_PARTY_DATA.md](THIRD_PARTY_DATA.md).
2. Confirm required attribution text is included in your distribution.
3. Confirm derivative/share-alike obligations (if any) are satisfied for generated dictionary artifacts.
4. Confirm AGPL obligations are satisfied for code distribution and network use.

## Legal Note

This README is a practical compliance guide for contributors, not legal advice.
For commercial distribution or legal uncertainty, ask qualified legal counsel before release.
