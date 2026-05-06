import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const wasm = require("../../crates/wasm/pkg-node/pl_stress_wasm.js");

if (typeof wasm.default === "function") {
  await wasm.default();
}

test("wasm package exposes batch functions", () => {
  assert.equal(typeof wasm.markBatch, "function");
  assert.equal(typeof wasm.stressBatch, "function");
  assert.equal(typeof wasm.lookupBatch, "function");
});

test("markBatch returns stress-marked strings", () => {
  const words = ["matematyka", "chodziliście", "GPS"];
  const result = wasm.markBatch(words);
  assert.ok(Array.isArray(result), "markBatch result should be an Array");
  assert.equal(result.length, 3);
  // matematyka — penultimate stress → matemátyka
  assert.equal(result[0], wasm.mark("matematyka"));
  // chodziliście — antepenultimate → chódziliście
  assert.equal(result[1], wasm.mark("chodziliście"));
  // GPS abbreviation
  assert.equal(result[2], wasm.mark("GPS"));
});

test("markBatch result matches individual mark() calls", () => {
  const words = [
    "biblioteka",
    "ekspres",
    "zdecydowalibyśmy",
    "foyer",
    "przyrodoznawstwo",
  ];
  const batch = wasm.markBatch(words);
  for (let i = 0; i < words.length; i++) {
    assert.equal(batch[i], wasm.mark(words[i]), `mismatch for "${words[i]}"`);
  }
});

test("stressBatch returns Int32Array of syllable indices", () => {
  const words = ["matematyka", "chodziliście", "mama"];
  const result = wasm.stressBatch(words);
  assert.ok(
    result instanceof Int32Array,
    "stressBatch result should be Int32Array",
  );
  assert.equal(result.length, 3);
  // each index should match individual stress() calls
  for (let i = 0; i < words.length; i++) {
    assert.equal(
      result[i],
      wasm.stress(words[i]),
      `index mismatch for "${words[i]}"`,
    );
  }
});

test("stressBatch result matches individual stress() calls", () => {
  const words = ["biblioteka", "ekspres", "portfel", "nauka", "polskie"];
  const batch = wasm.stressBatch(words);
  for (let i = 0; i < words.length; i++) {
    assert.equal(
      batch[i],
      wasm.stress(words[i]),
      `stress mismatch for "${words[i]}"`,
    );
  }
});

test("lookupBatch returns array of lookup result objects", () => {
  const words = ["matematyka", "chodziliście"];
  const result = wasm.lookupBatch(words);
  assert.ok(Array.isArray(result), "lookupBatch result should be an Array");
  assert.equal(result.length, 2);
  for (const r of result) {
    assert.ok(typeof r.form === "string", "each result has a form string");
    assert.ok(Array.isArray(r.readings), "each result has a readings array");
    assert.ok(r.readings.length > 0, "each result has at least one reading");
  }
});

test("lookupBatch result matches individual lookup() calls", () => {
  const words = [
    "biblioteka",
    "ekspres",
    "portfel",
    "amnezja",
    "zdecydowalibyśmy",
  ];
  const batch = wasm.lookupBatch(words);
  for (let i = 0; i < words.length; i++) {
    const single = wasm.lookup(words[i]);
    assert.equal(batch[i].form, single.form, `form mismatch for "${words[i]}"`);
    assert.equal(
      batch[i].readings.length,
      single.readings.length,
      `readings count mismatch for "${words[i]}"`,
    );
    assert.equal(
      batch[i].readings[0].stressedForm,
      single.readings[0].stressedForm,
      `stressedForm mismatch for "${words[i]}"`,
    );
    assert.deepEqual(
      batch[i].readings[0].wordSyllables,
      single.readings[0].wordSyllables,
      `wordSyllables mismatch for "${words[i]}"`,
    );
  }
});

test("lookupBatch handles empty input", () => {
  const result = wasm.lookupBatch([]);
  assert.ok(Array.isArray(result));
  assert.equal(result.length, 0);
});

test("markBatch handles empty input", () => {
  const result = wasm.markBatch([]);
  assert.ok(Array.isArray(result));
  assert.equal(result.length, 0);
});

test("stressBatch handles empty input", () => {
  const result = wasm.stressBatch([]);
  assert.ok(result instanceof Int32Array);
  assert.equal(result.length, 0);
});
