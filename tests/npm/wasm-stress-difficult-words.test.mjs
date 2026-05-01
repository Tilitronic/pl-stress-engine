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
  assert.equal(typeof wasm.stressInfo, "function");
});

test("difficult Polish words have expected stress-from-end", async (t) => {
  for (const sample of difficultPolishWords) {
    await t.test(`${sample.word} (${sample.category})`, () => {
      const info = wasm.stressInfo(sample.word);

      assert.equal(typeof info, "object");
      assert.equal(info.word, sample.word);
      assert.equal(info.stressFromEnd, sample.expectedStressFromEnd);

      const expectedIndex =
        info.syllables.length - sample.expectedStressFromEnd;
      assert.equal(info.syllableIndex, expectedIndex);
    });
  }
});
