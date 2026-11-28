// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { existsSync, fstatSync, mkdtempSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { FileSnapshotError, openRegularFileSnapshot } from "./open_regular_file_snapshot.mjs";
import { withReviewedPublicationBundle } from "./review_publication_bundle.mjs";

const fixture = (t) => {
  const directory = mkdtempSync(join(tmpdir(), "codec-bundle-review-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const path = join(directory, "bundle.zip");
  writeFileSync(path, "reviewed archive");
  return path;
};

test("upload consumes the private inspected snapshot even if the original is replaced", async (t) => {
  const path = fixture(t);
  let reviewedPath;
  let ownedBytes;
  const result = await withReviewedPublicationBundle(path, (snapshotPath) => {
    reviewedPath = snapshotPath;
    assert.notEqual(snapshotPath, path);
    assert.equal(statSync(dirname(snapshotPath)).mode & 0o777, 0o700);
    const snapshot = openRegularFileSnapshot(snapshotPath, 16);
    try {
      assert.equal(fstatSync(snapshot.descriptor).mode & 0o777, 0o600);
      assert.equal(snapshot.bytes.toString(), "reviewed archive");
    } finally { snapshot.close(); }
    renameSync(path, `${path}.old`);
    writeFileSync(path, "unreviewed archive");
  }, (bytes) => {
    ownedBytes = bytes;
    assert.equal(bytes.toString(), "reviewed archive");
    return "accepted";
  });
  assert.equal(result, "accepted");
  assert.ok(ownedBytes.every((byte) => byte === 0));
  assert.equal(existsSync(dirname(reviewedPath)), false);
});

for (const replacePath of [false, true]) {
  test(`inspection detects ${replacePath ? "replacement" : "rewriting"} of its snapshot`, async (t) => {
    const path = fixture(t);
    let reviewedPath;
    let published = false;
    await assert.rejects(withReviewedPublicationBundle(path, (snapshotPath) => {
      reviewedPath = snapshotPath;
      if (replacePath) renameSync(snapshotPath, `${snapshotPath}.old`);
      writeFileSync(snapshotPath, "changed! archive");
    }, () => { published = true; }), (error) => error instanceof FileSnapshotError && error.code === "file-changed");
    assert.equal(published, false);
    assert.equal(existsSync(dirname(reviewedPath)), false);
  });
}

test("inspection failures prevent publication and remove the private copy", async (t) => {
  const path = fixture(t);
  let reviewedPath;
  let published = false;
  const failure = new FileSnapshotError("inspection-rejected");
  await assert.rejects(withReviewedPublicationBundle(path, (snapshotPath) => {
    reviewedPath = snapshotPath;
    throw failure;
  }, () => { published = true; }), failure);
  assert.equal(published, false);
  assert.equal(existsSync(dirname(reviewedPath)), false);
});

test("publication failures wipe the retained bytes", async (t) => {
  const path = fixture(t);
  let ownedBytes;
  const failure = new FileSnapshotError("publication-failed");
  await assert.rejects(withReviewedPublicationBundle(path, () => {}, (bytes) => {
    ownedBytes = bytes;
    throw failure;
  }), failure);
  assert.ok(ownedBytes.every((byte) => byte === 0));
});
