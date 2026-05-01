# Syllabification Rules Audit (Temporary)

This temporary file tracks rules and example cases extracted from two scientific articles:

1. Rozwój programu dla podziału tekstów w języku polskim na sylaby
2. Wielowarstwowy model podziału wyrazów ortograficznych języka polskiego na sylaby

Status legend:

- PASS: engine output matches article expectation from provided excerpts
- CHECK: case present, but article excerpt allows variants or does not force one split
- DIFF: engine output differs from article-preferred split in provided excerpts

## Article 1 (rule-oriented set)

### Rule 1: every syllable contains a vowel nucleus

- ale -> a-le (PASS)
- oko -> o-ko (PASS)

### Rule 2: one-vowel words are not split

- most -> most (PASS)
- rak -> rak (PASS)
- dom -> dom (PASS)
- sok -> sok (PASS)

### Rule 3: do not split digraphs representing one sound

- szkola -> szko-la (PASS)
- chata -> cha-ta (PASS)
- czapka -> czap-ka (PASS)
- dziecko -> dziec-ko (PASS)

### Rule 4: initial au/eu stay in one syllable

- auto -> au-to (PASS)
- europa -> eu-ro-pa (PASS)

### Rule 5: separate prefix from stem

- przedszkole -> przed-szko-le (PASS)
- rozmowa -> roz-mo-wa (PASS)

### Rule 6: multiple valid splits can exist

- kostka -> kost-ka or kos-tka (CHECK, engine: kost-ka)
- matka -> mat-ka or ma-tka (CHECK, engine: mat-ka)

### Rule 7: softening i does not form own syllable

- ciasto -> cia-sto (PASS)
- dziecko -> dziec-ko (PASS)
- powiedziec -> po-wie-dziec (PASS)
- osiol -> o-siol (PASS)

### Rule 8: identical consonants are split

- wanna -> wan-na (PASS)
- anna -> an-na (PASS)

### Open/closed syllable examples from the article text

- mostek -> mo-stek (PASS)
- sarna -> sar-na (PASS)
- kulka -> kul-ka (PASS)
- rysunek -> ry-su-nek (PASS)
- szelest -> sze-lest (PASS)

## Article 2 (multi-layer SSP/MOP model)

### Core SSP/MOP examples

- konto -> kon-to (PASS)
- perspektywa -> per-spek-ty-wa (PASS)
- portfel -> port-fel (PASS)
- majstrem -> maj-strem (PASS)
- administracja -> ad-mi-ni-stra-cja (PASS)
- egzamin -> eg-za-min (PASS)
- agresywny -> a-gre-syw-ny (PASS)
- amnezja -> a-mne-zja (PASS)
- uschnac -> usch-nac (PASS)

### Morphology-layer examples

- dostudzic -> do-stu-dzic (PASS)
- nadleciec -> nad-le-ciec (PASS)
- nadworny -> na-dwor-ny (PASS, exception behavior)
- podjazd -> pod-jazd (PASS)
- podwladny -> pod-wlad-ny (PASS)
- obsluchac -> ob-slu-chac (PASS)
- rozmnażac -> roz-mna-zac (PASS)

### Vowel-adjacency examples

- aeroplan -> a-e-ro-plan (PASS)
- geoida -> ge-o-i-da (PASS)
- samoistny -> sa-mo-ist-ny (PASS)

### Inventory/table examples

- ziarno -> ziar-no (PASS)
- kuchnia -> kuch-nia (PASS)
- dzwonek -> dzwo-nek (PASS)
- dzialka -> dzial-ka (PASS)
- dzuma -> dzu-ma (PASS)
- kociol -> ko-ciol (PASS)
- jeden -> je-den (PASS)
- wiele -> wie-le (PASS)
- moneta -> mo-ne-ta (PASS)
- futro -> fu-tro (PASS)
- wysoki -> wy-so-ki (PASS)
- koza -> ko-za (PASS)
- ryba -> ry-ba (PASS)
- tama -> ta-ma (PASS)
- buda -> bu-da (PASS)
- palec -> pa-lec (PASS)

### Formerly different cases now aligned

- zemsta -> zem-sta (PASS)
- bydlak -> byd-lak (PASS)
- wydma -> wyd-ma (PASS)
- okolozwrotnikowy -> o-ko-lo-zwrot-ni-ko-wy (PASS)

### Phonetic-priority vowel hiatus for adjacent identical vowels

- kopii -> ko-pi-i (PASS)
- anarchii -> a-nar-chi-i (PASS)
- unii -> u-ni-i (PASS)

## Mapping to implemented automated tests

Rust core tests:

- crates/core/src/syllabify.rs
  - article_rules_1_to_8_smoke_suite
  - article_open_and_closed_syllables_examples
  - sledz\_\* tests (SSP/MOP + morphology + table cases)
  - article2_formerly_different_cases_now_aligned

Node/WASM integration tests:

- tests/npm/wasm-syllabification-rules.test.mjs
- tests/npm/wasm-stress-difficult-words.test.mjs

Temporary note:

- This file is intentionally temporary and can be removed after final conformance decisions.
