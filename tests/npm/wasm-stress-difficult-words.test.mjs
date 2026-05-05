import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";

import { difficultPolishWords } from "./difficult-polish-words.mjs";

const require = createRequire(import.meta.url);
const wasm = require("../../crates/wasm/pkg-node/pl_stress_wasm.js");

if (typeof wasm.default === "function") {
  await wasm.default();
}

test("wasm package exposes stress functions", () => {
  assert.equal(typeof wasm.stress, "function");
  assert.equal(typeof wasm.lookup, "function");
  assert.equal(typeof wasm.mark, "function");
});

test("difficult Polish words have expected stress-from-end", async (t) => {
  for (const sample of difficultPolishWords) {
    await t.test(`${sample.word} (${sample.category})`, () => {
      const result = wasm.lookup(sample.word);
      const info = result.readings[0];

      assert.equal(typeof result, "object");
      assert.equal(result.form, sample.word.toLowerCase());
      assert.equal(info.stressFromEnd, sample.expectedStressFromEnd);

      const expectedIndex =
        info.wordSyllables.length - sample.expectedStressFromEnd;
      assert.equal(info.syllableIndex, expectedIndex);
      assert.ok(Array.isArray(info.ipaSyllables));
      assert.equal(info.ipaSyllables.length, info.wordSyllables.length);
    });
  }
});
