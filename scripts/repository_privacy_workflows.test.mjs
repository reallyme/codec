// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflows = new Map([
  ["crates-package-preflight.yml", 1],
  ["crates-release.yml", 1],
  ["npm-package-preflight.yml", 1],
  ["npm-package-release.yml", 1],
  ["swift-package-preflight.yml", 1],
  ["swift-package-release.yml", 2],
  ["kotlin-android-package-preflight.yml", 3],
  ["kotlin-android-package-release.yml", 1],
]);

for (const [name, packageBoundaries] of workflows) {
  test(`${name} rejects private source history and checks every package boundary`, () => {
    const source = readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), "utf8");
    const checkout = source.indexOf("      - name: Checkout candidate\n");
    const runner = source.indexOf("      - name: Checkout release-readiness runner\n");
    assert.ok(checkout > 0 && runner > checkout);
    assert.match(source.slice(checkout, runner), /fetch-depth: 0/u);
    const history = source.indexOf("run: node scripts/check_ignored_files.mjs --history");
    const admission = source.indexOf("      - name: Verify release candidate\n");
    assert.ok(history > runner && history < admission);
    if (name.includes("preflight")) {
      const maintenanceTools = source.indexOf("      - name: Install dependency maintenance tools\n");
      assert.ok(maintenanceTools > runner && maintenanceTools < admission);
      const verificationJob = source.slice(maintenanceTools, admission);
      assert.match(verificationJob, /tool: cargo-audit@0\.23\.0\r?\n/u);
      assert.match(verificationJob, /fallback: none/u);
      assert.doesNotMatch(verificationJob, /tool:.*cargo-outdated/u);
      assert.match(verificationJob,
        /run: cargo \+1\.100\.0 install cargo-outdated --version '=0\.19\.0' --locked/u);
    }
    const checks = source.match(/- name: Verify publication privacy/gu) ?? [];
    assert.equal(checks.length, packageBoundaries + (name.includes("preflight") ? 1 : 0));
    for (const step of source.split("      - name: ").slice(1)) {
      if (!step.startsWith("Verify publication privacy\n")) continue;
      assert.doesNotMatch(step, /secrets\./u);
      assert.match(step, /node \.release-readiness\/scripts\/repository-privacy\.mjs --artifact "\$PUBLICATION_PATH"/u);
    }

  });
}

test("the ordinary CI policy job reads complete history before privacy validation", () => {
  const source = readFileSync(new URL("../.github/workflows/code-checks.yml", import.meta.url), "utf8");
  const policy = source.slice(source.indexOf("  repository-policy:"), source.indexOf("  rust:"));
  assert.match(policy, /fetch-depth: 0/u);
  assert.match(policy, /node scripts\/check_ignored_files\.mjs --history/u);
});
