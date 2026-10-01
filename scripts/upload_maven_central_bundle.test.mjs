// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const script = fileURLToPath(new URL("upload_maven_central_bundle.mjs", import.meta.url));
const deploymentId = "28570f16-da32-4c14-bd2e-c1acc0782365";

for (const [state, expectedStatus] of [
  ["PUBLISHED", 0],
  ["FAILED", 1],
  ["UNKNOWN", 1],
]) {
  test(`Central Portal ${state} status is handled deterministically`, (t) => {
    const directory = mkdtempSync(join(tmpdir(), "codec-portal-test-"));
    t.after(() => rmSync(directory, { recursive: true, force: true }));
    const archive = join(directory, "bundle.zip");
    const preload = join(directory, "portal.mjs");
    writeFileSync(archive, "bounded bundle fixture");
    writeFileSync(preload, `
      globalThis.fetch = async (url, options) => {
        if (String(url).endsWith('/upload?publishingType=AUTOMATIC&name=reallyme-codec-0.3.0')) {
          if (options.method !== 'POST' || !String(options.headers.Authorization).startsWith('Bearer ')) throw Error('wrong upload request');
          return { status: 201, text: async () => '${deploymentId}' };
        }
        return { ok: true, json: async () => ({
          deploymentId: '${deploymentId}', deploymentState: '${state}',
          purls: ['pkg:maven/me.really/codec@0.3.0', 'pkg:maven/me.really/codec-android@0.3.0'],
        }) };
      };
    `);
    const result = spawnSync(process.execPath, ["--import", pathToFileURL(preload).href, script, archive, "0.3.0"], {
      encoding: "utf8",
      env: { ...process.env, CENTRAL_PORTAL_USERNAME: "test-user", CENTRAL_PORTAL_PASSWORD: "private-test-password" },
      timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, expectedStatus, result.stderr);
    assert.doesNotMatch(`${result.stdout}${result.stderr}`, /private-test-password/u);
  });
}

test("Central Portal upload requires credentials before making a request", (t) => {
  const directory = mkdtempSync(join(tmpdir(), "codec-portal-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const archive = join(directory, "bundle.zip");
  writeFileSync(archive, "bounded bundle fixture");
  const result = spawnSync(process.execPath, [script, archive, "0.3.0"], {
    encoding: "utf8",
    env: { ...process.env, CENTRAL_PORTAL_USERNAME: "", CENTRAL_PORTAL_PASSWORD: "" },
    timeout: 10_000,
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /missing-portal-credentials/u);
});
