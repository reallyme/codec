// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { spawnSync } from "node:child_process";
import { rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_WASM_PACK_VERSION = [0, 15, 0];
const REQUIRED_WASM_BINDGEN_VERSION = [0, 2, 126];
const WASM_PACK_COMMAND = "wasm-pack";
const WASM_BINDGEN_COMMAND = "wasm-bindgen";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packageDirectory = resolve(scriptDirectory, "..");
const repositoryDirectory = resolve(packageDirectory, "..", "..");
const wasmCrateDirectory = resolve(repositoryDirectory, "crates", "wasm");
const outputDirectory = resolve(packageDirectory, "dist", "wasm");

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

if (compareVersion(wasmBindgenVersion, REQUIRED_WASM_BINDGEN_VERSION) < 0) {
  fail(
    `wasm-bindgen ${versionText(REQUIRED_WASM_BINDGEN_VERSION)} or newer is required; found ${versionText(
      wasmBindgenVersion,
    )}.`,
  );
}

const result = spawnSync(
  WASM_PACK_COMMAND,
  [
    "build",
    wasmCrateDirectory,
    "--target",
    "web",
    "--out-dir",
    outputDirectory,
    "--out-name",
    "reallyme_codec_wasm",
  ],
  {
    cwd: packageDirectory,
    stdio: "inherit",
  },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

for (const generatedFile of [
  ".gitignore",
  "package.json",
  "reallyme_codec_wasm_bg.wasm.d.ts",
]) {
  rmSync(resolve(outputDirectory, generatedFile), { force: true });
}
