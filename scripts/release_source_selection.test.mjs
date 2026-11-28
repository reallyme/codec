// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const workflows = ["crates-package", "npm-package", "swift-package", "kotlin-android-package"];
const dispatchSha = "1".repeat(40);
const ancestorSha = "2".repeat(40);

const guardScript = (source) => {
  const match = /      - name: Validate source selection\n[\s\S]*?        run: \|\n([\s\S]*?)\n      - name: Checkout candidate/u.exec(source);
  assert.ok(match, "source validation must run before candidate checkout");
  return match[1].split("\n").map((line) => line.slice(10)).join("\n");
};

const runGuard = (t, script, { ref = "refs/heads/main", sha = dispatchSha, onMain = "true" } = {}) => {
  const root = mkdtempSync(join(tmpdir(), "codec-release-source-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const output = join(root, "output");
  const calls = join(root, "calls");
  writeFileSync(output, "");
  writeFileSync(calls, "");
  writeFileSync(join(root, "gh"), '#!/bin/sh\nprintf "called\\n" >> "$TEST_CALLS"\nprintf "%s\\n" "$TEST_ON_MAIN"\n', { mode: 0o700 });
  const result = spawnSync("bash", ["-euo", "pipefail", "-c", script], {
    encoding: "utf8", env: {
      ...process.env, PATH: `${root}:${process.env.PATH}`, SOURCE_SHA: sha,
      GITHUB_SHA: dispatchSha, GITHUB_REF: ref, GITHUB_OUTPUT: output,
      GITHUB_REPOSITORY: "reallyme/codec", TEST_ON_MAIN: onMain, TEST_CALLS: calls,
    },
  });
  assert.equal(result.error, undefined);
  return { status: result.status, output: readFileSync(output, "utf8"), calls: readFileSync(calls, "utf8") };
};

for (const name of workflows) {
  const source = readFileSync(new URL(`../.github/workflows/${name}-preflight.yml`, import.meta.url), "utf8");
  test(`${name} preflight accepts only the exact main dispatch commit`, (t) => {
    const script = guardScript(source);
    const valid = runGuard(t, script);
    assert.equal(valid.status, 0);
    assert.equal(valid.output, `sha=${dispatchSha}\n`);
    for (const options of [
      { ref: "refs/heads/other" }, { sha: ancestorSha }, { sha: "main" },
      { sha: "$(exit 0)" }, { sha: "" }, { sha: "1".repeat(39) },
    ]) {
      const rejected = runGuard(t, script, options);
      assert.notEqual(rejected.status, 0);
      assert.equal(rejected.output, "");
    }
    // Build jobs use the immutable dispatch context directly, after this gate.
    for (const step of source.split("      - name: ").slice(1)) {
      if (!step.startsWith("Checkout\n") && !step.startsWith("Checkout candidate\n")) continue;
      assert.match(step, /ref: \$\{\{ github\.sha \}\}/u);
    }
  });
}

for (const file of ["crates-release", "npm-package-release", "swift-package-release", "kotlin-android-package-release"]) {
  const source = readFileSync(new URL(`../.github/workflows/${file}.yml`, import.meta.url), "utf8");
  test(`${file} accepts reviewed ancestors and rejects code outside main`, (t) => {
    const script = guardScript(source);
    const current = runGuard(t, script);
    assert.equal(current.status, 0);
    assert.equal(current.calls, "");
    const ancestor = runGuard(t, script, { sha: ancestorSha });
    assert.equal(ancestor.status, 0);
    assert.equal(ancestor.output, `sha=${ancestorSha}\n`);
    assert.equal(ancestor.calls, "called\n");
    for (const options of [
      { ref: "refs/heads/other" }, { sha: "main" }, { sha: "$(exit 0)" },
      { sha: ancestorSha, onMain: "false" }, { sha: ancestorSha, onMain: "" },
    ]) {
      const rejected = runGuard(t, script, options);
      assert.notEqual(rejected.status, 0);
      assert.equal(rejected.output, "");
    }
    assert.match(script, /\.status == "ahead" or \.status == "identical"/u);
    assert.match(script, /\.behind_by == 0/u);
    assert.match(script, /gh api --hostname github\.com/u);
  });
}
