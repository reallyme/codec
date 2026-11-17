// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const script = fileURLToPath(new URL("./validate_codec_vectors.mjs", import.meta.url));
const original = JSON.parse(readFileSync(new URL("../vectors/codec-vectors.json", import.meta.url), "utf8"));

const validate = (mutate) => {
  const root = mkdtempSync(join(tmpdir(), "codec-vector-validation-"));
  try {
    const manifest = structuredClone(original);
    mutate(manifest);
    mkdirSync(join(root, "vectors"));
    writeFileSync(join(root, "vectors/codec-vectors.json"), JSON.stringify(manifest));
    return spawnSync(process.execPath, [script], { cwd: root, encoding: "utf8" });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
};

test("shared vector validator accepts the reviewed PEM bundle matrix", () => {
  const result = validate(() => {});
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /17 PEM bundle fixtures/u);
});

test("shared vector validator rejects missing scope or profile coverage", () => {
  for (const name of ["first-stops-before-malformed-tail", "compatible-accepts-inline-armor"]) {
    const result = validate((manifest) => {
      manifest.pemBundleDecode.cases = manifest.pemBundleDecode.cases.filter((fixture) => fixture.name !== name);
    });
    assert.notEqual(result.status, 0);
    assert.ok(result.stderr.includes(`missing PEM bundle fixture ${name}`));
  }
});

test("shared vector validator rejects missing error origin", () => {
  const result = validate((manifest) => {
    const fixture = manifest.pemBundleDecode.cases.find((fixture) => fixture.name === "block-limit-exceeded");
    delete fixture.response.error.origin;
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /typed PEM failure and caller origin/u);
});

test("shared vector validator rejects incomplete golden bytes", () => {
  const result = validate((manifest) => {
    manifest.pemBundleDecode.cases[0].responseHex += "0";
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /responseHex must contain lowercase complete bytes/u);
});
