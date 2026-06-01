import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compileFromFile } from "json-schema-to-typescript";

const scriptDir = fileURLToPath(new URL(".", import.meta.url));
const root = resolve(scriptDir, "..");
const schemaPath = resolve(
  root,
  "crates/wasm/generated/word-lookup-result.schema.json",
);
const outputPath = resolve(root, "crates/wasm/generated/contracts.d.ts");

const ts = await compileFromFile(schemaPath, {
  bannerComment:
    "/* eslint-disable */\n/* tslint:disable */\n// Generated from crates/wasm/generated/word-lookup-result.schema.json\n",
  style: {
    singleQuote: false,
  },
});

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, ts, "utf8");
