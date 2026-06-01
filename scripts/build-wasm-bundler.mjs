import { copyFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const scriptDir = fileURLToPath(new URL(".", import.meta.url));
const root = resolve(scriptDir, "..");
const dictPath = resolve(root, "data/processed/exceptions.bin");
const generatedContractsPath = resolve(root, "crates/wasm/generated/contracts.d.ts");
const pkgBundlerPackageJsonPath = resolve(
  root,
  "crates/wasm/pkg-bundler/package.json",
);
const pkgBundlerContractsPath = resolve(root, "crates/wasm/pkg-bundler/contracts.d.ts");
const pkgBundlerTypesPath = resolve(root, "crates/wasm/pkg-bundler/pl_stress_wasm.d.ts");

function patchWasmTypes(typesPath) {
  if (!existsSync(typesPath)) {
    return;
  }

  let content = readFileSync(typesPath, "utf8");

  if (!content.includes('import type { WordLookupResult } from "./contracts";')) {
    content = `${content}import type { WordLookupResult } from "./contracts";\n`;
  }

  content = content.replace(
    "export function lookup(word: string): any;",
    "export function lookup(word: string): WordLookupResult;",
  );
  content = content.replace(
    "export function lookupBatch(words: Array<any>): any;",
    "export function lookupBatch(words: Array<any>): WordLookupResult[];",
  );

  writeFileSync(typesPath, content, "utf8");
}

const wasmPackCandidates = [
  "wasm-pack",
  resolve(homedir(), ".cargo", "bin", "wasm-pack.exe"),
  resolve(homedir(), ".cargo", "bin", "wasm-pack"),
];

const wasmPack = wasmPackCandidates.find((candidate) => {
  if (candidate === "wasm-pack") {
    const probe = spawnSync(candidate, ["--version"], {
      stdio: "ignore",
      shell: true,
    });
    return probe.status === 0;
  }
  return existsSync(candidate);
});

if (!wasmPack) {
  console.error("Cannot find wasm-pack.");
  console.error("Install it first, e.g. with the official installer.");
  process.exit(1);
}

const cargoBin = resolve(homedir(), ".cargo", "bin");
const pathSep = process.platform === "win32" ? ";" : ":";
const childPath = `${cargoBin}${pathSep}${process.env.PATH ?? ""}`;

if (!existsSync(dictPath)) {
  console.error("Missing required dictionary file:");
  console.error(`  ${dictPath}`);
  console.error(
    "\nGenerate it first with your data pipeline, then re-run this command.",
  );
  process.exit(1);
}

if (!existsSync(generatedContractsPath)) {
  console.error("Missing generated contract types:");
  console.error(`  ${generatedContractsPath}`);
  console.error("Run `pnpm run generate:contracts` first.");
  process.exit(1);
}

const result = spawnSync(
  wasmPack,
  [
    "build",
    "crates/wasm",
    "--target",
    "bundler",
    "--release",
    "--out-dir",
    "pkg-bundler",
  ],
  {
    stdio: "inherit",
    shell: true,
    cwd: root,
    env: {
      ...process.env,
      PATH: childPath,
    },
  },
);

if (typeof result.status === "number") {
  if (result.status === 0 && existsSync(pkgBundlerPackageJsonPath)) {
    copyFileSync(generatedContractsPath, pkgBundlerContractsPath);
    patchWasmTypes(pkgBundlerTypesPath);

    const pkgJson = JSON.parse(readFileSync(pkgBundlerPackageJsonPath, "utf8"));
    pkgJson.name = "@tilitronic/polish-stress-wasm";
    pkgJson.main = "pl_stress_wasm.js";
    pkgJson.module = "pl_stress_wasm.js";
    pkgJson.exports = {
      ".": {
        types: "./pl_stress_wasm.d.ts",
        import: "./pl_stress_wasm.js",
        default: "./pl_stress_wasm.js",
      },
      "./contracts": "./contracts.d.ts",
      "./pl_stress_wasm_bg.wasm": "./pl_stress_wasm_bg.wasm",
    };
    pkgJson.description =
      "WebAssembly bindings for Polish stress engine — bundler target (Vite, webpack, etc.)";
    pkgJson.author = "Tilitronic";
    pkgJson.license = "AGPL-3.0-or-later";
    pkgJson.repository = {
      type: "git",
      url: "https://github.com/Tilitronic/pl-stress-engine.git",
      directory: "crates/wasm",
    };
    pkgJson.keywords = [
      "polish",
      "stress",
      "syllable",
      "hyphenation",
      "ipa",
      "wasm",
      "webassembly",
      "nlp",
      "vite",
      "bundler",
    ];
    pkgJson.engines = { node: ">=16.0.0" };
    pkgJson.publishConfig = { access: "public" };
    pkgJson.files = [
      "pl_stress_wasm_bg.wasm",
      "pl_stress_wasm.js",
      "pl_stress_wasm_bg.js",
      "pl_stress_wasm.d.ts",
      "contracts.d.ts",
    ];
    pkgJson.scripts = {
      pretest: "node ../../../scripts/build-wasm-bundler.mjs",
      test: "node --test ../../../tests/npm/wasm-stress-difficult-words.test.mjs",
      prepublishOnly: "npm test",
    };

    writeFileSync(
      pkgBundlerPackageJsonPath,
      `${JSON.stringify(pkgJson, null, 2)}\n`,
      "utf8",
    );
  }

  process.exit(result.status);
}

process.exit(1);
