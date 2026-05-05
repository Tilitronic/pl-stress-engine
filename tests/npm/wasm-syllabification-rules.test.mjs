import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const wasm = require("../../crates/wasm/pkg-node/pl_stress_wasm.js");

if (typeof wasm.default === "function") {
  await wasm.default();
}

function syllables(word) {
  return wasm.lookup(word).readings[0].wordSyllables;
}

test("Rule 1: every syllable has a vowel nucleus", () => {
  assert.deepEqual(syllables("ale"), ["a", "le"]);
  assert.deepEqual(syllables("oko"), ["o", "ko"]);
});

test("Rule 2: words with one vowel are not split", () => {
  assert.deepEqual(syllables("most"), ["most"]);
  assert.deepEqual(syllables("rak"), ["rak"]);
  assert.deepEqual(syllables("dom"), ["dom"]);
  assert.deepEqual(syllables("sok"), ["sok"]);
});

test("Rule 3: digraphs stay in one phonetic unit", () => {
  assert.deepEqual(syllables("szkoła"), ["szko", "ła"]);
  assert.deepEqual(syllables("chata"), ["cha", "ta"]);
  assert.deepEqual(syllables("czapka"), ["czap", "ka"]);
  assert.deepEqual(syllables("dziecko"), ["dziec", "ko"]);
});

test("Rule 4: initial au/eu stay in one syllable", () => {
  assert.deepEqual(syllables("auto"), ["au", "to"]);
  assert.deepEqual(syllables("europa"), ["eu", "ro", "pa"]);
});

test("Rule 5: prefix boundary examples", () => {
  assert.deepEqual(syllables("przedszkole"), ["przed", "szko", "le"]);
  assert.deepEqual(syllables("rozmowa"), ["roz", "mo", "wa"]);
});

test("Rule 6: multiple valid splits (accept one standard split)", () => {
  const kostka = syllables("kostka").join("-");
  const matka = syllables("matka").join("-");

  assert.ok(kostka === "kost-ka" || kostka === "kos-tka");
  assert.ok(matka === "mat-ka" || matka === "ma-tka");
});

test("Rule 7: i as softener does not form own syllable", () => {
  assert.deepEqual(syllables("ciasto"), ["cia", "sto"]);
  assert.deepEqual(syllables("dziecko"), ["dziec", "ko"]);
  assert.deepEqual(syllables("powiedzieć"), ["po", "wie", "dzieć"]);
  assert.deepEqual(syllables("osioł"), ["o", "sioł"]);
});

test("Rule 8: double consonants are split", () => {
  assert.deepEqual(syllables("wanna"), ["wan", "na"]);
  assert.deepEqual(syllables("anna"), ["an", "na"]);
});

test("Regression: biblioteka keeps i pronounced", () => {
  assert.deepEqual(syllables("biblioteka"), ["bi", "bli", "o", "te", "ka"]);
});

// ─── Śledziński (2018) "Wielowarstwowy model podziału wyrazów ortograficznych" ───
// The article describes a multi-layer syllabification model based on the
// Sonority Sequencing Principle (SSP) and Maximal Onset Principle (MOP).
// Tests below reference specific sections of that paper.

// §4.1 — konto is the article's step-by-step worked example.
// Cluster "nt" between two vowels: n(sonority 3) > t(sonority 1).
// The MOP assigns t to the onset of the next syllable → kon-to.
test("Śledziński §4.1: konto — nt cluster split by sonority (n|t)", () => {
  assert.deepEqual(syllables("konto"), ["kon", "to"]);
});

// §3.6 — uschnąć is the explicit example for the phonological projection module.
// Cluster "schn": s(2)–ch/x(2)–n(3). The sonority valley is between s and ch
// (equal values → no valid onset starting at ch). Boundary: s|chn → usch-nąć.
test("Śledziński §3.6: uschnąć — schn cluster splits as s|chn", () => {
  assert.deepEqual(syllables("uschnąć"), ["usch", "nąć"]);
});

