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

## Exception Dictionary (Auto Stats)

<!-- EXCEPTIONS_STATS:BEGIN -->

- Exception dictionary entries: **158,983**
- Entries with IPA: **104,786**
- Source datasets: Polish Wiktionary dump + PoliMorf morphological dictionary
- Generated from: `data/processed/exceptions.json` by `python -m pl_stress.builder`
<!-- EXCEPTIONS_STATS:END -->

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

    result = polish_stress.lookup("matematyka")
    print(result["readings"][0]["stressed_form"])   # → "matemátyka"
    print(result["readings"][0]["syllable_index"])  # → 2
    print(result["readings"][0]["ipa_syllables"])   # → ["ma","tɛ","ˈma","tɨ","ka"]

    # Convenience shortcuts
    polish_stress.mark("matematyka")  # → "matemátyka"
    polish_stress.stress("matematyka")  # → 2  (0-based syllable index)

Fields returned by `lookup(word)["readings"][0]`:

- `syllable_index` — 0-based stressed syllable from start
- `stress_from_end` — 1-based from end (2 = penultimate)
- `syllable_count` — total syllable count (0 for clitics like "z", "w")
- `form` — normalised (lowercase) form
- `stressed_form` — form with combining acute U+0301 on the stressed vowel
- `word_syllables` — grapheme syllables aligned with `ipa_syllables`
- `ipa` — full IPA transcription
- `ipa_syllables` — IPA per syllable; stressed syllable prefixed with `ˈ`
- `tokens` — token-level phonetic detail (empty list for Polish)
- `morph` — UD morphological readings (empty list for Polish)
- `confidence` — `"exact"` | `"rule"` | `"default"`

## Web Usage

Install browser package:

    npm install @tilitronic/polish-stress-wasm

Then in app code:

    import { lookup, mark, stress, markBatch, stressBatch, lookupBatch } from "@tilitronic/polish-stress-wasm";

    console.log(lookup("matematyka"));
    console.log(mark("chodziliście"));   // → "chódziliście"
    console.log(stress("matematyka"));   // → 2

    // Batch processing — faster than calling mark/stress/lookup in a loop
    markBatch(["matematyka", "chodziliście"]);     // → ["matemátyka", "chódziliście"]
    stressBatch(["matematyka", "chodziliście"]);   // → Int32Array [2, 1]
    lookupBatch(["matematyka", "chodziliście"]);   // → Array of lookup result objects

`lookup(word)` returns an object with:

- `form` — normalised input
- `readings` — array with one element (Polish stress is deterministic)

Each reading has:

- `syllableIndex` — 0-based stressed syllable from start
- `stressFromEnd` — 1-based from end (2 = penultimate)
- `syllableCount` — 0 for zero-syllable words like "z", "w"
- `form` — normalised form
- `stressedForm` — combining acute on stressed vowel
- `wordSyllables` — grapheme syllables
- `ipa` — full IPA
- `ipaSyllables` — IPA per syllable; stressed prefixed with `ˈ`
- `tokens` — empty array (Polish)
- `morph` — empty array (Polish)
- `confidence` — `"exact"` | `"rule"` | `"default"`

### Batch functions

All three single-word functions have a batch variant that is significantly faster
when processing multiple words, because the JS↔WASM overhead is amortised:

| Single         | Batch                | Return type  |
| -------------- | -------------------- | ------------ |
| `mark(word)`   | `markBatch(words)`   | `string[]`   |
| `stress(word)` | `stressBatch(words)` | `Int32Array` |
| `lookup(word)` | `lookupBatch(words)` | object array |

```js
markBatch(["matematyka", "biblioteka", "GPS"]);
// → ["matemátyka", "biblióteka", "GPS"]

stressBatch(["matematyka", "biblioteka"]);
// → Int32Array [2, 2]

const results = lookupBatch(["ekspres", "portfel"]);
results[0].readings[0].wordSyllables; // → ["eks", "pres"]
results[1].readings[0].stressedForm; // → "pórtfel"
```

Example:

    {
      "form": "chodziliście",
      "readings": [{
        "syllableIndex": 1,
        "stressFromEnd": 3,
        "syllableCount": 4,
        "form": "chodziliście",
        "stressedForm": "chódziliście",
        "wordSyllables": ["cho", "dzi", "li", "ście"],
        "ipa": "xɔd͡zilʲiɕt͡ɕɛ",
        "ipaSyllables": ["xɔ", "ˈd͡zi", "lʲi", "ɕt͡ɕɛ"],
        "tokens": [],
        "morph": [],
        "confidence": "rule"
      }]
    }

## Distribution Checklist

Before publishing wheels, wasm packages, or API services that embed generated dictionary artifacts:

1. Confirm source licenses for all ingested datasets are documented in [THIRD_PARTY_DATA.md](THIRD_PARTY_DATA.md).
2. Confirm required attribution text is included in your distribution.
3. Confirm derivative/share-alike obligations (if any) are satisfied for generated dictionary artifacts.
4. Confirm AGPL obligations are satisfied for code distribution and network use.

## Legal Note

This README is a practical compliance guide for contributors, not legal advice.
For commercial distribution or legal uncertainty, ask qualified legal counsel before release.

## Linguistic References

The syllabification and stress tests in this repository were aligned against these references:

1. Śledziński, Daniel. "Rozwój programu dla podziału tekstów w języku polskim na sylaby." Vol. IX: 193.
2. Śledziński, Daniel. "Wielowarstwowy model podziału wyrazów ortograficznych języka polskiego na sylaby." Polonica (2018).
3. Wągiel, Marcin. "Międzynarodowy alfabet fonetyczny (IPA) w transkrypcji fonetycznej języka polskiego." W S. Gajda & I. Jokiel (Red.), Polonistyka wobec wyzwań współczesności: V Kongres Polonistyki Zagranicznej (2014): 134-145.
4. Nagórko, Alicja. Podręczna gramatyka języka polskiego. Wydawn. Naukowe PWN, 2010.
