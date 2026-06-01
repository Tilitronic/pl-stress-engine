import { copyFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const scriptDir = fileURLToPath(new URL(".", import.meta.url));
const root = resolve(scriptDir, "..");
const dictPath = resolve(root, "data/processed/exceptions.bin");
const generatedContractsPath = resolve(
  root,
  "crates/wasm/generated/contracts.d.ts",
);
const pkgNodePackageJsonPath = resolve(
  root,
  "crates/wasm/pkg-node/package.json",
);
const pkgBundlerPackageJsonPath = resolve(root, "crates/wasm/pkg/package.json");
const pkgNodeContractsPath = resolve(
  root,
  "crates/wasm/pkg-node/contracts.d.ts",
);
const pkgBundlerContractsPath = resolve(root, "crates/wasm/pkg/contracts.d.ts");
const pkgNodeTypesPath = resolve(
  root,
  "crates/wasm/pkg-node/pl_stress_wasm.d.ts",
);
const pkgBundlerTypesPath = resolve(
  root,
  "crates/wasm/pkg/pl_stress_wasm.d.ts",
);

function patchWasmTypes(typesPath) {
  if (!existsSync(typesPath)) {
    return;
  }

  let content = readFileSync(typesPath, "utf8");

  if (
    !content.includes('import type { WordLookupResult } from "./contracts";')
  ) {
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
  process.exit(1);
}

const cargoBin = resolve(homedir(), ".cargo", "bin");
const pathSep = process.platform === "win32" ? ";" : ":";
const childPath = `${cargoBin}${pathSep}${process.env.PATH ?? ""}`;

if (!existsSync(dictPath)) {
  console.error("Missing required dictionary file:");
  console.error(`  ${dictPath}`);
  process.exit(1);
}

if (!existsSync(generatedContractsPath)) {
  console.error("Missing generated contract types:");
  console.error(`  ${generatedContractsPath}`);
  console.error("Run `pnpm run generate:contracts` first.");
  process.exit(1);
}

const spawnOpts = {
  stdio: "inherit",
  shell: true,
  cwd: root,
  env: {
    ...process.env,
    PATH: childPath,
  },
};

const resultNode = spawnSync(
  wasmPack,
  [
    "build",
    "crates/wasm",
    "--target",
    "nodejs",
    "--release",
    "--out-dir",
    "pkg-node",
  ],
  spawnOpts,
);

const resultBundler = spawnSync(
  wasmPack,
  [
    "build",
    "crates/wasm",
    "--target",
    "bundler",
    "--release",
    "--out-dir",
    "pkg",
  ],
  spawnOpts,
);

const ok = resultNode.status === 0 && resultBundler.status === 0;

if (ok) {
  copyFileSync(generatedContractsPath, pkgNodeContractsPath);
  copyFileSync(generatedContractsPath, pkgBundlerContractsPath);
  patchWasmTypes(pkgNodeTypesPath);
  patchWasmTypes(pkgBundlerTypesPath);

  if (existsSync(pkgNodePackageJsonPath)) {
    const p = JSON.parse(readFileSync(pkgNodePackageJsonPath, "utf8"));
    p.name = "@tilitronic/polish-stress-wasm-node";
    p.description =
      "Node.js test build of the Polish stress WASM engine (internal)";
    p.author = "Tilitronic";
    p.license = "AGPL-3.0-or-later";
    p.private = true;
    p.files = [
      "pl_stress_wasm.js",
      "pl_stress_wasm_bg.js",
      "pl_stress_wasm_bg.wasm",
      "pl_stress_wasm.d.ts",
      "pl_stress_wasm_bg.wasm.d.ts",
      "contracts.d.ts",
    ];
    p.exports = {
      ".": {
        types: "./pl_stress_wasm.d.ts",
        default: "./pl_stress_wasm.js",
      },
      "./contracts": "./contracts.d.ts",
    };
    p.scripts = {
      test: "node --test ../../../tests/npm/wasm-stress-difficult-words.test.mjs",
    };
    writeFileSync(
      pkgNodePackageJsonPath,
      `${JSON.stringify(p, null, 2)}\n`,
      "utf8",
    );
  }

  if (existsSync(pkgBundlerPackageJsonPath)) {
    const p = JSON.parse(readFileSync(pkgBundlerPackageJsonPath, "utf8"));
    p.name = "@tilitronic/polish-stress-wasm";
    p.description =
      "WebAssembly bindings for the Polish stress engine — syllabification, stress placement, IPA transcription. ESM/bundler build (Vite, webpack, Rollup).";
    p.author = "Tilitronic";
    p.license = "AGPL-3.0-or-later";
    p.module = "pl_stress_wasm.js";
    p.main = "pl_stress_wasm.js";
    p.types = "pl_stress_wasm.d.ts";
    p.sideEffects = false;
    p.files = [
      "pl_stress_wasm.js",
      "pl_stress_wasm_bg.js",
      "pl_stress_wasm_bg.wasm",
      "pl_stress_wasm.d.ts",
      "pl_stress_wasm_bg.wasm.d.ts",
      "contracts.d.ts",
    ];
    p.exports = {
      ".": {
        types: "./pl_stress_wasm.d.ts",
        import: "./pl_stress_wasm.js",
        default: "./pl_stress_wasm.js",
      },
      "./contracts": "./contracts.d.ts",
      "./pl_stress_wasm_bg.wasm": "./pl_stress_wasm_bg.wasm",
    };
    p.repository = {
      type: "git",
      url: "https://github.com/Tilitronic/pl-stress-engine.git",
      directory: "crates/wasm",
    };
    p.keywords = [
      "polish",
      "stress",
      "syllable",
      "ipa",
      "wasm",
      "webassembly",
      "nlp",
      "browser",
      "vite",
    ];
    p.publishConfig = { access: "public" };
    p.scripts = {
      pretest: "node ../../../scripts/build-wasm-node.mjs",
      test: "node --test ../../../tests/npm/wasm-stress-difficult-words.test.mjs",
      prepublishOnly: "npm test",
    };

    writeFileSync(
      pkgBundlerPackageJsonPath,
      `${JSON.stringify(p, null, 2)}\n`,
      "utf8",
    );
  }
}

process.exit(resultNode.status ?? 1);
