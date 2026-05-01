import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const scriptDir = fileURLToPath(new URL(".", import.meta.url));
const root = resolve(scriptDir, "..");
const dictPath = resolve(root, "data/processed/exceptions.bin");
const pkgNodePackageJsonPath = resolve(
  root,
  "crates/wasm/pkg-node/package.json",
);
const pkgBundlerPackageJsonPath = resolve(root, "crates/wasm/pkg/package.json");

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
  if (existsSync(pkgNodePackageJsonPath)) {
    const p = JSON.parse(readFileSync(pkgNodePackageJsonPath, "utf8"));
    p.name = "@tilitronic/polish-stress-wasm-node";
    p.description =
      "Node.js test build of the Polish stress WASM engine (internal)";
    p.author = "Tilitronic";
    p.license = "AGPL-3.0-or-later";
    p.private = true;
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
    ];
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
