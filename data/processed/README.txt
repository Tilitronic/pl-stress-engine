This file is a placeholder. Run the builder first:

    cargo run -p builder --release

The builder will generate:
  - data/processed/exceptions.bin   (used by wasm and python crates)
  - data/processed/exceptions.json  (human-readable debug copy)

Requirements:
  - data/raw/plwiktionary-latest-pages-articles.xml.bz2  (already present)
  - data/raw/PoliMorf-0.6.7.tab                          (download from zil.ipipan.waw.pl/PoliMorf)
