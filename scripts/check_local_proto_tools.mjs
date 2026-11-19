#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const fail = () => {
  process.stderr.write("codec protocol generation requires its pinned local tools and local plugins\n");
  process.exit(1);
};
const config = readFileSync(new URL("../buf.gen.yaml", import.meta.url), "utf8");
if (/^\s*-?\s*remote\s*:/mu.test(config)) fail();
for (const [command, arguments_, expected] of [
  ["protoc", ["--version"], "libprotoc 36.2"],
  ["protoc-gen-swift", ["--version"], "protoc-gen-swift 1.39.0"],
  [process.execPath, ["packages/ts/node_modules/@bufbuild/protoc-gen-es/bin/protoc-gen-es", "--version"], "protoc-gen-es v2.17.0"],
]) {
  const result = spawnSync(command, arguments_, { cwd: root, encoding: "utf8", timeout: 30_000 });
  if (result.error !== undefined || result.status !== 0 || result.stdout.trim() !== expected) fail();
}
