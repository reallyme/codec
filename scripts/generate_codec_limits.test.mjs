// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = fileURLToPath(new URL("../", import.meta.url));
const swiftFile = "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift";
const declaration = /private let maxProtoJsonRequestLength = [\d_]+/u;
const inputs = [
  "scripts/generate_codec_limits.mjs", "scripts/codec_limits.json",
  "crates/cbor/src/deterministic/limits.rs", "crates/cbor/src/limits.rs",
  "crates/codec/src/tlv/validate_options.rs", "crates/proto/src/limits.rs",
  "packages/kotlin/src/main/kotlin/me/really/codec/CodecWireLimits.kt",
  swiftFile, "packages/swift/Sources/ReallyMeCodec/ValidateDeterministicCbor.swift",
  "packages/ts/src/boundary.ts", "packages/ts/src/deterministicCborBoundary.ts",
];

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "codec-limit-generation-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const relative of inputs) {
    const destination = join(root, relative);
    mkdirSync(dirname(destination), { recursive: true });
    cpSync(join(repository, relative), destination);
  }
  return { root, run: (mode) => spawnSync(process.execPath,
    ["scripts/generate_codec_limits.mjs", mode], { cwd: root, encoding: "utf8" }) };
}

test("limit generation checks and repairs the Swift ProtoJSON transport cap", (t) => {
  const { root, run } = fixture(t);
  const file = join(root, swiftFile);
  const original = readFileSync(file, "utf8");
  assert.equal(run("--check").status, 0);
  writeFileSync(file, original.replace(declaration, "private let maxProtoJsonRequestLength = 1"));
  const rejected = run("--check");
  assert.notEqual(rejected.status, 0);
  assert.match(rejected.stderr, /codec limits differ from scripts\/codec_limits.json/u);
  assert.equal(run("--write").status, 0);
  assert.equal(readFileSync(file, "utf8"), original);
  assert.equal(run("--check").status, 0);
});

test("Swift ProtoJSON cap follows changed shared payload and framing limits", (t) => {
  const { root, run } = fixture(t);
  const path = join(root, "scripts/codec_limits.json");
  const limits = JSON.parse(readFileSync(path, "utf8"));
  limits.tlv.maximumInputBytes += 7;
  limits.protobuf.fixedOperationBytes += 32;
  writeFileSync(path, JSON.stringify(limits));
  assert.notEqual(run("--check").status, 0);
  assert.equal(run("--write").status, 0);
  const expected = Math.ceil(limits.tlv.maximumInputBytes / 3) * 4 + limits.protobuf.fixedOperationBytes;
  const text = readFileSync(join(root, swiftFile), "utf8");
  const actual = text.match(declaration)?.[0].split(" = ")[1].replaceAll("_", "");
  assert.equal(actual, String(expected));
  assert.equal(run("--check").status, 0);
});
