#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const generators = ["protoc-gen-reallyme-buffa", "protoc-gen-reallyme-buffa-packaging"];

const fail = (message) => {
  console.error(`codec protocol generation failed: ${message}`);
  process.exit(1);
};

const arguments_ = process.argv.slice(2);
if (arguments_.length > 1 || (arguments_.length === 1 && arguments_[0] !== "--check-tools")) {
  fail("unsupported arguments");
}

// Matching package versions alone is insufficient: old upstream executables
// may coexist on PATH. Require both the fork's name and exact release output.
for (const generator of generators) {
  const result = spawnSync(generator, ["--version"], { cwd: root, encoding: "utf8" });
  if (result.error !== undefined || result.status !== 0 || result.stdout.trim() !== `${generator} 0.1.1`) {
    fail(`install ${generator} from its published 0.1.1 package`);
  }
}
if (arguments_[0] === "--check-tools") {
  process.exit(0);
}

for (const [command, arguments_] of [
  ["buf", ["lint"]],
  ["buf", ["generate"]],
  [process.execPath, ["scripts/generate_codec_proto_support.mjs"]],
  [process.execPath, ["scripts/generate_codec_sdk_contract.mjs"]],
  [process.execPath, ["scripts/generate_codec_sdk_contract.mjs", "--check"]],
  [process.execPath, ["scripts/harden_codec_sdk_proto.mjs"]],
  [process.execPath, ["scripts/harden_codec_sdk_proto.mjs", "--check-idempotent"]],
  ["cargo", ["fmt", "--package", "reallyme-codec-proto"]],
]) {
  const result = spawnSync(command, arguments_, { cwd: root, stdio: "inherit" });
  if (result.error !== undefined || result.status !== 0) {
    fail("generation step failed");
  }
}
