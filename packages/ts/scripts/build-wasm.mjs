// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_WASM_PACK_VERSION = [0, 15, 0];
const WASM_PACK_COMMAND = "wasm-pack";
const WASM_BINDGEN_COMMAND = "wasm-bindgen";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packageDirectory = resolve(scriptDirectory, "..");
const repositoryDirectory = resolve(packageDirectory, "..", "..");
const wasmCrateDirectory = resolve(repositoryDirectory, "crates", "wasm");
const outputDirectory = resolve(packageDirectory, "dist", "wasm");
const cargoLockPath = resolve(repositoryDirectory, "Cargo.lock");
const cargoLock = readFileSync(cargoLockPath, "utf8");
const lockedBindgen = /^name = "wasm-bindgen"\nversion = "(\d+\.\d+\.\d+)"$/m.exec(cargoLock);
if (lockedBindgen === null) {
  fail("Cargo.lock does not contain one pinned wasm-bindgen version.");
}

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function parseToolVersion(command, output) {
  const match = new RegExp(`^${command} (\\d+)\\.(\\d+)\\.(\\d+)$`).exec(output.trim());
  if (match === null) {
    return null;
  }

  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function compareVersion(left, right) {
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] < right[index]) {
      return -1;
    }
    if (left[index] > right[index]) {
      return 1;
    }
  }

  return 0;
}

function versionText(version) {
  return version.join(".");
}

const versionResult = spawnSync(WASM_PACK_COMMAND, ["--version"], {
  cwd: packageDirectory,
  encoding: "utf8",
});

if (versionResult.status !== 0) {
  fail("wasm-pack is required to build the ReallyMe codec WASM artifact.");
}

const wasmPackVersion = parseToolVersion(WASM_PACK_COMMAND, versionResult.stdout);
if (wasmPackVersion === null) {
  fail("wasm-pack reported an unrecognized version string.");
}

if (compareVersion(wasmPackVersion, REQUIRED_WASM_PACK_VERSION) < 0) {
  fail(
    `wasm-pack ${versionText(REQUIRED_WASM_PACK_VERSION)} or newer is required; found ${versionText(
      wasmPackVersion,
    )}.`,
  );
}

const wasmBindgenVersionResult = spawnSync(WASM_BINDGEN_COMMAND, ["--version"], {
  cwd: packageDirectory,
  encoding: "utf8",
});

if (wasmBindgenVersionResult.status !== 0) {
  fail("wasm-bindgen is required to build the ReallyMe codec WASM artifact.");
}

const wasmBindgenVersion = parseToolVersion(WASM_BINDGEN_COMMAND, wasmBindgenVersionResult.stdout);
if (wasmBindgenVersion === null) {
  fail("wasm-bindgen reported an unrecognized version string.");
}

if (versionText(wasmBindgenVersion) !== lockedBindgen[1]) {
  fail(
    `wasm-bindgen ${lockedBindgen[1]} is required by Cargo.lock; found ${versionText(wasmBindgenVersion)}.`,
  );
}

// wasm-pack resolves metadata before forwarding --locked to Cargo. Check the
// lockfile first so that metadata cannot silently repair a stale dependency.
const lockedMetadata = spawnSync(
  "cargo",
  ["metadata", "--locked", "--format-version", "1"],
  { cwd: repositoryDirectory, stdio: ["inherit", "ignore", "inherit"] },
);
if (lockedMetadata.status !== 0) {
  fail("Cargo.lock is not valid for the WASM workspace.");
}

const result = spawnSync(
  WASM_PACK_COMMAND,
  [
    "build",
    wasmCrateDirectory,
    "--mode",
    "no-install",
    "--target",
    "web",
    "--out-dir",
    outputDirectory,
    "--out-name",
    "reallyme_codec_wasm",
    "--",
    "--locked",
  ],
  {
    cwd: packageDirectory,
    stdio: "inherit",
  },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

if (readFileSync(cargoLockPath, "utf8") !== cargoLock) {
  fail("The WASM build changed Cargo.lock.");
}

// The generated initializer is intentionally install-once. Retain its compiled
// module but replace the instance after a trap, which may skip Rust drop paths.
// Wipe the old linear memory before releasing the trapped instance.
// Match the exact generated footer so a bindgen layout change fails the build.
const gluePath = resolve(outputDirectory, "reallyme_codec_wasm.js");
const glue = readFileSync(gluePath, "utf8");
const footer = "export { initSync, __wbg_init as default };";
if (glue.split(footer).length !== 2 ||
    !glue.includes("let wasmModule, wasmInstance, wasm;")) {
  fail("The generated WASM initializer layout changed.");
}
const recoveryExport = `export function reinstantiate() {
    if (wasmModule === undefined) {
        throw new TypeError('WASM module is not initialized');
    }
    if (!(wasm.memory instanceof WebAssembly.Memory)) {
        throw new TypeError('WASM memory is unavailable');
    }
    new Uint8Array(wasm.memory.buffer).fill(0);
    const imports = __wbg_get_imports();
    const instance = new WebAssembly.Instance(wasmModule, imports);
    __wbg_finalize_init(instance, wasmModule);
}

${footer}`;
writeFileSync(gluePath, glue.replace(footer, recoveryExport));

const declarationsPath = resolve(outputDirectory, "reallyme_codec_wasm.d.ts");
const declarations = readFileSync(declarationsPath, "utf8");
writeFileSync(declarationsPath, `${declarations}\n/** Replace a trapped instance using its compiled module. */\nexport function reinstantiate(): void;\n`);

for (const generatedFile of [
  ".gitignore",
  "package.json",
  "reallyme_codec_wasm_bg.wasm.d.ts",
]) {
  rmSync(resolve(outputDirectory, generatedFile), { force: true });
}