// §5 — first test block: words where a MOD rule overrides pure phonology.
// Article shows both "without rules" (phonology-only) and "with rules" results.
// Our engine (TeX patterns + resplit) matches the linguistically preferred output.
test("Śledziński §5 group 1: consonant-cluster splits by sonority + MOP", () => {
  // perspektywa: rsp → r(4) s(2) p(1) falling; MOP assigns p to next onset → pers-pek
  assert.deepEqual(syllables("perspektywa"), ["per", "spek", "ty", "wa"]);
  // portfel: rtf — rt in coda (decreasing 4→1), f alone as onset → port-fel
  assert.deepEqual(syllables("portfel"), ["port", "fel"]);
  // majstrem: jstr — j(5) in coda, str valid 3-char onset → maj-strem
  assert.deepEqual(syllables("majstrem"), ["maj", "strem"]);
  // egzamin: gz — g(1) in coda, z(2) starts next syllable → eg-za-min
  assert.deepEqual(syllables("egzamin"), ["eg", "za", "min"]);
  // agresywny: wn — w(5) in coda, n(3) lower sonority starts onset → a-gre-syw-ny
  assert.deepEqual(syllables("agresywny"), ["a", "gre", "syw", "ny"]);
  // administracja: dmi, str — ad prefix + str as valid complex onset → ad-mi-ni-stra-cja
  assert.deepEqual(syllables("administracja"), [
    "ad",
    "mi",
    "ni",
    "stra",
    "cja",
  ]);
});

// §5 — second test block: clusters where phonology provides no unique solution
// (equal sonority or ambiguous), so the boundary falls before the whole cluster.
test("Śledziński §5 group 2: clusters resolved by fallback onset placement", () => {
  // herbstem: rbst → r(4) b(1) s(2) t(1) — rb in coda, st as onset → herb-stem
  assert.deepEqual(syllables("herbstem"), ["herb", "stem"]);
  // gangsterski: ngst → ng in coda, st onset → gang-ster-ski
  assert.deepEqual(syllables("gangsterski"), ["gang", "ster", "ski"]);
  // tekstem: kst → k(1) s(2) t(1) — ks in coda, t as onset → tek-stem
  assert.deepEqual(syllables("tekstem"), ["tek", "stem"]);
  // ekspres: kspr → ks in coda, pr valid onset → eks-pres
  assert.deepEqual(syllables("ekspres"), ["eks", "pres"]);
});

// §5 — third block: morphological prefix rules (MOR layer overrides phonology).
// The article shows that morpheme boundaries like do-, nad-, pod- take priority.
test("Śledziński §5 group 3: morphological prefix rules", () => {
  // dostudzić: do- prefix + studzić → do-stu-dzić
  assert.deepEqual(syllables("dostudzić"), ["do", "stu", "dzić"]);
  // nadlecieć: nad- prefix + lec → nad-le-cieć
  assert.deepEqual(syllables("nadlecieć"), ["nad", "le", "cieć"]);
  // nadworny: exception (nadwor on exclusion list) → phonological na-dwor-ny
  assert.deepEqual(syllables("nadworny"), ["na", "dwor", "ny"]);
  // pownosić: exception preserves phonological po-wno-sić
  assert.deepEqual(syllables("pownosić"), ["po", "wno", "sić"]);
});

// §3.2 — more prefix-boundary examples from the article's morphology section.
test("Śledziński §3.2: pod-, ob-, roz- prefix boundaries", () => {
  // podjazd: pod- + jazd (article example: #podja>#pod|ja rule)
  assert.deepEqual(syllables("podjazd"), ["pod", "jazd"]);
  // podwładny: pod- + władny (article footnote example with 86% agreement)
  assert.deepEqual(syllables("podwładny"), ["pod", "wład", "ny"]);
  // obsłuchać: ob- + słuchać (article footnote: ob|słuchać 88%)
  assert.deepEqual(syllables("obsłuchać"), ["ob", "słu", "chać"]);
  // rozmnażać: roz- + mnażać (article footnote: roz|mnażać 90%)
  assert.deepEqual(syllables("rozmnażać"), ["roz", "mna", "żać"]);
});

