// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./pin_android_consumer_publication.mjs", import.meta.url));
const original = "<verification-metadata>\n   <components>\n   </components>\n</verification-metadata>\n";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "android-publication-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repository = join(root, "repository");
  const artifacts = join(repository, "me/really/codec-android/1.2.3");
  const metadata = join(root, "verification-metadata.xml");
  mkdirSync(artifacts, { recursive: true });
  writeFileSync(metadata, original);
  return {
    artifacts,
    metadata,
    run: () => spawnSync(process.execPath, [script, repository, metadata], { encoding: "utf8", timeout: 10_000 }),
  };
}

test("pins the exact staged AAR and repository metadata while preserving other pins", (t) => {
  const f = fixture(t);
  for (const extension of ["aar", "module", "pom"]) {
    writeFileSync(join(f.artifacts, `codec-android-1.2.3.${extension}`), extension);
  }
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  const metadata = readFileSync(f.metadata, "utf8");
  assert.match(metadata, /<component group="me\.really" name="codec-android" version="1\.2\.3">/u);
  for (const extension of ["aar", "module", "pom"]) {
    const sha = createHash("sha256").update(extension).digest("hex");
    assert.ok(metadata.includes(`<artifact name="codec-android-1.2.3.${extension}">\n            <sha256 value="${sha}"/>`));
  }
  assert.equal(f.run().status, 1);
  assert.equal(readFileSync(f.metadata, "utf8"), metadata);
});

test("leaves verification metadata intact when a staged artifact is missing", (t) => {
  const f = fixture(t);
  writeFileSync(join(f.artifacts, "codec-android-1.2.3.aar"), "aar");
  const result = f.run();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /could not read the publication or write its verification metadata/u);
  assert.equal(readFileSync(f.metadata, "utf8"), original);
});
