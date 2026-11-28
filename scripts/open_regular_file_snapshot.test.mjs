// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { appendFileSync, fstatSync, mkdirSync, mkdtempSync, renameSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { FileSnapshotError, openRegularFileSnapshot } from "./open_regular_file_snapshot.mjs";

const fixture = (t) => {
  const directory = mkdtempSync(join(tmpdir(), "codec-file-snapshot-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const path = join(directory, "input");
  writeFileSync(path, "reviewed");
  return { directory, path };
};
const reason = (code) => (error) => error instanceof FileSnapshotError && error.code === code;

test("exact-limit snapshots retain one descriptor and wipe their bytes on close", (t) => {
  const { path } = fixture(t);
  const snapshot = openRegularFileSnapshot(path, 8);
  const descriptor = snapshot.descriptor;
  assert.equal(snapshot.bytes.toString(), "reviewed");
  snapshot.assertUnchanged();
  snapshot.close();
  assert.ok(snapshot.bytes.every((value) => value === 0));
  assert.throws(() => fstatSync(descriptor), { code: "EBADF" });
  snapshot.close();
});

test("invalid limits, empty files, oversized files and directories fail closed", (t) => {
  const { path, directory } = fixture(t);
  for (const maximum of [0, -1, 1.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => openRegularFileSnapshot(path, maximum), reason("invalid-file-limit"));
  }
  assert.throws(() => openRegularFileSnapshot(path, 7), reason("invalid-file-size-or-type"));
  writeFileSync(path, "");
  assert.throws(() => openRegularFileSnapshot(path, 8), reason("invalid-file-size-or-type"));
  mkdirSync(join(directory, "subdirectory"));
  assert.throws(() => openRegularFileSnapshot(join(directory, "subdirectory"), 8), reason("invalid-file-size-or-type"));
  assert.throws(() => openRegularFileSnapshot(join(directory, "missing"), 8), reason("file-missing"));
});

test("symbolic links cannot redirect a bounded read", (t) => {
  const { path, directory } = fixture(t);
  const link = join(directory, "link");
  symlinkSync(path, link);
  assert.throws(() => openRegularFileSnapshot(link, 8), reason("file-unavailable"));
});

test("pathname replacement cannot redirect the retained descriptor", (t) => {
  const { path } = fixture(t);
  const snapshot = openRegularFileSnapshot(path, 8);
  try {
    renameSync(path, `${path}.old`);
    writeFileSync(path, "replaced");
    assert.equal(snapshot.bytes.toString(), "reviewed");
    assert.throws(() => snapshot.assertUnchanged(), reason("file-changed"));
  } finally { snapshot.close(); }
});

for (const rewrite of [false, true]) {
  test(`${rewrite ? "same-size rewrites" : "growth"} invalidate an open snapshot`, (t) => {
    const { path } = fixture(t);
    const snapshot = openRegularFileSnapshot(path, 8);
    try {
      if (rewrite) writeFileSync(path, "changed!");
      else appendFileSync(path, "!");
      assert.throws(() => snapshot.assertUnchanged(), reason("file-changed"));
      assert.equal(snapshot.bytes.toString(), "reviewed");
    } finally { snapshot.close(); }
  });
}