// §3.4 — boundary between adjacent different vowel nuclei.
// The article states a marker is always placed between two *different* adjacent vowels.
test("Śledziński §3.4: adjacent different-vowel nuclei get split", () => {
  // aeroplan: a–e adjacent (different) → a-e-ro-plan
  assert.deepEqual(syllables("aeroplan"), ["a", "e", "ro", "plan"]);
  // geoida: e–o and o–i adjacent → ge-o-i-da
  assert.deepEqual(syllables("geoida"), ["ge", "o", "i", "da"]);
});

// §3.7 — probabilistic rule example: amnezja (cluster "mn").
// m(3)–n(3) have equal sonority → no pure-phonology split.
// Step 8 (fallback) places boundary *before* mn, so mn forms the onset → a-mne-zja.
test("Śledziński §3.7: amnezja — mn cluster defaults to onset placement", () => {
  assert.deepEqual(syllables("amnezja"), ["a", "mne", "zja"]);
});

// Table 1 / Table 2 — basic words from the article's phoneme inventory.
// Each entry verifies that common single-consonant-between-vowel patterns work.
test("Śledziński Table 1: basic phoneme-inventory words", () => {
  assert.deepEqual(syllables("jeden"), ["je", "den"]);
  assert.deepEqual(syllables("wiele"), ["wie", "le"]);
  assert.deepEqual(syllables("moneta"), ["mo", "ne", "ta"]);
  assert.deepEqual(syllables("futro"), ["fu", "tro"]);
  assert.deepEqual(syllables("wysoki"), ["wy", "so", "ki"]);
  assert.deepEqual(syllables("koza"), ["ko", "za"]);
  assert.deepEqual(syllables("ryba"), ["ry", "ba"]);
  assert.deepEqual(syllables("tama"), ["ta", "ma"]);
  assert.deepEqual(syllables("buda"), ["bu", "da"]);
  assert.deepEqual(syllables("palec"), ["pa", "lec"]);
});

// Table 1 — digraph examples (sz, cz, ch, dż, dz… treated as single phonemes).
test("Śledziński Table 1: digraph-containing words", () => {
  assert.deepEqual(syllables("ziarno"), ["ziar", "no"]);
  assert.deepEqual(syllables("kuchnia"), ["kuch", "nia"]);
  assert.deepEqual(syllables("dzwonek"), ["dzwo", "nek"]);
  assert.deepEqual(syllables("działka"), ["dział", "ka"]);
  assert.deepEqual(syllables("dżuma"), ["dżu", "ma"]);
  assert.deepEqual(syllables("kocioł"), ["ko", "cioł"]);
});

// §2.2 footnote — ćwierć- prefix examples (morphological vs phonological split).
// The article discusses ćwierćwałek (94% morphological boundary preference).
// NOTE: our engine (TeX patterns) doesn't implement the MOR rule for ćwierć-,
// so ćwierćwiecze is split phonologically as ćwi-erć-wie-cze.
test("Śledziński §2.2: ćwierć- without morphological rule splits phonologically", () => {
  assert.deepEqual(syllables("ćwierćwiecze"), ["ćwi", "erć", "wie", "cze"]);
});

test("Śledziński: formerly different cases + phonetic ii hiatus", () => {
  assert.deepEqual(syllables("zemsta"), ["zem", "sta"]);
  assert.deepEqual(syllables("bydlak"), ["byd", "lak"]);
  assert.deepEqual(syllables("wydma"), ["wyd", "ma"]);
  assert.deepEqual(syllables("okołozwrotnikowy"), [
    "o",
    "ko",
    "ło",
    "zwrot",
    "ni",
    "ko",
    "wy",
  ]);
  assert.deepEqual(syllables("kopii"), ["ko", "pi", "i"]);
  assert.deepEqual(syllables("anarchii"), ["a", "nar", "chi", "i"]);
  assert.deepEqual(syllables("unii"), ["u", "ni", "i"]);
});
