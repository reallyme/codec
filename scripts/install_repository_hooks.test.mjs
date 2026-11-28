// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, existsSync, fstatSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { openRegularFileSnapshot } from "./open_regular_file_snapshot.mjs";

const script = fileURLToPath(new URL("./install_repository_hooks.mjs", import.meta.url));

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "codec-hook-install-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const env = { ...process.env, GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: join(root, "no-global-config") };
  for (const key of ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"]) delete env[key];
  const git = (...args) => {
    const result = spawnSync("git", args, { cwd: root, env, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
  };
  git("init", "--quiet");
  const run = () => spawnSync(process.execPath, [script], { cwd: root, env, encoding: "utf8" });
  return { root, run, git };
}

test("hook installation is repeatable and retains executable guards", (t) => {
  const { root, run } = fixture(t);
  assert.equal(run().status, 0);
  assert.equal(run().status, 0);
  for (const name of ["pre-commit", "pre-push"]) {
    const path = join(root, ".git/hooks", name);
    const expected = readFileSync(new URL(`../.github/hooks/${name}`, import.meta.url));
    const snapshot = openRegularFileSnapshot(path, expected.length);
    try {
      assert.ok(fstatSync(snapshot.descriptor).mode & 0o100);
      assert.ok(snapshot.bytes.equals(expected));
    } finally { snapshot.close(); }
  }
});

test("unrelated hooks are preserved without partial installation", (t) => {
  const { root, run } = fixture(t);
  const path = join(root, ".git/hooks/pre-push");
  writeFileSync(path, "existing guard\n");
  assert.equal(run().status, 1);
  assert.equal(readFileSync(path, "utf8"), "existing guard\n");
  assert.equal(existsSync(join(root, ".git/hooks/pre-commit")), false);
});

test("custom hook directories are preserved", (t) => {
  const { root, run, git } = fixture(t);
  const directory = join(root, "custom-hooks");
  mkdirSync(directory);
  git("config", "core.hooksPath", directory);
  assert.equal(run().status, 1);
  assert.equal(existsSync(join(directory, "pre-push")), false);
});

test("a symlinked existing guard is rejected without changing its target", (t) => {
  const { root, run } = fixture(t);
  const target = join(root, "other-guard");
  const original = readFileSync(new URL("../.github/hooks/pre-push", import.meta.url));
  writeFileSync(target, original, { mode: 0o640 });
  chmodSync(target, 0o640);
  symlinkSync(target, join(root, ".git/hooks/pre-push"));
  assert.equal(run().status, 1);
  const snapshot = openRegularFileSnapshot(target, original.length);
  try {
    assert.equal(fstatSync(snapshot.descriptor).mode & 0o777, 0o640);
    assert.ok(snapshot.bytes.equals(original));
  } finally { snapshot.close(); }
  assert.equal(existsSync(join(root, ".git/hooks/pre-commit")), false);
});
